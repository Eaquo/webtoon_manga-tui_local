use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dirs::config_dir;
use log::{debug, error};
use serde::{Deserialize, Serialize};

/// Toutes les entrées portent `#[serde(default)]` : un champ absent prend sa
/// valeur par défaut au lieu de faire échouer la lecture du fichier entier.
/// Sans cela, une seule clé manquante suffisait à repartir d'une config vide,
/// et la sauvegarde suivante effaçait la liste des chapitres lus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub last_manga_dir: Option<PathBuf>,
    pub read_chapters: HashSet<String>,
    pub open_command: Option<String>,
    pub settings: Settings,
    pub last_download_url: Option<String>,
    pub last_downloaded_chapters: Vec<u32>,
    /// Clés que cette version ne connaît pas. Sans ce fourre-tout, elles
    /// étaient silencieusement supprimées à chaque sauvegarde.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
    /// Vrai quand le fichier n'a pas pu être lu : on refuse alors de le
    /// réécrire, pour ne pas transformer une erreur de lecture en perte de
    /// données définitive.
    #[serde(skip)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub prefer_external: bool,
    pub auto_mark_read: bool,
    pub default_provider: String,
    pub enable_image_rendering: bool,
    pub reader_options: HashMap<String, String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_manga_dir: None,
            read_chapters: HashSet::new(),
            open_command: None,
            settings: Settings::default(),
            last_download_url: None,
            last_downloaded_chapters: Vec::new(),
            extra: HashMap::new(),
            read_only: false,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let mut reader_options = HashMap::new();
        reader_options.insert("mode".to_string(), "webtoon".to_string());

        Self {
            prefer_external: false,
            auto_mark_read: true,
            default_provider: "manual".to_string(),
            enable_image_rendering: true,
            reader_options,
            extra: HashMap::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_dir = Self::config_dir()?;
        let config_path = config_dir.join("config.json");

        debug!("Loading config from {:?}", config_path);

        if !config_path.exists() {
            debug!("Config file doesn't exist, creating default config");
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let config_str = fs::read_to_string(&config_path).context("Failed to read config file")?;

        match serde_json::from_str(&config_str) {
            Ok(config) => {
                debug!("Config loaded successfully");
                Ok(config)
            }
            Err(e) => {
                // On repart sur les valeurs par défaut pour que l'application
                // démarre, mais on met `read_only` pour ne surtout pas
                // réécrire par-dessus un fichier qu'on n'a pas su lire :
                // il contient peut-être des années de chapitres lus.
                error!("Failed to parse config file: {}", e);
                let backup = config_path.with_extension("json.corrupt");
                let _ = fs::copy(&config_path, &backup);
                error!("Config file preserved at {:?}", backup);
                let mut config = Self::default();
                config.read_only = true;
                Ok(config)
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        if self.read_only {
            debug!("Config marked read-only (unreadable on load), skipping save");
            return Ok(());
        }

        let config_dir = Self::config_dir()?;
        let config_path = config_dir.join("config.json");

        debug!("Saving config to {:?}", config_path);

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        }

        let config_str =
            serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        // Écriture atomique : on écrit à côté puis on renomme. Une coupure en
        // cours d'écriture laisse ainsi l'ancienne config intacte plutôt qu'un
        // fichier tronqué (donc illisible, donc des chapitres lus perdus).
        let tmp_path = config_path.with_extension("json.tmp");
        fs::write(&tmp_path, config_str).context("Failed to write temporary config file")?;
        fs::rename(&tmp_path, &config_path).context("Failed to replace config file")?;

        debug!("Config saved successfully");
        Ok(())
    }

    fn config_dir() -> Result<PathBuf> {
        let config_dir = config_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine config directory"))?
            .join("manga_reader");
        Ok(config_dir)
    }

    pub fn mark_chapter_as_read<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        self.read_chapters.insert(path_str);
        self.save()?;
        Ok(())
    }

    pub fn mark_chapter_as_unread<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        self.read_chapters.remove(&path_str);
        self.save()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cas réel : la config de l'utilisateur contenait `hyprland_integration`,
    /// `wayland_native` et `display_scale`, absents de `Settings`. Ces clés
    /// disparaissaient à chaque sauvegarde.
    #[test]
    fn les_cles_inconnues_survivent_a_un_aller_retour() {
        let json = r#"{
            "last_manga_dir": "/home/x/Scan",
            "read_chapters": ["/home/x/Scan/a/Chapitre_001.cbz"],
            "open_command": null,
            "settings": {
                "prefer_external": false,
                "auto_mark_read": true,
                "default_provider": "manual",
                "enable_image_rendering": true,
                "reader_options": {"mode": "webtoon"},
                "hyprland_integration": false,
                "wayland_native": false,
                "display_scale": 1.0
            }
        }"#;

        let config: Config = serde_json::from_str(json).expect("doit se lire");
        let round_trip = serde_json::to_string(&config).expect("doit se sérialiser");

        assert!(round_trip.contains("hyprland_integration"));
        assert!(round_trip.contains("wayland_native"));
        assert!(round_trip.contains("display_scale"));
    }

    /// Cas le plus grave : un seul champ manquant faisait échouer toute la
    /// lecture, l'application repartait d'une config vide, et la sauvegarde
    /// suivante effaçait les chapitres lus.
    #[test]
    fn un_champ_manquant_nefface_pas_les_chapitres_lus() {
        // `enable_image_rendering` est volontairement absent.
        let json = r#"{
            "read_chapters": ["/a/Chapitre_001.cbz", "/a/Chapitre_002.cbz"],
            "settings": {
                "prefer_external": false,
                "auto_mark_read": true,
                "default_provider": "manual",
                "reader_options": {"mode": "webtoon"}
            }
        }"#;

        let config: Config = serde_json::from_str(json).expect("doit se lire malgré le champ absent");
        assert_eq!(config.read_chapters.len(), 2);
        assert!(config.settings.enable_image_rendering); // valeur par défaut
    }

    #[test]
    fn une_config_illisible_nest_jamais_reecrite() {
        let mut config = Config::default();
        config.read_only = true;
        // save() doit réussir sans rien écrire : on ne touche pas à un fichier
        // qu'on n'a pas su lire.
        assert!(config.save().is_ok());
    }

    #[test]
    fn read_only_nest_pas_serialise() {
        let mut config = Config::default();
        config.read_only = true;
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("read_only"));
    }
}

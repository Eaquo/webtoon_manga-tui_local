use anyhow::Result;
use log::debug;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use std::collections::HashSet;

#[allow(dead_code)]
pub struct Manga {
    pub id: i64,
    pub name: String,
    pub ta: Option<String>,         // chemin vers image
    pub synopsis: Option<String>,   // texte synopsis
    pub source_url: Option<String>, // url source
}

pub fn open_db() -> Result<Connection> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let db_dir = Path::new(&home).join(".config/manga_reader");
    let db_path = db_dir.join("library.db");

    if let Err(e) = fs::create_dir_all(&db_dir) {
        return Err(anyhow::anyhow!(
            "Failed to create database directory: {}",
            e
        ));
    }

    debug!("Opening database at: {:?}", db_path);

    let conn = Connection::open(&db_path)?;
    debug!("Database connection established");

    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Check and create mangas table
    let mangas_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'mangas'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !mangas_exists {
        debug!("Table 'mangas' does not exist, creating it");
        conn.execute(
            "CREATE TABLE mangas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                cover TEXT,
                thumbnail TEXT,
                synopsis TEXT,
                source_url TEXT
            )",
            [],
        )?;
        debug!("Table 'mangas' created successfully");
    } else {
        // Check if the synopsis column exists
        let synopsis_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('mangas') WHERE name = 'synopsis'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .map(|count| count > 0)
            .unwrap_or(false);

        if !synopsis_exists {
            debug!("Column 'synopsis' not found, performing migration");
            conn.execute(
                "CREATE TABLE mangas_temp (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    cover TEXT,
                    thumbnail TEXT,
                    synopsis TEXT,
                    source_url TEXT
                )",
                [],
            )?;
            conn.execute(
                "INSERT INTO mangas_temp (id, name, cover, thumbnail, source_url)
                 SELECT id, name, cover, thumbnail, source_url FROM mangas",
                [],
            )?;
            conn.execute("DROP TABLE mangas", [])?;
            conn.execute("ALTER TABLE mangas_temp RENAME TO mangas", [])?;
            debug!("Migration completed: added 'synopsis' column");
        }
    }

    // Création de la table `chapters` si elle n'existe pas.
    //
    // Sans cela, une base vierge partait directement sur la migration
    // ci-dessous, dont le premier geste est `ALTER TABLE chapters RENAME` :
    // l'ordre échouait, open_db renvoyait une erreur, et une installation
    // neuve ne pouvait donc jamais constituer sa bibliothèque.
    let chapters_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'chapters'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !chapters_exists {
        debug!("Table 'chapters' absente, création avec le schéma courant");
        conn.execute(
            "CREATE TABLE chapters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                manga_id INTEGER NOT NULL,
                num INTEGER NOT NULL,
                file TEXT NOT NULL,
                read INTEGER DEFAULT 0,
                last_page_read INTEGER,
                full_pages_read INTEGER,
                size INTEGER NOT NULL DEFAULT 0,
                modified INTEGER NOT NULL DEFAULT 0,
                UNIQUE(manga_id, num),
                FOREIGN KEY (manga_id) REFERENCES mangas(id)
            )",
            [],
        )?;
    }

    // Create or update chapters table
    let has_unique: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_index_list('chapters') WHERE name LIKE '%manga_id_num%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0) > 0;

    if !has_unique {
        debug!("Adding UNIQUE(manga_id, num) constraint to 'chapters' table");

        conn.execute("ALTER TABLE chapters RENAME TO chapters_old", [])?;

        conn.execute(
            "CREATE TABLE chapters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                manga_id INTEGER NOT NULL,
                num INTEGER NOT NULL,
                file TEXT NOT NULL,
                read INTEGER DEFAULT 0,
                last_page_read INTEGER,
                full_pages_read INTEGER,
                size INTEGER NOT NULL DEFAULT 0,
                modified INTEGER NOT NULL DEFAULT 0,
                UNIQUE(manga_id, num),
                FOREIGN KEY (manga_id) REFERENCES mangas(id)
            )",
            [],
        )?;

        conn.execute(
            "INSERT INTO chapters (id, manga_id, num, file, read, last_page_read, full_pages_read, size, modified)
             SELECT id, manga_id, num, file, read, last_page_read, full_pages_read, size, modified FROM chapters_old",
            [],
        )?;

        conn.execute("DROP TABLE chapters_old", [])?;
    }

    // Check if size and modified columns exist
    let size_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('chapters') WHERE name = 'size'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !size_exists {
        debug!("Column 'size' not found, adding it");
        conn.execute(
            "ALTER TABLE chapters ADD COLUMN size INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        conn.execute(
            "ALTER TABLE chapters ADD COLUMN modified INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        debug!("Columns 'size' and 'modified' added");
    }

    // Create metadata table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value INTEGER
        )",
        [],
    )?;
    debug!("Table 'metadata' ensured");

    // Create index
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chapters_manga_id ON chapters(manga_id)",
        [],
    )?;
    debug!("Index 'idx_chapters_manga_id' ensured");

    // Dans manga_indexer::open_db
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_mangas_name ON mangas(name)",
        [],
    )?;
    debug!("Index 'idx_mangas_name' ensured");

    Ok(conn)
}


pub fn scan_and_index(conn: &Connection, root: &Path) -> Result<()> {
    debug!("Scan complet des fichiers");

    let (tx, rx) = mpsc::channel();
    let root_path = root.to_path_buf();

    thread::spawn(move || {
        for entry in WalkDir::new(&root_path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "cbz" || ext == "cbr" {
                        let _ = tx.send(entry.path().to_path_buf());
                    }
                }
            }
        }
    });

    let mut manga_cache: HashMap<String, i64> = HashMap::new();
    let mut stmt = conn.prepare("SELECT id, name FROM mangas")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
        let (id, name) = row?;
        manga_cache.insert(name, id);
    }

    let mut found_files = HashMap::new();

    for path in rx {
        let manga_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let manga_name = manga_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let manga_id = if let Some(&id) = manga_cache.get(&manga_name) {
            id
        } else {
            conn.execute("INSERT INTO mangas (name) VALUES (?1)", [manga_name.clone()])?;
            let id = conn.last_insert_rowid();
            manga_cache.insert(manga_name.clone(), id);
            id
        };

        // Charger cover et synopsis
        let cover_path = ["cover.jpg", "cover.png", "cover.webp"]
            .iter()
            .map(|f| manga_dir.join(f))
            .find(|p| p.exists());
        let synopsis_path = manga_dir.join("synopsis.txt");

        let cover = cover_path.map(|p| p.to_string_lossy().to_string());
        let (synopsis, source_url) = if synopsis_path.exists() {
            match fs::read_to_string(&synopsis_path) {
                Ok(text) => {
                    let parts: Vec<&str> = text.split("\nSource: ").collect();
                    let synopsis_text = parts[0].trim().to_string();
                    let source = parts
                        .get(1)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty() && s.starts_with("http"));
                    debug!("Synopsis: {}, Source URL: {:?}", synopsis_text, source);
                    (Some(synopsis_text), source)
                }
                Err(e) => {
                    debug!("Failed to read synopsis.txt: {}", e);
                    (None, None)
                }
            }
        } else {
            debug!("No synopsis.txt found in {:?}", manga_dir);
            (None, None)
        };

        if cover.is_some() || synopsis.is_some() || source_url.is_some() {
            conn.execute(
                "UPDATE mangas SET thumbnail = ?1, synopsis = ?2, source_url = ?3 WHERE id = ?4",
                rusqlite::params![
                    cover.as_deref(),
                    synopsis.as_deref(),
                    source_url.as_deref(),
                    manga_id
                ],
            )?;
        }

        if let Some(num) = crate::manga::extract_chapter_number(
            path.file_name().unwrap_or_default().to_str().unwrap_or(""),
        ) {
            let num = num as i64;
            let metadata = fs::metadata(&path)?;
            let size = metadata.len() as i64;
            let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;

            found_files.insert((manga_id, num), path.clone());

            conn.execute(
                "INSERT INTO chapters (
                    manga_id, num, file, size, modified,
                    read, last_page_read, full_pages_read
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    COALESCE((SELECT read FROM chapters WHERE manga_id = ?1 AND num = ?2), 0),
                    (SELECT last_page_read FROM chapters WHERE manga_id = ?1 AND num = ?2),
                    (SELECT full_pages_read FROM chapters WHERE manga_id = ?1 AND num = ?2)
                )
                ON CONFLICT(manga_id, num) DO UPDATE SET
                    file = excluded.file,
                    size = excluded.size,
                    modified = excluded.modified",
                rusqlite::params![
                    manga_id,
                    num,
                    path.to_string_lossy().to_string(),
                    size,
                    modified
                ],
            )?;
        }
    }

    // Suppression des chapitres dont le fichier a disparu du disque.
    //
    // `found_files` était rempli au fil du parcours mais n'était jamais
    // exploité : les chapitres supprimés restaient donc en base indéfiniment.
    // On ne se fie qu'à ce qui vient d'être vu, la table étant entièrement
    // reconstruite depuis `root` à chaque scan complet.
    {
        let mut stmt = conn.prepare("SELECT id, manga_id, num FROM chapters")?;
        let rows: Vec<(i64, i64, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        // Garde-fou : ne rien supprimer si le parcours n'a rien trouvé alors
        // que la base est peuplée. Ce cas ne traduit pas une bibliothèque vidée
        // mais un disque non monté ou une racine erronée — et l'effacement
        // ferait perdre l'état « lu » de toute la bibliothèque.
        if found_files.is_empty() && !rows.is_empty() {
            debug!(
                "Aucun fichier trouvé alors que la base en contient {} : \
                 nettoyage annulé (disque absent ?)",
                rows.len()
            );
            return finish_scan(conn);
        }

        let mut removed = 0;
        for (id, manga_id, num) in rows {
            if !found_files.contains_key(&(manga_id, num)) {
                conn.execute("DELETE FROM chapters WHERE id = ?1", [id])?;
                removed += 1;
            }
        }
        if removed > 0 {
            debug!("{} chapitre(s) retiré(s) : fichier absent du disque", removed);
        }
    }

    // Suppression des mangas dont le dossier n’existe plus
    let root_abs = fs::canonicalize(root)?;
    let existing_dirs: HashSet<String> = fs::read_dir(&root_abs)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let mut stmt = conn.prepare("SELECT id, name FROM mangas")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
        let (manga_id, manga_name) = row?;
        if !existing_dirs.contains(&manga_name) {
            debug!("Suppression de '{}' (dossier manquant)", manga_name);
            conn.execute("DELETE FROM chapters WHERE manga_id = ?1", [manga_id])?;
            conn.execute("DELETE FROM mangas WHERE id = ?1", [manga_id])?;
        }
    }

    finish_scan(conn)?;
    remember_root(conn, root)
}

/// Mémorise la racine qui vient d'être indexée.
///
/// Sans cette trace, un changement de dossier passait inaperçu : l'heuristique
/// ne compare que des dates et un nombre de fichiers, deux bibliothèques
/// distinctes pouvant très bien coïncider sur les deux.
pub fn remember_root(conn: &Connection, root: &Path) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS scan_root (id INTEGER PRIMARY KEY CHECK (id = 0), path TEXT)",
        [],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO scan_root (id, path) VALUES (0, ?1)",
        [root.to_string_lossy().to_string()],
    )?;
    Ok(())
}

/// Racine indexée lors du dernier scan, si elle est connue.
pub fn last_scanned_root(conn: &Connection) -> Option<String> {
    conn.query_row("SELECT path FROM scan_root WHERE id = 0", [], |r| r.get(0))
        .ok()
}

/// Enregistre l'heure du scan. Appelé aussi lors d'un abandon volontaire du
/// nettoyage, pour ne pas relancer un scan complet à chaque démarrage.
fn finish_scan(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('last_scan_time', ?1)",
        [SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64],
    )?;

    debug!("Scan terminé.");
    Ok(())
}

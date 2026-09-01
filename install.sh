#!/usr/bin/env bash

# Script d'installation pour manga-reader et webtoon-dl

set -e

# Couleurs pour les messages
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Début de l'installation ===${NC}"

# Vérifier et installer Rust/Cargo
echo -e "${YELLOW}Vérification de Rust et Cargo...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${GREEN}Installation de Rust et Cargo...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo -e "${GREEN}Cargo est déjà installé. $(cargo --version)${NC}"
fi

# Détecter la distribution
DISTRO=""
if [ -f /etc/arch-release ]; then
    DISTRO="arch"
elif [ -f /etc/debian_version ]; then
    DISTRO="debian"
elif [ -f /etc/fedora-release ]; then
    DISTRO="fedora"
elif [ -f /etc/centos-release ] || [ -f /etc/redhat-release ]; then
    DISTRO="rhel"
elif [ -f /etc/os-release ] && grep -q "ID=opensuse" /etc/os-release; then
    DISTRO="opensuse"
elif [ -f /etc/alpine-release ]; then
    DISTRO="alpine"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    DISTRO="macos"
else
    DISTRO="unknown"
fi
echo -e "${YELLOW}Distribution détectée : $DISTRO${NC}"

# Vérifier et installer Python 3
echo -e "${YELLOW}Vérification de Python 3...${NC}"
if ! command -v python3 &> /dev/null; then
    echo -e "${GREEN}Installation de Python 3...${NC}"
    case $DISTRO in
        arch)
            sudo pacman -S --noconfirm python python-pip
            ;;
        debian)
            sudo apt-get update
            sudo apt-get install -y python3 python3-pip
            ;;
        fedora)
            sudo dnf install -y python3 python3-pip
            ;;
        rhel)
            sudo dnf install -y python3 python3-pip
            ;;
        opensuse)
            sudo zypper install -y python3 python3-pip
            ;;
        alpine)
            sudo apk add python3 py3-pip
            ;;
        macos)
            if ! command -v brew &> /dev/null; then
                echo -e "${RED}Homebrew requis sur macOS. Installez-le avec :${NC}"
                echo -e "${YELLOW}/bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"${NC}"
                exit 1
            fi
            brew install python3
            ;;
        *)
            echo -e "${RED}Distribution non supportée pour l'installation automatique de Python.${NC}"
            echo -e "${YELLOW}Veuillez installer Python 3 manuellement, puis relancez le script.${NC}"
            exit 1
            ;;
    esac
else
    echo -e "${GREEN}Python 3 est déjà installé. $(python3 --version)${NC}"
fi

# Installer les dépendances système pour manga-reader
echo -e "${YELLOW}Installation des dépendances système...${NC}"
case $DISTRO in
    arch)
        sudo pacman -S --noconfirm libx11
        ;;
    debian)
        sudo apt-get install -y libx11-dev
        ;;
    fedora)
        sudo dnf install -y libX11-devel
        ;;
    rhel)
        sudo dnf install -y libX11-devel
        ;;
    opensuse)
        sudo zypper install -y libX11-devel
        ;;
    alpine)
        sudo apk add libx11-dev
        ;;
    macos)
        # XQuartz pour X11 sur macOS (optionnel, peut être requis pour ratatui-image)
        if command -v brew &> /dev/null; then
            brew install --cask xquartz
        fi
        ;;
    *)
        echo -e "${YELLOW}Dépendances système non installées automatiquement. Assurez-vous que libX11 est installé si nécessaire.${NC}"
        ;;
esac

# Installer les dépendances Python
echo -e "${YELLOW}Installation des dépendances Python...${NC}"

case $DISTRO in
    arch)
        echo -e "${YELLOW}Installation des paquets Python depuis les dépôts officiels...${NC}"

        # Disponibles dans extra/. --needed évite de tout réinstaller à chaque
        # exécution du script.
        PACMAN_PKGS=(
            python-requests         # webtoon-dl : requêtes HTTP
            python-beautifulsoup4   # webtoon-dl : analyse du HTML
            python-lxml             # webtoon-dl : parseur rapide pour BeautifulSoup
            python-pillow           # les deux : traitement des images
            python-pygame           # manga-live : rendu du lecteur
            python-rarfile          # manga-live : lecture des archives .cbr
            tk                      # manga-live : sélecteur de fichiers (tkinter)
            poppler                 # manga-live : fournit pdftoppm, requis par pdf2image
            unrar                   # manga-live : décompression effective des .cbr
        )
        sudo pacman -S --needed --noconfirm "${PACMAN_PKGS[@]}" || {
            echo -e "${RED}Échec de l'installation avec pacman.${NC}"
            exit 1
        }

        # Ces deux-là ne sont pas dans les dépôts officiels : ils vivent dans l'AUR.
        AUR_PKGS=(
            python-pdf2image        # manga-live : conversion des PDF en images
            python-cloudscraper     # webtoon-dl : contournement de la protection Cloudflare
        )

        AUR_HELPER=""
        for helper in yay paru pikaur trizen; do
            if command -v "$helper" &> /dev/null; then
                AUR_HELPER="$helper"
                break
            fi
        done

        if [ -n "$AUR_HELPER" ]; then
            echo -e "${YELLOW}Installation des paquets AUR avec $AUR_HELPER...${NC}"
            "$AUR_HELPER" -S --needed --noconfirm "${AUR_PKGS[@]}" || {
                echo -e "${RED}Échec de l'installation depuis l'AUR.${NC}"
                exit 1
            }
        else
            # Sans assistant AUR, on compile à la main. pip --user ne fonctionne
            # pas ici : Arch marque son Python comme « externally managed ».
            echo -e "${YELLOW}Aucun assistant AUR détecté. Compilation manuelle...${NC}"
            sudo pacman -S --needed --noconfirm base-devel git || exit 1
            BUILD_DIR=$(mktemp -d)
            for pkg in "${AUR_PKGS[@]}"; do
                echo -e "${YELLOW}Construction de $pkg...${NC}"
                git clone --depth 1 "https://aur.archlinux.org/$pkg.git" "$BUILD_DIR/$pkg" || exit 1
                (cd "$BUILD_DIR/$pkg" && makepkg -si --noconfirm) || {
                    echo -e "${RED}Échec de la construction de $pkg.${NC}"
                    echo -e "${YELLOW}Installez un assistant AUR (yay ou paru) puis relancez.${NC}"
                    exit 1
                }
            done
            rm -rf "$BUILD_DIR"
        fi
        ;;
    *)
        PIP_PKGS="requests beautifulsoup4 pillow cloudscraper lxml pygame rarfile pdf2image"
        python3 -m pip install --user $PIP_PKGS || {
            echo -e "${RED}Erreur lors de l'installation des dépendances Python${NC}"
            echo -e "${YELLOW}Essayez manuellement : python3 -m pip install --user $PIP_PKGS${NC}"
            echo -e "${YELLOW}Note : pdf2image requiert aussi poppler-utils, et rarfile requiert unrar.${NC}"
            exit 1
        }
        ;;
esac

# Créer le dossier bin
mkdir -p ~/.local/bin

# Ajouter ~/.local/bin au PATH si nécessaire
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo -e "${YELLOW}Ajout de ~/.local/bin au PATH...${NC}"
    if [ -n "$ZSH_VERSION" ]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
    else
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
    fi
    export PATH="$HOME/.local/bin:$PATH"
fi

# Configurer webtoon-dl comme une commande exécutable
echo -e "${YELLOW}Configuration de webtoon-dl...${NC}"
if [ -f "webtoon-dl.py" ]; then
    chmod +x webtoon-dl.py
    cp webtoon-dl.py ~/.local/bin/webtoon-dl
    echo -e "${GREEN}webtoon-dl installé comme commande exécutable.${NC}"
else
    echo -e "${RED}Erreur : webtoon-dl.py non trouvé.${NC}"
    exit 1
fi

# Configurer manga-live comme une commande exécutable
echo -e "${YELLOW}Configuration de manga-live...${NC}"
if [ -f "manga-live.py" ]; then
    chmod +x manga-live.py
    cp manga-live.py ~/.local/bin/manga-live
    echo -e "${GREEN}manga-live installé comme commande exécutable.${NC}"
else
    echo -e "${RED}Erreur : manga-live.py non trouvé.${NC}"
    exit 1
fi

# Compiler le programme Rust en mode release
echo -e "${YELLOW}Compilation du programme Rust...${NC}"
cargo build --release
if [ $? -eq 0 ]; then
    echo -e "${GREEN}Compilation réussie ! L'exécutable est dans target/release/manga-reader${NC}"
else
    echo -e "${RED}Erreur lors de la compilation.${NC}"
    exit 1
fi

# Copier l'exécutable manga-reader vers ~/.local/bin/
echo -e "${YELLOW}Installation de manga-reader comme commande globale...${NC}"
if [ -f "target/release/manga-reader" ]; then
    cp target/release/manga-reader ~/.local/bin/
    chmod +x ~/.local/bin/manga-reader
    echo -e "${GREEN}manga-reader installé comme commande globale dans ~/.local/bin/${NC}"
else
    echo -e "${RED}Erreur : l'exécutable manga-reader n'a pas été trouvé dans target/release/${NC}"
    exit 1
fi

echo -e "${YELLOW}=== Installation terminée ===${NC}"
echo -e "${GREEN}Vous pouvez maintenant exécuter :${NC}"
echo -e "${GREEN}  - webtoon-dl${NC}"
echo -e "${GREEN}  - manga-live${NC}"
echo -e "${GREEN}  - manga-reader${NC}"
echo -e "${YELLOW}Note : Redémarrez votre terminal ou exécutez 'source ~/.bashrc' pour utiliser les commandes${NC}"
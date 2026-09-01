#!/usr/bin/env python3
import os
import json
import re
import time
import shutil
import urllib.parse
from zipfile import ZipFile
from typing import List, Tuple, Set
from PIL import Image
import io
import argparse
import requests
import cloudscraper
from bs4 import BeautifulSoup
import sys

# Configuration initiale
default_output_dir = os.path.expanduser("~/Documents/Scan")
config_path = os.path.expanduser("~/.config/manga_reader/config.json")
try:
    with open(config_path, "r") as f:
        config = json.load(f)
        if config.get("last_manga_dir"):
            default_output_dir = config["last_manga_dir"]
except (FileNotFoundError, json.JSONDecodeError, KeyError):
    print("⚠️ Could not read manga_reader config, using default output directory")

# Arguments de la ligne de commande
parser = argparse.ArgumentParser(description="Webtoon Downloader for mangas-origines.fr and anime-sama.fr")
parser.add_argument(
    "url",
    type=str,
    help="URL of the scan page (e.g., https://mangas-origines.fr/oeuvre/.../chapitre-1 or https://anime-sama.fr/catalogue/.../scan/vf/)"
)
parser.add_argument(
    "chapters",
    type=str,
    nargs="?",
    default="1",
    help="Chapters to download (e.g., 1-3,5)"
)
parser.add_argument(
    "--tui",
    action="store_true",
    help="Emit machine-readable progress events for manga-reader (hides verbose logs)"
)
parser.add_argument(
    "-o",
    "--output-dir",
    type=str,
    default=default_output_dir,
    help=f"Directory where manga folders and cache will be created (default: {default_output_dir})"
)
args = parser.parse_args()

# ---------------------------------------------------------------------------
# Sortie : une ligne humaine lisible + (si --tui) un evenement structure que
# manga-reader parse pour sa barre de progression. Un evenement = une ligne
#   @EVT|<type>|cle=valeur|cle=valeur
# Les lignes @EVT ne sont jamais affichees dans le panneau de logs du TUI.
# ---------------------------------------------------------------------------
TUI = args.tui

def say(message: str) -> None:
    """Affiche une ligne destinee a un humain (log lisible)."""
    sys.stdout.write(message + "\n")
    sys.stdout.flush()

def evt(kind: str, **fields) -> None:
    """Emet un evenement structure pour le TUI (ignore en mode CLI)."""
    if not TUI:
        return
    parts = ["@EVT", kind]
    for key, value in fields.items():
        parts.append(f"{key}={str(value).replace('|', '/')}")
    sys.stdout.write("|".join(parts) + "\n")
    sys.stdout.flush()

# Création des répertoires
output_dir = os.path.expanduser(args.output_dir)
os.makedirs(output_dir, exist_ok=True)

# Validation de l'URL et détection du site
start_url = args.url.strip()
site = None
base_url = None
oeuvre_url = None

# Détection pour mangas-origines.fr
if "mangas-origines.fr" in start_url:
    site = "mangas-origines"
    base_match = re.search(r'(.*?/chapitre-)(\d+)', start_url)
    if not base_match:
        print("❌ Invalid URL provided for mangas-origines.fr.")
        exit(1)
    base_url = base_match.group(1)
    oeuvre_url_match = re.search(r'(.*?/oeuvre/[^/]+)', start_url)
    if not oeuvre_url_match:
        print("❌ Could not determine the manga page URL for mangas-origines.fr")
        exit(1)
    oeuvre_url = oeuvre_url_match.group(1)

# Détection pour anime-sama.fr
elif "anime-sama.fr" in start_url:
    site = "anime-sama"
    base_match = re.search(r'(.*?/scan/vf/?)', start_url)
    if not base_match:
        print("❌ Invalid URL provided for anime-sama.fr. Expected format: /scan/vf/")
        exit(1)
    base_url = base_match.group(1)  # Base URL pour construire les URLs des chapitres
    oeuvre_url_match = re.search(r'(.*?/catalogue/[^/]+)', start_url)
    if not oeuvre_url_match:
        print("❌ Could not determine the manga page URL for anime-sama.fr")
        exit(1)
    oeuvre_url = oeuvre_url_match.group(1)
else:
    print("❌ Unsupported site. Supported sites: mangas-origines.fr, anime-sama.fr")
    exit(1)

# Analyse des chapitres demandés
chap_input = args.chapters.strip()
try:
    chapters: Set[int] = set()
    for part in chap_input.split(','):
        if '-' in part:
            start, end = map(int, part.split('-'))
            chapters.update(range(start, end + 1))
        else:
            chapters.add(int(part))
    chapters = sorted(chapters)
except ValueError:
    print("❌ Invalid chapter format. Use numbers, commas, or ranges (e.g., 1-3,5)")
    exit(1)

# Afficher le résumé initial
say(f"🎯 {len(chapters)} chapitre(s) à télécharger : {min(chapters)} → {max(chapters)}")
evt("plan", total=len(chapters), first=min(chapters), last=max(chapters))

# Initialisation du scraper
scraper = cloudscraper.create_scraper()

# Extraire le nom du manga
try:
    soup_preview = BeautifulSoup(scraper.get(oeuvre_url).text, 'html.parser')
    if site == "mangas-origines":
        breadcrumb = soup_preview.select_one('.breadcrumb a[href*="/oeuvre/"]')
        manga_name = breadcrumb.text.strip().replace(" ", "_") if breadcrumb else "Manga"
    else:  # anime-sama.fr
        # Plusieurs sélecteurs possibles pour le titre
        title_selectors = [
            'h1.text-2xl',
            'h1',
            '.text-2xl',
            'title'
        ]
        manga_name = None
        for selector in title_selectors:
            title_elem = soup_preview.select_one(selector)
            if title_elem:
                manga_name = title_elem.get_text().strip()
                # Nettoyer le titre (enlever les suffixes comme "- Anime-Sama")
                manga_name = re.sub(r'\s*-\s*Anime[- ]Sama.*$', '', manga_name, flags=re.IGNORECASE)
                break
        
        if not manga_name:
            # Fallback: extraire depuis l'URL
            url_match = re.search(r'/catalogue/([^/]+)', start_url)
            if url_match:
                manga_name = url_match.group(1).replace('-', ' ')
        
        manga_name = manga_name.replace(" ", "_") if manga_name else "Manga"
        
    say(f"📖 Manga : {manga_name.replace('_', ' ')}")
    evt("manga", name=manga_name.replace('_', ' '), total=len(chapters))
except Exception as e:
    print(f"⚠️ Error fetching manga name: {e}")
    manga_name = "Manga"

# Création des dossiers
manga_dir = os.path.join(output_dir, manga_name)
cache_dir = os.path.join(output_dir, "cache")
say(f"📁 Dossier : {manga_dir}")
evt("folder", path=manga_dir)
os.makedirs(manga_dir, exist_ok=True)
os.makedirs(cache_dir, exist_ok=True)

# Téléchargement de la couverture et du synopsis
cover_path = os.path.join(manga_dir, "cover.jpg")
synopsis_path = os.path.join(manga_dir, "synopsis.txt")

if not os.path.exists(cover_path) or not os.path.exists(synopsis_path):
    print(f"📘 Fetching manga info: {oeuvre_url}")
    try:
        r_oeuvre = scraper.get(oeuvre_url)
        if r_oeuvre.status_code != 200:
            print(f"❌ Manga page not found: {oeuvre_url}")
        else:
            soup_oeuvre = BeautifulSoup(r_oeuvre.text, 'html.parser')

            # Télécharger la couverture
            cover_url = None
            if site == "mangas-origines":
                og_image = soup_oeuvre.select_one('meta[property="og:image"]')
                if og_image and og_image.get('content'):
                    cover_url = og_image['content']
                    print(f"ℹ️ Found og:image: {cover_url}")

                if not cover_url:
                    cover_img = (
                        soup_oeuvre.select_one('div.summary_image img[srcset]') or
                        soup_oeuvre.select_one('picture.img-responsive img[srcset]') or
                        soup_oeuvre.select_one('img[alt*="Cover"][data-fullsrc]') or
                        soup_oeuvre.select_one('img[alt*="thumbnail"][data-fullsrc]') or
                        soup_oeuvre.select_one('div.summary_image img') or
                        soup_oeuvre.select_one('picture.img-responsive img') or
                        soup_oeuvre.select_one('img[alt*="Cover"], img[alt*="thumbnail"]') or
                        soup_oeuvre.select_one('.thumbnail img')
                    )
                    if cover_img:
                        if cover_img.get('srcset'):
                            srcset = cover_img['srcset'].split(',')
                            cover_url = max(srcset, key=lambda x: int(x.split()[-2]) if x.split()[-2].isdigit() else 0).split()[0]
                        elif cover_img.get('data-fullsrc'):
                            cover_url = cover_img['data-fullsrc']
                        elif cover_img.get('data-src'):
                            cover_url = cover_img['data-src']
                        else:
                            cover_url = cover_img.get('src')

                        if cover_url and not cover_url.startswith("http"):
                            cover_url = urllib.parse.urljoin(oeuvre_url, cover_url)

                        if "/thumbnail/" in cover_url.lower():
                            cover_url = cover_url.replace("/thumbnail/", "/full/")
                        elif "?size=" in cover_url.lower():
                            cover_url = re.sub(r"size=\w+", "size=large", cover_url)
                        elif "/small/" in cover_url.lower():
                            cover_url = cover_url.replace("/small/", "/large/")
            else:  # anime-sama.fr
                cover_img = soup_oeuvre.select_one('#coverOeuvre')
                if cover_img and cover_img.get('src'):
                    cover_url = cover_img['src']
                    print(f"ℹ️ Found cover: {cover_url}")

            if cover_url:
                try:
                    print(f"ℹ️ Attempting to download cover from: {cover_url}")
                    cover_data = scraper.get(cover_url).content
                    original_image = Image.open(io.BytesIO(cover_data))
                    original_width, original_height = original_image.size

                    # Redimensionner à une largeur fixe de 900px
                    target_width = 900
                    aspect_ratio = original_height / original_width
                    new_width = target_width
                    new_height = int(new_width * aspect_ratio)
                    print(f"ℹ️ Resizing cover from {original_width}x{original_height} to {new_width}x{new_height}")
                    resized_image = original_image.resize((new_width, new_height), Image.Resampling.LANCZOS)
                    resized_image = resized_image.convert('RGB')
                    resized_image.save(cover_path, quality=95)

                    print(f"✅ Cover downloaded and resized: {cover_path}")

                    file_size = os.path.getsize(cover_path) / 1024  # Taille en KB
                    if file_size < 50:
                        print(f"⚠️ Cover file size is small ({file_size:.1f} KB), consider checking the source.")
                except Exception as e:
                    print(f"❌ Error downloading or resizing cover: {e}")
                    try:
                        with open(cover_path, 'wb') as f:
                            f.write(cover_data)
                        print(f"⚠️ Saved original cover without resizing: {cover_path}")
                    except Exception as e2:
                        print(f"❌ Failed to save original cover: {e2}")
            else:
                print("⚠️ Cover URL not found")

            # Télécharger le synopsis
            synopsis_text = None
            if site == "mangas-origines":
                synopsis_div = soup_oeuvre.select_one('.summary__content') or soup_oeuvre.select_one('.summary images')
                if synopsis_div:
                    synopsis_text = synopsis_div.get_text(separator=' ', strip=True)
            else:  # anime-sama.fr
                synopsis_div = soup_oeuvre.select_one('p.text-sm.text-gray-400.mt-2')
                if synopsis_div:
                    synopsis_text = synopsis_div.get_text(separator=' ', strip=True)

            if synopsis_text:
                sentences = [s.strip() for s in synopsis_text.split('.')]
                sentences = [s + '.' for s in sentences if s]
                if sentences and not sentences[-1].endswith('.'):
                    sentences[-1] = sentences[-1] + '.'
                formatted_synopsis = '\n\n'.join(sentences)

                title = f"{manga_name.replace('_', ' ')}"
                title_underline = '━' * len(title)

                with open(synopsis_path, 'w', encoding='utf-8') as f:
                    f.write(f"{title}\n{title_underline}\n\n{formatted_synopsis}\n\nSource: {start_url}")
                print(f"✅ Synopsis saved: {synopsis_path}")
            else:
                print("⚠️ Synopsis not found")

    except Exception as e:
        print(f"❌ Error fetching manga info: {e}")
else:
    print("ℹ️ Cover and synopsis already exist, skipping.")

# Téléchargement des chapitres
downloaded_chapters: List[Tuple[int, str, str]] = []

for idx, current_chapter in enumerate(chapters, 1):
    say(f"📥 Chapitre {current_chapter} ({idx}/{len(chapters)})")
    evt("chapter_start", num=current_chapter, idx=idx, total=len(chapters))
    time.sleep(0.1)  # Délai pour permettre à Rust de capturer les logs

    # Réinitialiser le cache
    if os.path.exists(cache_dir):
        shutil.rmtree(cache_dir)
    os.makedirs(cache_dir, exist_ok=True)

    try:
        img_urls = []
        
        if site == "mangas-origines":
            url = base_url + str(current_chapter) + "/"
            r = scraper.get(url)
            if r.status_code != 200:
                say(f"❌ Chapitre {current_chapter} : page inaccessible ({url})")
                evt("chapter_failed", num=current_chapter, reason="page inaccessible")
                downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", "Failed: Page not accessible"))
                continue

            soup = BeautifulSoup(r.text, 'html.parser')
            
            # Vérifier le mode de lecture (Manga ou Webcomic)
            reading_style_select = soup.select_one('select.selectpicker.reading-style-select')
            is_manga_mode = True  # Par défaut, on assume Manga
            if reading_style_select:
                selected_option = reading_style_select.find('option', selected=True)
                if selected_option and selected_option.text.strip().lower() == 'webcomic':
                    is_manga_mode = False
                    print(f"ℹ️ Webcomic mode detected for Chapter {current_chapter}")
                else:
                    print(f"ℹ️ Manga mode detected for Chapter {current_chapter}")
            
            if is_manga_mode:
                # Mode Manga : utiliser les URLs paginées
                page_select = soup.select_one('select#single-pager')
                if not page_select:
                    print(f"⚠️ No page selector found for Chapter {current_chapter}, falling back to Webcomic mode")
                    is_manga_mode = False
                
                if is_manga_mode:
                    # Extraire le nombre total de pages
                    page_options = page_select.find_all('option')
                    if not page_options:
                        print(f"⚠️ No pages found in page selector for Chapter {current_chapter}")
                        sys.stdout.write(f"No valid images for Chapter {current_chapter}\n")
                        sys.stdout.flush()
                        downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", "Failed: No pages found"))
                        continue
                    
                    # Déterminer le nombre total de pages (par exemple, "1/53" -> 53)
                    total_pages = 0
                    for option in page_options:
                        page_text = option.text.strip()  # Exemple : "1/53"
                        if '/' in page_text:
                            try:
                                total_pages = max(total_pages, int(page_text.split('/')[-1]))
                            except ValueError:
                                continue
                    
                    if total_pages == 0:
                        print(f"⚠️ Could not determine total pages for Chapter {current_chapter}")
                        sys.stdout.write(f"No valid images for Chapter {current_chapter}\n")
                        sys.stdout.flush()
                        downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", "Failed: No pages found"))
                        continue
                    
                    print(f"ℹ️ Found {total_pages} pages for Chapter {current_chapter}")
                    
                    # Obtenir le pattern d'URL des images depuis la première page
                    base_image_url = None
                    try:
                        # Charger la première page paginée pour extraire le pattern
                        first_page_url = f"{url}/p/1/"
                        r_first_page = scraper.get(first_page_url)
                        if r_first_page.status_code == 200:
                            soup_first_page = BeautifulSoup(r_first_page.text, 'html.parser')
                            img_tag = soup_first_page.find('img', class_='wp-manga-chapter-img') or \
                                    soup_first_page.find('img', src=re.compile(r'/WP-manga/data/'))
                            
                            if img_tag:
                                img_url = (
                                    img_tag.get('data-src') or
                                    img_tag.get('src') or
                                    img_tag.get('data-lazy-src') or
                                    img_tag.get('data-cfsrc')
                                )
                                if img_url and '/WP-manga/data/' in img_url:
                                    # Extraire le chemin de base (jusqu'au dossier contenant les images)
                                    base_image_url = '/'.join(img_url.split('/')[:-1]) + '/'
                                    print(f"ℹ️ Base image URL detected: {base_image_url}")
                                    
                                    # Générer toutes les URLs directement sans vérification
                                    print(f"ℹ️ Generating {total_pages} image URLs using detected pattern")
                                    for page_num in range(1, total_pages + 1):
                                        img_filename = f"{page_num:02d}.png"
                                        img_url = f"{base_image_url}{img_filename}"
                                        img_urls.append(img_url)
                                    
                                    print(f"✅ Generated {len(img_urls)} image URLs for Chapter {current_chapter}")
                    
                    except Exception as e:
                        print(f"⚠️ Error fetching first page to detect image pattern: {e}")
                    
                    # Si on n'a pas pu générer les URLs avec le pattern, fallback vers l'ancienne méthode
                    if not img_urls:
                        print(f"⚠️ Could not use image pattern, falling back to loading each page")
                        for page_num in range(1, total_pages + 1):
                            page_url = f"{url}/p/{page_num}/"
                            try:
                                r_page = scraper.get(page_url)
                                if r_page.status_code != 200:
                                    print(f"❌ Page {page_num} not accessible: {page_url}")
                                    continue
                                
                                soup_page = BeautifulSoup(r_page.text, 'html.parser')
                                img_tag = soup_page.find('img', class_='wp-manga-chapter-img') or \
                                        soup_page.find('img', src=re.compile(r'/WP-manga/data/'))
                                
                                if img_tag:
                                    img_url = (
                                        img_tag.get('data-src') or
                                        img_tag.get('src') or
                                        img_tag.get('data-lazy-src') or
                                        img_tag.get('data-cfsrc')
                                    )
                                    if img_url:
                                        if not img_url.startswith("http"):
                                            img_url = urllib.parse.urljoin(page_url, img_url)
                                        img_urls.append(img_url)
                                        print(f"ℹ️ Found image for page {page_num}: {img_url}")
                                    else:
                                        print(f"⚠️ No valid image URL found for page {page_num}")
                                else:
                                    print(f"⚠️ No image found for page {page_num}")
                            
                            except Exception as e:
                                print(f"❌ Error fetching page {page_num}: {e}")
                                continue
                            
                            time.sleep(0.2)  # Pause pour éviter de surcharger le serveur
            
            if not is_manga_mode or not img_urls:
                # Mode Webcomic ou fallback : récupérer toutes les images depuis la page principale
                img_tags = [img for img in soup.find_all('img') if "/uploads/" in (img.get('src') or '')]
                if len(img_tags) <= 1:
                    print(f"⚠️ No valid images for Chapter {current_chapter}")
                    sys.stdout.write(f"No valid images for Chapter {current_chapter}\n")
                    sys.stdout.flush()
                    downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", "Failed: No images"))
                    continue
                img_tags = img_tags[1:]  # Ignorer la première image

                for img in img_tags:
                    img_url = (
                        img.get('data-src') or
                        img.get('src') or
                        img.get('data-lazy-src') or
                        img.get('data-cfsrc')
                    )
                    if img_url:
                        if not img_url.startswith("http"):
                            img_url = urllib.parse.urljoin(url, img_url)
                        img_urls.append(img_url)
        
        else:  # anime-sama.fr
            # [Code existant pour anime-sama.fr inchangé]
            manga_name_for_url = manga_name.replace('_', ' ')
            base_img_url = f"https://anime-sama.fr/s2/scans/{manga_name_for_url}/{current_chapter}/"
            
            print(f"🔍 Building image URLs for chapter {current_chapter}: {base_img_url}")
            
            max_pages = 100
            for page in range(1, max_pages + 1):
                img_url = f"{base_img_url}{page}.jpg"
                try:
                    response = scraper.head(img_url, timeout=10)
                    if response.status_code == 200:
                        img_urls.append(img_url)
                        if page == 1:
                            print(f"✅ First image found: {img_url}")
                    else:
                        if page == 1:
                            print(f"❌ First image not found: {img_url}")
                        break
                except requests.RequestException as e:
                    if page == 1:
                        print(f"❌ Error checking first image: {e}")
                    break
                time.sleep(0.1)

        if not img_urls:
            print(f"⚠️ No valid images for Chapter {current_chapter}")
            sys.stdout.write(f"No valid images for Chapter {current_chapter}\n")
            sys.stdout.flush()
            downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", "Failed: No images"))
            continue

        say(f"🔍 Chapitre {current_chapter} : {len(img_urls)} image(s) trouvée(s)")
        evt("images", num=current_chapter, count=len(img_urls))
        time.sleep(0.1)

        # Télécharger les images
        final_images = []
        for i, img_url in enumerate(img_urls, 1):
            ext = os.path.splitext(img_url)[-1].split('?')[0]
            if ext.lower() not in ['.jpg', '.jpeg', '.png', '.webp']:
                ext = '.jpg'
            filename = os.path.join(cache_dir, f"{i:03d}{ext}")

            max_retries = 3
            success = False
            for attempt in range(max_retries):
                try:
                    img_data = scraper.get(img_url).content
                    img = Image.open(io.BytesIO(img_data))
                    if img.mode != "RGB":
                        img = img.convert("RGB")

                    max_width = 1200
                    if img.width > max_width:
                        ratio = max_width / img.width
                        new_size = (max_width, int(img.height * ratio))
                        img = img.resize(new_size, Image.Resampling.LANCZOS)

                    filename_jpg = os.path.splitext(filename)[0] + ".jpg"
                    img.save(filename_jpg, "JPEG", quality=78, optimize=True, progressive=True)
                    
                    # Vérifier la taille du fichier
                    #file_size = os.path.getsize(filename_jpg) / 1024  # Taille en KB
                    #if file_size < 50:  # Seuil pour détecter une image invalide
                        #print(f"⚠️ Image {i} is too small ({file_size:.1f} KB), likely invalid")
                        #sys.stdout.write(f"Image {i} is too small ({file_size:.1f} KB), likely invalid\n")
                        #sys.stdout.flush()
                        #break
                    
                    final_images.append(filename_jpg)
                    progress = (i / len(img_urls)) * 100
                    evt("image", num=current_chapter, i=i, n=len(img_urls))
                    if not TUI:
                        say(f"📄 Image {i}/{len(img_urls)} ({progress:.1f}%)")
                    success = True
                    break
                except Exception as e:
                    if attempt < max_retries - 1:
                        print(f"⚠️ Page {i} error (attempt {attempt + 1}/{max_retries}): {e}. Retrying...")
                        sys.stdout.write(f"Page {i} error (attempt {attempt + 1}/{max_retries}): {e}. Retrying...\n")
                        sys.stdout.flush()
                        time.sleep(1)
                    else:
                        print(f"❌ Page {i} failed after {max_retries} attempts: {e}")
                        sys.stdout.write(f"Page {i} failed after {max_retries} attempts: {e}\n")
                        sys.stdout.flush()
                        break
            time.sleep(0.2)

        if not final_images:
            say(f"⚠️ Chapitre {current_chapter} : aucune image téléchargée")
            evt("chapter_failed", num=current_chapter, reason="aucune image")
            downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", "Failed: No images downloaded"))
            continue

        # Créer le fichier CBR
        zip_path = os.path.join(manga_dir, f"Chapitre_{current_chapter:03d}.cbz")
        with ZipFile(zip_path, 'w') as myzip:
            for img in final_images:
                myzip.write(img, os.path.basename(img))

        say(f"✅ Chapitre_{current_chapter:03d}.cbz créé ({len(final_images)} image(s))")
        evt("chapter_done", num=current_chapter, images=len(final_images))
        downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", "Success"))

        # Nettoyage
        for file in os.listdir(cache_dir):
            os.remove(os.path.join(cache_dir, file))
        time.sleep(1)

    except Exception as e:
        say(f"❌ Chapitre {current_chapter} : erreur inattendue ({e})")
        evt("chapter_failed", num=current_chapter, reason=str(e))
        downloaded_chapters.append((current_chapter, f"Chapitre_{current_chapter:03d}", f"Failed: {e}"))

# Résumé final
_ok = len([c for c in downloaded_chapters if 'Success' in c[2]])
_ko = len(downloaded_chapters) - _ok
say(f"🎉 Terminé : {_ok} chapitre(s) téléchargé(s)" + (f", {_ko} en échec" if _ko else ""))
evt("done", ok=_ok, failed=_ko)

if downloaded_chapters:
    print("📋 Résumé du téléchargement:")
    success_count = 0
    for chap, title, status in downloaded_chapters:
        if 'Success' in status:
            success_count += 1
            print(f"✅ Chapitre {chap} - {title}: {status}")
            sys.stdout.write(f"✅ Chapitre {chap}: Terminé avec succès\n")
        else:
            print(f"❌ Chapitre {chap} - {title}: {status}")
            sys.stdout.write(f"❌ Chapitre {chap}: Échec - {status}\n")
    
    sys.stdout.write(f"📊 Statistiques finales: {success_count}/{len(downloaded_chapters)} chapitres réussis\n")
    sys.stdout.flush()
else:
    print("⚠️ Aucun chapitre n'a été téléchargé.")
    sys.stdout.write("⚠️ Aucun chapitre n'a été téléchargé.\n")
    sys.stdout.flush()
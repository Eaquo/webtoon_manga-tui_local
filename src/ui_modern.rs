use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, BorderType, Clear, List, ListItem, Padding, Paragraph, Wrap
    },
    Frame,
};
use ratatui_image::StatefulImage;

use crate::app::{App, AppState, InputField};
use crate::manga::Manga;
use crate::theme::Theme;

// 🎨 Palette de couleurs adaptée aux thèmes wallust
pub struct ModernColors;

impl ModernColors {
    // Fonction pour obtenir les couleurs à partir du thème wallust
    pub fn get_colors(theme: &Theme) -> WallustColors {
        WallustColors {
            // Utiliser les couleurs wallust avec des fallbacks modernes
            primary: theme.colors[4],           // color4 (bleu)
            accent: theme.colors[2],            // color2 (vert)
            error: theme.colors[1],             // color1 (rouge)
            
            // Interface basée sur wallust
            background: theme.background,       // Background wallust
            border: theme.colors[8],            // color8 (gris)
            border_focus: theme.colors[12],     // color12 (bleu clair)
            
            // Texte basé sur wallust
            text_primary: theme.foreground,     // Foreground wallust
            text_secondary: theme.colors[7],    // color7 (blanc/gris clair)
            text_muted: theme.colors[15],       // color15 (gris)
            
            // Status avec couleurs wallust
            success: theme.colors[10],          // color10 (vert clair)
        }
    }

    // Fallbacks pour les couleurs si wallust n'est pas disponible
    pub const PRIMARY: Color = Color::Rgb(129, 140, 248);
    pub const SECONDARY: Color = Color::Rgb(139, 92, 246);
    pub const ACCENT: Color = Color::Rgb(34, 197, 94);
    pub const WARNING: Color = Color::Rgb(251, 191, 36);
    pub const ERROR: Color = Color::Rgb(239, 68, 68);
    pub const BACKGROUND: Color = Color::Rgb(15, 23, 42);
    pub const SURFACE: Color = Color::Rgb(30, 41, 59);
    pub const BORDER: Color = Color::Rgb(71, 85, 105);
    pub const BORDER_FOCUS: Color = Color::Rgb(129, 140, 248);
    pub const TEXT_PRIMARY: Color = Color::Rgb(248, 250, 252);
    pub const TEXT_SECONDARY: Color = Color::Rgb(148, 163, 184);
    pub const TEXT_MUTED: Color = Color::Rgb(100, 116, 139);
    pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
}

// Structure pour contenir les couleurs wallust
pub struct WallustColors {
    pub primary: Color,
    pub accent: Color,
    pub error: Color,
    pub background: Color,
    pub border: Color,
    pub border_focus: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub success: Color,
}

// 🎨 Helpers pour les barres de progression élégantes
impl WallustColors {
    // Génère une barre de progression discrète et moderne
    pub fn create_subtle_progress_bar(&self, progress: f32, width: usize, style: ProgressStyle) -> String {
        let filled = (progress * width as f32) as usize;
        let empty = width.saturating_sub(filled);
        
        match style {
            ProgressStyle::Dots => {
                format!("{}{}",
                    "●".repeat(filled.min(width)),
                    "○".repeat(empty.min(width))
                )
            }
            ProgressStyle::Blocks => {
                let full_blocks = filled;
                let empty_blocks = empty;
                format!("{}{}",
                    "█".repeat(full_blocks.min(width)),
                    "░".repeat(empty_blocks.min(width))
                )
            }
            ProgressStyle::Minimal => {
                if progress >= 1.0 {
                    "▰".repeat(width)
                } else if progress > 0.0 {
                    let filled_count = (progress * width as f32).ceil() as usize;
                    format!("{}{}",
                        "▰".repeat(filled_count.min(width)),
                        "▱".repeat((width - filled_count.min(width)).min(width))
                    )
                } else {
                    "▱".repeat(width)
                }
            }
        }
    }

    // Obtient la couleur appropriée selon le statut
    pub fn get_progress_color(&self, progress: f32) -> Color {
        if progress >= 1.0 {
            self.success  // Vert pour complété
        } else if progress > 0.0 {
            self.primary  // Bleu wallust pour en cours
        } else {
            self.text_muted  // Gris pour non commencé
        }
    }
}

#[derive(Clone, Copy)]
pub enum ProgressStyle {
    Dots,      // Points avec ● et ○
    Blocks,    // Blocs avec █ et ░
    Minimal,   // Style minimal avec ▰ et ▱
}

// 📱 Icônes Unicode modernes
pub struct Icons;

impl Icons {
    // Status
    pub const READ: &'static str = "✓";
    pub const IN_PROGRESS: &'static str = "⏯";
    pub const UNREAD: &'static str = "➤";
    pub const BOOKMARK: &'static str = "⟢";
    
    // Navigation
    pub const FOLDER: &'static str = "📁";
    pub const FILE: &'static str = "📄";
    pub const DOWNLOAD: &'static str = "⬇️";
    pub const REFRESH: &'static str = "↺";
    pub const SETTINGS: &'static str = "⚙️";
    pub const HELP: &'static str = "❓";
    
    // Interface
    pub const MANGA: &'static str = "📖";
    pub const CHAPTER: &'static str = "";
    pub const IMAGE: &'static str = "🖼️";
    pub const LINK: &'static str = "🔗";
    
    // Progress
    pub const ARROW_RIGHT: &'static str = "▶";
    pub const DOT: &'static str = "•";
}

pub fn draw_modern(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let colors = ModernColors::get_colors(&app.theme);
    
    // Layout principal avec header moderne
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Header
            Constraint::Min(1),       // Content
            Constraint::Length(3),    // Footer avec stats
        ])
        .split(area);

    draw_modern_header(f, app, main_layout[0], &colors);
    
    match app.state {
        AppState::BrowseManga | AppState::DownloadInput | AppState::Downloading => {
            draw_modern_browse(f, app, main_layout[1], &colors)
        }
        AppState::ViewMangaDetails => draw_modern_details(f, app, main_layout[1], &colors),
        AppState::Settings => draw_modern_settings(f, app, main_layout[1], &colors),
    };

    draw_modern_footer(f, app, main_layout[2], &colors);

    if app.show_help {
        draw_modern_help_overlay(f, app, area, &colors);
    }

    app.reset_refresh();
}

fn draw_modern_header(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),   // Logo/Titre
            Constraint::Min(1),       // Espace central
            Constraint::Length(25),   // Stats/Info
        ])
        .split(area);

    // Logo stylisé avec couleurs wallust
    let logo = format!("{} Manga Reader", Icons::MANGA);
    let logo_widget = Paragraph::new(logo)
        .style(Style::default().fg(colors.primary).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        )
        .alignment(Alignment::Left);
    f.render_widget(logo_widget, header_layout[0]);

    // Info centrale dynamique
    let center_text = match app.state {
        AppState::BrowseManga => {
            if let Some(manga) = app.current_manga() {
                format!("{} {}", Icons::BOOKMARK, manga.name.replace("_", " "))
            } else {
                format!("{} Sélectionnez un manga", Icons::FOLDER)
            }
        }
        AppState::DownloadInput => format!("{} Téléchargement", Icons::DOWNLOAD),
        AppState::Downloading => format!("{} Téléchargement en cours...", Icons::DOWNLOAD),
        _ => "Manga Reader".to_string(),
    };
    
    let center_widget = Paragraph::new(center_text)
        .style(Style::default().fg(colors.text_primary))
        .alignment(Alignment::Center);
    f.render_widget(center_widget, header_layout[1]);

    // Stats rapides avec couleurs wallust
    let stats_text = format!("Mangas: {} • Focus: {}", 
                app.mangas.len(),
                if app.is_manga_list_focused { "List" } else { "Chapters" });
    
    let stats_widget = Paragraph::new(stats_text)
        .style(Style::default().fg(colors.text_secondary))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        )
        .alignment(Alignment::Right);
    f.render_widget(stats_widget, header_layout[2]);
}

fn draw_modern_browse(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),  // Liste mangas
            Constraint::Percentage(32),  // Liste chapitres  
            Constraint::Percentage(40),  // Info + Preview
        ])
        .margin(1)
        .split(area);

    draw_modern_manga_list(f, app, main_layout[0], colors);
    draw_modern_chapter_list(f, app, main_layout[1], colors);
    
    let info_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(65),  // Preview + Synopsis
            Constraint::Percentage(35),  // Actions/Download
        ])
        .split(main_layout[2]);
    
    draw_modern_manga_info(f, app, info_layout[0], colors);
    
    match app.state {
        AppState::DownloadInput => draw_modern_download_input(f, app, info_layout[1], colors),
        AppState::Downloading => draw_modern_downloading(f, app, info_layout[1], colors),
        _ => draw_modern_quick_actions(f, app, info_layout[1], colors),
    }
}

fn draw_modern_manga_list(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    // Le bloc délimite les emprunts immuables sur `app` : la zone et le
    // décalage ne peuvent être mémorisés qu'une fois ces emprunts relâchés.
    let rendered_offset;
    let captured_item_height;
    {
    let filtered_mangas_vec: Vec<&Manga> = app.filtered_mangas().collect();
    
    let items: Vec<ListItem> = filtered_mangas_vec
        .iter()
        .enumerate()
        .map(|(idx, manga)| {
            let (_read, _total, progress) = app.manga_progress(manga);
            let display_name = manga.name.replace("_", " ");
            
            // Icône de status
            let status_icon = if progress >= 1.0 {
                Icons::READ
            } else if progress > 0.0 {
                Icons::IN_PROGRESS
            } else {
                Icons::UNREAD
            };
            
            // Couleur wallust basée sur le progrès
            let progress_color = colors.get_progress_color(progress);
            
            // Barre de progression élégante et discrète
            let progress_bar = colors.create_subtle_progress_bar(progress, 15, ProgressStyle::Minimal);
            
            let is_selected = app.selected_manga == Some(idx);
            let title_style = if is_selected {
                Style::default().fg(colors.text_primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.text_primary)
            };
            
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(status_icon, Style::default().fg(progress_color)),
                    Span::raw(" "),
                    Span::styled(display_name, title_style),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(progress_bar, Style::default().fg(progress_color)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:.0}%", progress * 100.0),
                        Style::default().fg(colors.text_muted)
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{} {} chapters", Icons::CHAPTER, manga.chapters.len()),
                        Style::default().fg(colors.text_secondary)
                    ),
                ]),
            ])
        })
        .collect();

    let border_color = if app.is_manga_list_focused {
        colors.border_focus
    } else {
        colors.border
    };
    
    let item_height = items.first().map(|i| i.height() as u16).unwrap_or(1).max(1);

    let manga_list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} Bibliothèque ({}) ", Icons::MANGA, filtered_mangas_vec.len()))
                .title_style(Style::default().fg(colors.text_primary).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .padding(Padding::horizontal(1))
        )
        .highlight_style(
            Style::default()
                .bg(colors.primary)
                .fg(colors.background)
                .add_modifier(Modifier::BOLD)
        );

    let mut state = ratatui::widgets::ListState::default();
    if let Some(idx) = app.selected_manga {
        if idx < filtered_mangas_vec.len() {
            state.select(Some(idx));
        }
    }
    f.render_stateful_widget(manga_list, area, &mut state);
    // `render_stateful_widget` ajuste l'offset pour rendre la sélection
    // visible : on le relit après coup, pas avant.
    rendered_offset = state.offset();
    captured_item_height = item_height;
    }

    // Mémorisés pour convertir une ligne cliquée en index (voir handle_key).
    app.manga_list_area = Some(area);
    app.manga_list_offset = rendered_offset;
    app.manga_list_item_height = captured_item_height;
}

fn draw_modern_chapter_list(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    let mut rendered_offset = None;
    let mut item_height = 1u16;
    {
    let border_color = if !app.is_manga_list_focused {
        colors.border_focus
    } else {
        colors.border
    };

    if let Some(manga) = app.current_manga() {
        let items: Vec<ListItem> = manga
            .chapters
            .iter()
            .enumerate()
            .map(|(_idx, chapter)| {
                let (status_icon, status_color) = match (chapter.read, chapter.last_page_read, chapter.full_pages_read) {
                    (true, _, _) => (Icons::READ, colors.success),
                    (false, Some(last), Some(total)) if last > 0 && last < total => {
                        (Icons::IN_PROGRESS, colors.primary)
                    },
                    _ => (Icons::UNREAD, colors.text_muted),
                };
                
                // Progress dans le chapitre
                let progress_text = match (chapter.last_page_read, chapter.full_pages_read) {
                    (Some(page), Some(total)) => {
                        let progress = page as f32 / total as f32;
                        let progress_bar = colors.create_subtle_progress_bar(progress, 8, ProgressStyle::Dots);
                        format!(" {} {}/{}", progress_bar, page, total)
                    },
                    (Some(page), None) => format!(" [Page {}]", page),
                    _ => String::new(),
                };
                
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::raw(" "),
                        Span::styled(
                            format!("{} - {}", chapter.number_display(), chapter.title),
                            Style::default().fg(ModernColors::TEXT_PRIMARY)
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(chapter.size_display(), Style::default().fg(ModernColors::TEXT_SECONDARY)),
                        Span::styled(progress_text, Style::default().fg(status_color)),
                    ]),
                ])
            })
            .collect();

        let display_name = manga.name.replace("_", " ");
        item_height = items.first().map(|i| i.height() as u16).unwrap_or(1).max(1);

        let chapter_list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" {} {} ({} ch.) ", Icons::CHAPTER, display_name, manga.chapters.len()))
                    .title_style(Style::default().fg(ModernColors::TEXT_PRIMARY).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .padding(Padding::horizontal(1))
            )
            .highlight_style(
                Style::default()
                    .bg(ModernColors::SECONDARY)
                    .fg(ModernColors::BACKGROUND)
                    .add_modifier(Modifier::BOLD)
            );

        let mut chapter_state = ratatui::widgets::ListState::default();
        if let Some(idx) = app.selected_chapter {
            if idx < manga.chapters.len() {
                chapter_state.select(Some(idx));
            }
        }
        f.render_stateful_widget(chapter_list, area, &mut chapter_state);
        rendered_offset = Some(chapter_state.offset());
    } else {
        let empty_widget = Paragraph::new(format!("{} Aucun manga sélectionné", Icons::FOLDER))
            .style(Style::default().fg(ModernColors::TEXT_MUTED))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(format!(" {} Chapitres ", Icons::CHAPTER))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
            );
        f.render_widget(empty_widget, area);
    }
    }

    // Mémorisés pour convertir une ligne cliquée en index (voir handle_key).
    app.chapter_list_area = Some(area);
    if let Some(offset) = rendered_offset {
        app.chapter_list_offset = offset;
        app.chapter_list_item_height = item_height;
    }
}

fn draw_modern_manga_info(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    let info_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),  // Image de couverture
            Constraint::Percentage(65),  // Synopsis + info
        ])
        .split(area);

    // Image de couverture moderne
    draw_modern_cover_image(f, app, info_layout[0], colors);
    
    let text_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(75),  // Synopsis
            Constraint::Percentage(25),  // Source link
        ])
        .split(info_layout[1]);

    draw_modern_synopsis(f, app, text_layout[0], colors);
    draw_modern_source_link(f, app, text_layout[1], colors);
}

fn draw_modern_cover_image(f: &mut Frame, app: &mut App, area: Rect, _colors: &WallustColors) {
    let cover_block = Block::default()
        .title(format!(" {} Couverture ", Icons::IMAGE))
        .title_style(Style::default().fg(ModernColors::TEXT_PRIMARY))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ModernColors::BORDER))
        .padding(Padding::uniform(1));
    
    f.render_widget(&cover_block, area);
    let inner_area = cover_block.inner(area);

    // Sans protocole graphique, ratatui-image se rabat sur des demi-blocs :
    // une couverture y devient une bouillie de pixels colorés. On préfère
    // afficher une fiche sobre, qui reste lisible et informative.
    if app.image_protocol == ratatui_image::picker::ProtocolType::Halfblocks {
        draw_cover_fallback(f, app, inner_area);
    } else if app.render_image && app.config.settings.enable_image_rendering {
        if let Some(state) = &mut app.image_state {
            let image_widget = StatefulImage::new(None);
            f.render_stateful_widget(image_widget, inner_area, state);
        } else {
            let placeholder_text = if app.pending_image_load.is_some() {
                format!("{} Chargement...", Icons::REFRESH)
            } else {
                format!("{} Pas d'image", Icons::IMAGE)
            };
            
            let image_placeholder = Paragraph::new(placeholder_text)
                .style(Style::default().fg(ModernColors::TEXT_MUTED))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(image_placeholder, inner_area);
        }
    } else {
        let disabled_text = format!("{} Rendu désactivé", Icons::IMAGE);
        let image_disabled = Paragraph::new(disabled_text)
            .style(Style::default().fg(ModernColors::TEXT_MUTED))
            .alignment(Alignment::Center);
        f.render_widget(image_disabled, inner_area);
    }
}

/// Remplace la couverture quand le terminal ne sait pas afficher d'images.
///
/// Affiche le titre et l'avancement plutôt qu'une image dégradée. Alacritty,
/// par exemple, n'implémente aucun protocole graphique — c'est un choix
/// assumé de ses auteurs, pas une lacune temporaire.
fn draw_cover_fallback(f: &mut Frame, app: &mut App, area: Rect) {
    let (title, progress_line) = match app.current_manga() {
        Some(manga) => {
            let (read, total, ratio) = app.manga_progress(manga);
            (
                manga.name.replace('_', " "),
                format!("{} / {} chapitres  ({:.0} %)", read, total, ratio * 100.0),
            )
        }
        None => (String::from("Aucun manga sélectionné"), String::new()),
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{}", Icons::MANGA),
            Style::default().fg(ModernColors::SECONDARY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(ModernColors::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    if !progress_line.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            progress_line,
            Style::default().fg(ModernColors::TEXT_SECONDARY),
        )));
    }

    // La zone est souvent courte : l'explication n'est affichée que si elle
    // ne risque pas de repousser le titre hors du cadre.
    if area.height >= 9 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Couvertures indisponibles dans ce terminal",
            Style::default().fg(ModernColors::TEXT_MUTED),
        )));
    }

    let widget = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}

fn draw_modern_synopsis(f: &mut Frame, app: &mut App, area: Rect, _colors: &WallustColors) {
    let synopsis = app.current_manga()
        .and_then(|manga| manga.synopsis.as_ref())
        .unwrap_or(&"Aucun synopsis disponible.".to_string())
        .clone();

    let synopsis_widget = Paragraph::new(synopsis)
        .style(Style::default().fg(ModernColors::TEXT_PRIMARY))
        .block(
            Block::default()
                .title(format!(" {} Synopsis ", Icons::FILE))
                .title_style(Style::default().fg(ModernColors::TEXT_PRIMARY))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ModernColors::BORDER))
                .padding(Padding::uniform(1))
        )
        .wrap(Wrap { trim: true });
    
    f.render_widget(synopsis_widget, area);
}

fn draw_modern_source_link(f: &mut Frame, app: &mut App, area: Rect, _colors: &WallustColors) {
    let source_url = app.current_manga()
        .and_then(|manga| manga.source_url.as_ref())
        .unwrap_or(&"Aucune source disponible".to_string())
        .clone();

    let link_widget = Paragraph::new(format!("{} {}", Icons::LINK, source_url))
        .style(Style::default().fg(ModernColors::ACCENT).add_modifier(Modifier::UNDERLINED))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ModernColors::BORDER))
                .padding(Padding::horizontal(1))
        )
        .alignment(Alignment::Left);
    
    f.render_widget(link_widget, area);
    app.source_link_area = Some(area);
}

fn draw_modern_quick_actions(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    let actions_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    // Bouton Download - Style discret
    let download_btn = Paragraph::new(format!("{} Télécharger (d)", Icons::DOWNLOAD))
        .style(Style::default().fg(colors.accent))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        );
    f.render_widget(download_btn, actions_layout[0]);

    // Bouton Refresh - Style discret
    let refresh_btn = Paragraph::new(format!("{} Actualiser (r)", Icons::REFRESH))
        .style(Style::default().fg(colors.primary))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        );
    f.render_widget(refresh_btn, actions_layout[1]);

    // Stats détaillées
    if let Some(manga) = app.current_manga() {
        let (read, total, progress) = app.manga_progress(manga);
        let stats_text = format!(
            "📊 Statistiques\n\n{} Chapitres lus: {}/{}\n{} Progression: {:.1}%\n{} Taille totale: {}",
            Icons::READ, read, total,
            Icons::IN_PROGRESS, progress * 100.0,
            Icons::FOLDER, 
            manga.chapters.iter().map(|c| c.size).sum::<u64>() as f64 / (1024.0 * 1024.0)
        );
        
        let stats_widget = Paragraph::new(stats_text)
            .style(Style::default().fg(colors.text_secondary))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(colors.border))
                    .padding(Padding::uniform(1))
            );
        f.render_widget(stats_widget, actions_layout[2]);
    }
}

fn draw_modern_download_input(f: &mut Frame, app: &mut App, area: Rect, _colors: &WallustColors) {
    let input_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    // URL Input
    let url_focused = app.input_field == InputField::Url;
    let url_style = if url_focused {
        Style::default().fg(ModernColors::TEXT_PRIMARY).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ModernColors::TEXT_SECONDARY)
    };

    let url_input = Paragraph::new(app.download_url.as_str())
        .style(url_style)
        .block(
            Block::default()
                .title(format!(" {} URL ", Icons::LINK))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if url_focused { 
                    ModernColors::BORDER_FOCUS 
                } else { 
                    ModernColors::BORDER 
                }))
                .padding(Padding::horizontal(1))
        );
    f.render_widget(url_input, input_layout[0]);

    // Chapters Input
    let chapters_focused = app.input_field == InputField::Chapters;
    let chapters_style = if chapters_focused {
        Style::default().fg(ModernColors::TEXT_PRIMARY).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ModernColors::TEXT_SECONDARY)
    };

    let chapters_input = Paragraph::new(app.selected_chapters_input.as_str())
        .style(chapters_style)
        .block(
            Block::default()
                .title(format!(" {} Chapitres (ex: 1,2,3 ou 1-3) ", Icons::CHAPTER))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if chapters_focused { 
                    ModernColors::BORDER_FOCUS 
                } else { 
                    ModernColors::BORDER 
                }))
                .padding(Padding::horizontal(1))
        );
    f.render_widget(chapters_input, input_layout[1]);

    // Instructions
    let instructions = format!(
        "{} Navigation: Tab pour changer de champ\n{} Action: Enter pour télécharger\n{} Annuler: Esc",
        Icons::ARROW_RIGHT, Icons::DOWNLOAD, Icons::DOT
    );
    
    let instructions_widget = Paragraph::new(instructions)
        .style(Style::default().fg(ModernColors::TEXT_MUTED))
        .block(
            Block::default()
                .title(format!(" {} Aide ", Icons::HELP))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ModernColors::BORDER))
                .padding(Padding::uniform(1))
        );
    f.render_widget(instructions_widget, input_layout[2]);
}

fn draw_modern_downloading(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    let download_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),   // Status header élégant
            Constraint::Length(5),   // Progress et stats détaillées
            Constraint::Min(1),      // Logs avec couleurs améliorées
            Constraint::Length(3),   // Actions footer
        ])
        .split(area);

    // 🎨 Header de statut élégant avec icônes animés
    let (total_chapters, completed_chapters, progress, _, _, current_chapter) = 
        app.calculate_download_progress();

    let status_icon = if app.download_finished {
        "✅"
    } else {
        "📥" // Icône de téléchargement plus moderne
    };

    let manga_title = app.current_download_manga_name.replace('_', " ");
    let status_text = if app.download_finished {
        format!("{} Téléchargement terminé", status_icon)
    } else {
        format!("{} Téléchargement en cours...", status_icon)
    };

    let header_widget = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(status_text, Style::default().fg(colors.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("📚 ", Style::default().fg(colors.accent)),
            Span::styled(manga_title, Style::default().fg(colors.text_primary)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(colors.border))
            .padding(Padding::horizontal(2))
    )
    .alignment(Alignment::Center);
    f.render_widget(header_widget, download_layout[0]);

    // 🎯 Deux barres : avancement global, puis avancement du chapitre en cours.
    let progress_percent = progress;
    let chapter_ratio = app.download.chapter_ratio();
    let global_bar = colors.create_subtle_progress_bar(progress_percent / 100.0, 28, ProgressStyle::Blocks);
    let chapter_bar = colors.create_subtle_progress_bar(chapter_ratio, 28, ProgressStyle::Blocks);

    let images_label = if app.download.images_total > 0 {
        format!(" {}/{} images", app.download.images_done, app.download.images_total)
    } else {
        " en attente…".to_string()
    };

    let chapter_label = if current_chapter > 0 {
        format!("Chapitre {}", current_chapter)
    } else {
        "Préparation".to_string()
    };
    let position_label = if app.download.current_index > 0 {
        format!("{}/{}", app.download.current_index, total_chapters)
    } else {
        format!("0/{}", total_chapters)
    };

    let mut detail_line = vec![
        Span::styled(chapter_label, Style::default().fg(colors.accent).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  •  {} ", position_label), Style::default().fg(colors.text_secondary)),
        Span::styled(format!(" ✅ {}", completed_chapters), Style::default().fg(colors.success)),
    ];
    if app.download.failed_chapters > 0 {
        detail_line.push(Span::styled(
            format!("  ❌ {}", app.download.failed_chapters),
            Style::default().fg(colors.error),
        ));
    }

    let stats_content = vec![
        Line::from(vec![
            Span::styled("Global   ", Style::default().fg(colors.text_secondary)),
            Span::styled(global_bar, Style::default().fg(colors.get_progress_color(progress_percent / 100.0))),
            Span::styled(format!(" {:>5.1}%", progress_percent), Style::default().fg(colors.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("Chapitre ", Style::default().fg(colors.text_secondary)),
            Span::styled(chapter_bar, Style::default().fg(colors.get_progress_color(chapter_ratio))),
            Span::styled(images_label, Style::default().fg(colors.text_muted)),
        ]),
        Line::from(detail_line),
    ];

    let progress_widget = Paragraph::new(stats_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(2))
        )
        .alignment(Alignment::Left);
    f.render_widget(progress_widget, download_layout[1]);

    // 📝 Logs avec coloration intelligente et style amélioré
    // webtoon-dl préfixe déjà chaque ligne par une icône : on ne la double pas,
    // on se contente de colorer la ligne selon sa nature.
    let logs_text: Vec<Line> = app.download_logs
        .iter()
        .map(|log| {
            let color = if log.contains("❌") || log.contains("échec") || log.contains("Error") {
                colors.error
            } else if log.contains("⚠️") {
                colors.accent
            } else if log.contains("✅") || log.contains("🎉") {
                colors.success
            } else if log.contains("📥") {
                colors.primary
            } else if log.contains("🔍") {
                colors.accent
            } else {
                colors.text_secondary
            };

            let already_has_icon = log
                .trim_start()
                .chars()
                .next()
                .map(|c| !c.is_ascii())
                .unwrap_or(false);

            if already_has_icon {
                Line::from(Span::styled(log.clone(), Style::default().fg(color)))
            } else {
                Line::from(vec![
                    Span::styled("· ", Style::default().fg(colors.text_muted)),
                    Span::styled(log.clone(), Style::default().fg(color)),
                ])
            }
        })
        .collect();

    let logs_widget = Paragraph::new(Text::from(logs_text))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0))
        .block(
            Block::default()
                .title(" 📜 Logs de téléchargement ")
                .title_style(Style::default().fg(colors.text_primary).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        );
    f.render_widget(logs_widget, download_layout[2]);

    // 🎮 Footer avec actions disponibles
    let actions_text = if app.download_finished {
        "Enter: Retour • r: Nouveau téléchargement • q: Quitter"
    } else {
        "j/k: Défiler logs • Esc: Annuler • q: Quitter"
    };

    let footer_widget = Paragraph::new(actions_text)
        .style(Style::default().fg(colors.text_muted))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        );
    f.render_widget(footer_widget, download_layout[3]);
}

fn draw_modern_details(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    // Layout principal pour les détails
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Contenu principal
            Constraint::Percentage(30),  // Panneau latéral avec image
        ])
        .split(area);

    // Layout pour le contenu principal
    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),    // Info manga
            Constraint::Min(1),       // Liste des chapitres
        ])
        .split(main_layout[0]);

    // Afficher les informations du manga
    draw_modern_manga_info(f, app, content_layout[0], colors);

    // Afficher la liste des chapitres en mode détails
    draw_modern_chapter_list(f, app, content_layout[1], colors);

    // Layout pour le panneau latéral
    let side_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),  // Image de couverture
            Constraint::Percentage(40),  // Synopsis ou actions
        ])
        .split(main_layout[1]);

    // Afficher l'image de couverture
    draw_modern_cover_image(f, app, side_layout[0], colors);

    // Afficher le synopsis ou les actions rapides
    if let Some(manga) = app.current_manga() {
        if manga.synopsis.is_some() && !manga.synopsis.as_ref().unwrap().is_empty() {
            draw_modern_synopsis(f, app, side_layout[1], colors);
        } else {
            draw_modern_quick_actions(f, app, side_layout[1], colors);
        }
    }
}

fn draw_modern_settings(f: &mut Frame, app: &mut App, area: Rect, _colors: &WallustColors) {
    let block = Block::default()
        .title(format!(" {} Settings ", Icons::SETTINGS))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ModernColors::BORDER_FOCUS))
        .padding(Padding::uniform(1));

    let input_text = if app.input_mode {
        if app.input_field == InputField::MangaDir {
            format!("Manga Directory: {}", app.filter)
        } else {
            "Enter path and press Enter to confirm".to_string()
        }
    } else {
        format!("Current Directory: {}", app.manga_dir.display())
    };

    let content = vec![
        Line::from(vec![
            Span::styled("📁 ", Style::default().fg(ModernColors::ACCENT)),
            Span::styled("Manga Directory Configuration", Style::default().fg(ModernColors::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Current: ", Style::default().fg(ModernColors::TEXT_SECONDARY)),
            Span::styled(app.manga_dir.display().to_string(), Style::default().fg(ModernColors::TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(input_text),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(ModernColors::TEXT_SECONDARY)),
            Span::styled("Enter", Style::default().fg(ModernColors::ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" to confirm, ", Style::default().fg(ModernColors::TEXT_SECONDARY)),
            Span::styled("Esc", Style::default().fg(ModernColors::ERROR).add_modifier(Modifier::BOLD)),
            Span::styled(" to cancel", Style::default().fg(ModernColors::TEXT_SECONDARY)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn draw_modern_footer(f: &mut Frame, app: &mut App, area: Rect, colors: &WallustColors) {
    let footer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Status
            Constraint::Percentage(60),  // Raccourcis
        ])
        .split(area);

    // Status avec icônes et couleurs wallust
    let status_text = format!("{} {}", Icons::DOT, app.status);
    let status_widget = Paragraph::new(status_text)
        .style(Style::default().fg(colors.text_secondary))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        )
        .alignment(Alignment::Left);
    f.render_widget(status_widget, footer_layout[0]);

    // Raccourcis contextuels modernes
    let keys = match app.state {
        AppState::BrowseManga => {
            if app.is_manga_list_focused {
                format!("Enter:Focus {} • j/k:Nav • r:Refresh {} • d:Download {} • ?:Help {}", 
                       Icons::CHAPTER, Icons::REFRESH, Icons::DOWNLOAD, Icons::HELP)
            } else {
                format!("Tab:Focus {} • j/k:Nav • Enter:Read {} • ?:Help {}", 
                       Icons::MANGA, Icons::CHAPTER, Icons::HELP)
            }
        }
        AppState::DownloadInput => format!("Tab:Switch • Enter:Download {} • Esc:Cancel", Icons::DOWNLOAD),
        AppState::Downloading => format!("j/k:Scroll • Esc:Cancel • r:Refresh {}", Icons::REFRESH),
        _ => "Navigation: j/k • Actions: Enter • Aide: ?".to_string(),
    };

    let keys_widget = Paragraph::new(keys)
        .style(Style::default().fg(colors.text_muted))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(colors.border))
                .padding(Padding::horizontal(1))
        )
        .alignment(Alignment::Right);
    f.render_widget(keys_widget, footer_layout[1]);
}

fn draw_modern_help_overlay(f: &mut Frame, _app: &mut App, area: Rect, _colors: &WallustColors) {
    let popup_area = centered_rect(70, 80, area);
    
    // Overlay sombre
    f.render_widget(Clear, popup_area);
    
    let help_block = Block::default()
        .title(format!(" {} Aide - Manga Reader ", Icons::HELP))
        .title_style(Style::default().fg(ModernColors::TEXT_PRIMARY).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ModernColors::BORDER_FOCUS))
        .style(Style::default().bg(ModernColors::SURFACE))
        .padding(Padding::uniform(1));
    
    f.render_widget(&help_block, popup_area);
    let inner_area = help_block.inner(popup_area);

    let help_text = vec![
        Line::from(vec![
            Span::styled(format!("{} Navigation", Icons::ARROW_RIGHT), 
                        Style::default().fg(ModernColors::ACCENT).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  j/k ou ↑/↓ : Naviguer haut/bas"),
        Line::from("  Tab : Changer de focus (Manga/Chapitres)"),
        Line::from("  ←/→ : Focus Manga/Chapitres"),
        Line::from(""),
        
        Line::from(vec![
            Span::styled(format!("{} Actions", Icons::DOWNLOAD), 
                        Style::default().fg(ModernColors::PRIMARY).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  Enter/o : Ouvrir chapitre"),
        Line::from("  m : Marquer lu/non-lu"),
        Line::from("  M : Marquer tous non-lus"),
        Line::from("  d : Télécharger"),
        Line::from("  r : Actualiser la liste"),
        Line::from(""),
        
        Line::from(vec![
            Span::styled(format!("{} Interface", Icons::SETTINGS), 
                        Style::default().fg(ModernColors::WARNING).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  / : Filtrer les mangas"),
        Line::from("  c : Paramètres"),
        Line::from("  ? : Cette aide"),
        Line::from("  q : Quitter"),
        Line::from(""),
        
        Line::from(vec![
            Span::styled(format!("{} Téléchargement", Icons::DOWNLOAD), 
                        Style::default().fg(ModernColors::SUCCESS).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  Tab : Changer de champ"),
        Line::from("  Enter : Commencer le téléchargement"),
        Line::from("  Esc : Annuler"),
    ];

    let help_widget = Paragraph::new(Text::from(help_text))
        .style(Style::default().fg(ModernColors::TEXT_PRIMARY))
        .wrap(Wrap { trim: true });
    
    f.render_widget(help_widget, inner_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
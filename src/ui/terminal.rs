use std::{
    cmp,
    collections::BTreeSet,
    io::{self, Write},
    sync::OnceLock,
    time::{Duration, SystemTime},
};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute, queue,
    style::{Attribute, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme as SyntectTheme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
    util::as_24_bit_terminal_escaped,
};

use crate::{
    core::{config::AppConfig, document::Document, theme::ThemeTokens},
    io::fs::FileSystemDocumentSource,
    io::kitty_graphics::{
        DeleteCommand, KittyImagePlacement, encode_delete, encode_place, encode_transmit_png,
    },
    io::{browser, image_decoder::ImageDecoder, mermaid_cli::MermaidCliRenderer},
    render::{
        markdown::parse_document,
        text::{
            RenderedDocument, RenderedGraphic, RenderedGraphicContent, RenderedLine,
            RenderedLineKind, render_document, scaled_graphic_height,
        },
    },
};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
static FALLBACK_THEME: OnceLock<SyntectTheme> = OnceLock::new();

#[derive(Clone, Copy)]
struct GraphicViewport {
    scroll: usize,
    content_height: usize,
    article_x: u16,
    article_width: u16,
    viewport_width: u16,
}

pub struct TerminalViewer {
    config: AppConfig,
    source: FileSystemDocumentSource,
    document: Document,
    rendered: RenderedDocument,
    scroll: usize,
    last_modified: Option<SystemTime>,
    warning: Option<String>,
    needs_redraw: bool,
    visible_placements: Vec<(u32, u32)>,
    transmitted_graphics: BTreeSet<u32>,
    mermaid_renderer: MermaidCliRenderer,
}

#[must_use]
pub fn is_supported_terminal(term_program: Option<&str>, term: Option<&str>) -> bool {
    term_program.is_some_and(|value| value.eq_ignore_ascii_case("ghostty"))
        || term.is_some_and(|value| value.contains("kitty"))
}

impl TerminalViewer {
    #[must_use]
    pub fn new(config: AppConfig, source: FileSystemDocumentSource, document: Document) -> Self {
        let rendered =
            render_document(&document, config.theme, config.mermaid_mode, content_width());
        let last_modified = source.modified_at().ok();

        Self {
            config,
            source,
            document,
            rendered,
            scroll: 0,
            last_modified,
            warning: None,
            needs_redraw: true,
            visible_placements: Vec::new(),
            transmitted_graphics: BTreeSet::new(),
            mermaid_renderer: MermaidCliRenderer::from_env(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide, Clear(ClearType::All))?;

        let run_result = self.event_loop();

        disable_raw_mode()?;
        execute!(
            io::stdout(),
            LeaveAlternateScreen,
            cursor::Show,
            Print(encode_delete(DeleteCommand::AllVisiblePlacements))
        )?;
        run_result
    }

    fn event_loop(&mut self) -> Result<()> {
        loop {
            if self.needs_redraw && !event::poll(Duration::ZERO)? {
                self.draw()?;
                self.needs_redraw = false;
            }
            if self.config.watch && self.maybe_reload() {
                continue;
            }

            if !event::poll(Duration::from_millis(16))? {
                if self.render_next_visible_graphic() {
                    self.needs_redraw = true;
                }
                continue;
            }

            let pending_events = self.read_pending_events()?;
            if self.apply_event_batch(pending_events) {
                return Ok(());
            }
        }
    }

    fn read_pending_events(&mut self) -> Result<Vec<Event>> {
        let mut events = vec![event::read()?];
        while event::poll(Duration::ZERO)? {
            events.push(event::read()?);
        }
        Ok(events)
    }

    fn apply_event_batch<I>(&mut self, events: I) -> bool
    where
        I: IntoIterator<Item = Event>,
    {
        let mut next_scroll = self.scroll;
        let mut scroll_dirty = false;

        let flush_scroll = |viewer: &mut Self, next_scroll: &mut usize, scroll_dirty: &mut bool| {
            if !*scroll_dirty {
                return;
            }
            viewer.needs_redraw |= viewer.set_scroll(*next_scroll);
            *next_scroll = viewer.scroll;
            *scroll_dirty = false;
        };

        for event in events {
            match event {
                Event::Resize(_, _) => {
                    flush_scroll(self, &mut next_scroll, &mut scroll_dirty);
                    self.rerender();
                    let _ = self.set_scroll(cmp::min(self.scroll, self.max_scroll()));
                    next_scroll = self.scroll;
                    self.needs_redraw = true;
                }
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('q') => return true,
                        KeyCode::Char('j') | KeyCode::Down => {
                            next_scroll =
                                cmp::min(next_scroll.saturating_add(1), self.max_scroll());
                            scroll_dirty = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            next_scroll = next_scroll.saturating_sub(1);
                            scroll_dirty = true;
                        }
                        KeyCode::PageDown => {
                            next_scroll = cmp::min(
                                next_scroll.saturating_add(self.page_height()),
                                self.max_scroll(),
                            );
                            scroll_dirty = true;
                        }
                        KeyCode::PageUp => {
                            next_scroll = next_scroll.saturating_sub(self.page_height());
                            scroll_dirty = true;
                        }
                        KeyCode::Char('g') => {
                            next_scroll = 0;
                            scroll_dirty = true;
                        }
                        KeyCode::Char('G') => {
                            next_scroll = self.max_scroll();
                            scroll_dirty = true;
                        }
                        KeyCode::Char('o') => {
                            flush_scroll(self, &mut next_scroll, &mut scroll_dirty);
                            self.open_focused_link();
                            self.needs_redraw = true;
                        }
                        KeyCode::Char('r') => {
                            flush_scroll(self, &mut next_scroll, &mut scroll_dirty);
                            self.reload();
                            next_scroll = self.scroll;
                            self.needs_redraw = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        flush_scroll(self, &mut next_scroll, &mut scroll_dirty);
        false
    }

    fn draw(&mut self) -> Result<()> {
        let mut stdout = io::stdout().lock();
        let (width, height) = terminal::size()?;
        let content_height = usize::from(height.saturating_sub(1));
        let tokens = ThemeTokens::for_theme(self.config.theme);
        let article_width = article_wrap_width(width);
        let article_x = article_origin(width);
        let visible_end = self.rendered.lines.len().min(self.scroll.saturating_add(content_height));

        let visible_lines = &self.rendered.lines[self.scroll..visible_end];
        let (graphic_commands, visible_placement_ids) = collect_graphics_commands(
            &self.rendered,
            GraphicViewport {
                scroll: self.scroll,
                content_height,
                article_x,
                article_width,
                viewport_width: width,
            },
            &self.visible_placements,
            &mut self.transmitted_graphics,
        );

        for row in 0..content_height {
            queue!(stdout, cursor::MoveTo(0, row as u16), Clear(ClearType::CurrentLine))?;
            let Some(line) = visible_lines.get(row) else {
                continue;
            };
            queue!(stdout, cursor::MoveTo(article_x, row as u16))?;
            self.draw_line(&mut stdout, line, usize::from(article_width), tokens)?;
        }

        for command in graphic_commands {
            write!(stdout, "{command}")?;
        }
        self.visible_placements = visible_placement_ids;

        let status = self.status_line(usize::from(width));
        queue!(
            stdout,
            cursor::MoveTo(0, height.saturating_sub(1)),
            SetForegroundColor(tokens.foreground),
            SetBackgroundColor(tokens.status_background),
            Print(status),
            ResetColor
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn page_height(&self) -> usize {
        terminal::size().map(|(_, height)| usize::from(height.saturating_sub(1))).unwrap_or(20)
    }

    fn max_scroll(&self) -> usize {
        self.rendered.lines.len().saturating_sub(self.page_height())
    }

    fn maybe_reload(&mut self) -> bool {
        let Ok(modified) = self.source.modified_at() else {
            return false;
        };

        let should_reload = self.last_modified.is_none_or(|current| modified > current);

        if should_reload {
            self.reload();
            self.last_modified = Some(modified);
            return true;
        }
        false
    }

    fn reload(&mut self) {
        match self.source.read_to_string() {
            Ok(content) => match parse_document(self.source.path().clone(), &content) {
                Ok(document) => {
                    self.document = document;
                    self.rerender();
                    self.warning = None;
                    let _ = self.set_scroll(cmp::min(self.scroll, self.max_scroll()));
                }
                Err(error) => {
                    self.warning = Some(format!("reload failed: {error}"));
                }
            },
            Err(error) => {
                self.warning = Some(format!("reload failed: {error}"));
            }
        }
    }

    fn open_focused_link(&mut self) {
        let Some(url) = self.document.meta.links.first() else {
            self.warning = Some("no link in document".to_string());
            return;
        };

        match browser::open_url(url) {
            Ok(()) => {
                self.warning = Some(format!("opened {url}"));
            }
            Err(error) => {
                self.warning = Some(format!("open failed: {error}"));
            }
        }
    }

    fn rerender(&mut self) {
        let width = content_width();
        self.rendered =
            render_document(&self.document, self.config.theme, self.config.mermaid_mode, width);
        self.transmitted_graphics.clear();
    }

    fn set_scroll(&mut self, next_scroll: usize) -> bool {
        if next_scroll == self.scroll {
            return false;
        }
        self.scroll = next_scroll;
        true
    }

    fn render_next_visible_graphic(&mut self) -> bool {
        let content_height = self.page_height();
        let visible_end = self.scroll.saturating_add(content_height);
        let Some((graphic_index, source)) =
            self.rendered.graphics.iter().enumerate().find_map(|(index, graphic)| {
                if graphic.line_index < self.scroll || graphic.line_index >= visible_end {
                    return None;
                }

                match &graphic.content {
                    RenderedGraphicContent::Mermaid { source, png_bytes: None, failed: false } => {
                        Some((index, source.clone()))
                    }
                    _ => None,
                }
            })
        else {
            return false;
        };

        match self.mermaid_renderer.render_png(&source) {
            Ok(rendered_png) => {
                let image_decoder = ImageDecoder::new();
                let new_height = image_decoder
                    .dimensions_from_png_bytes(&rendered_png)
                    .map(|(width, height)| {
                        scaled_graphic_height(
                            self.rendered.graphics[graphic_index].width_cells,
                            width,
                            height,
                        )
                    })
                    .unwrap_or(self.rendered.graphics[graphic_index].height_cells);

                resize_graphic_space(&mut self.rendered, graphic_index, new_height);
                if let RenderedGraphicContent::Mermaid { png_bytes, failed, .. } =
                    &mut self.rendered.graphics[graphic_index].content
                {
                    *png_bytes = Some(rendered_png);
                    *failed = false;
                }
                self.warning = None;
            }
            Err(error) => {
                if let RenderedGraphicContent::Mermaid { failed, .. } =
                    &mut self.rendered.graphics[graphic_index].content
                {
                    *failed = true;
                }
                self.warning = Some(format!("mermaid render failed: {error}"));
            }
        }

        true
    }

    fn draw_line(
        &self,
        stdout: &mut impl Write,
        line: &RenderedLine,
        width: usize,
        tokens: ThemeTokens,
    ) -> Result<()> {
        match &line.kind {
            RenderedLineKind::Blank => {
                queue!(stdout, ResetColor, Print(" ".repeat(width)))?;
            }
            RenderedLineKind::Heading { level } => {
                let text = fit_to_width(&line.display_text, width);
                queue!(
                    stdout,
                    SetForegroundColor(tokens.accent),
                    SetAttribute(Attribute::Bold),
                    Print(match *level {
                        1 => text.to_uppercase(),
                        _ => text,
                    }),
                    SetAttribute(Attribute::Reset),
                    ResetColor
                )?;
            }
            RenderedLineKind::Paragraph | RenderedLineKind::List => {
                let text = fit_to_width(&line.display_text, width);
                queue!(stdout, SetForegroundColor(tokens.foreground), Print(text), ResetColor)?;
            }
            RenderedLineKind::Quote => {
                let text = fit_to_width(&line.display_text, width);
                queue!(stdout, SetForegroundColor(tokens.muted), Print(text), ResetColor)?;
            }
            RenderedLineKind::Callout { kind } => {
                let text = fit_to_width(&line.display_text, width.saturating_sub(2));
                let foreground = if matches!(
                    *kind,
                    crate::core::document::CalloutKind::Warning
                        | crate::core::document::CalloutKind::Caution
                ) {
                    tokens.warning
                } else {
                    tokens.accent
                };
                queue!(
                    stdout,
                    SetForegroundColor(foreground),
                    Print("▎ "),
                    SetBackgroundColor(tokens.subtle_background),
                    SetForegroundColor(tokens.foreground),
                    SetAttribute(Attribute::Bold),
                    Print(text),
                    SetAttribute(Attribute::Reset),
                    ResetColor
                )?;
            }
            RenderedLineKind::Code { language, is_fence_delimiter } => {
                self.draw_code_line(
                    stdout,
                    line,
                    language.as_deref(),
                    *is_fence_delimiter,
                    width,
                    tokens,
                )?;
            }
            RenderedLineKind::Table => {
                let text = fit_to_width(&line.display_text, width);
                queue!(stdout, SetForegroundColor(tokens.accent), Print(text), ResetColor)?;
            }
            RenderedLineKind::Rule => {
                let text = fit_to_width(&line.display_text, width);
                queue!(stdout, SetForegroundColor(tokens.muted), Print(text), ResetColor)?;
            }
            RenderedLineKind::Meta => {
                let text = fit_to_width(&line.display_text, width);
                queue!(
                    stdout,
                    SetForegroundColor(tokens.muted),
                    SetAttribute(Attribute::Italic),
                    Print(text),
                    SetAttribute(Attribute::Reset),
                    ResetColor
                )?;
            }
        }
        Ok(())
    }

    fn draw_code_line(
        &self,
        stdout: &mut impl Write,
        line: &RenderedLine,
        language: Option<&str>,
        is_fence_delimiter: bool,
        width: usize,
        tokens: ThemeTokens,
    ) -> Result<()> {
        let frame = if is_fence_delimiter {
            if line.display_text.is_empty() {
                format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
            } else {
                let label = format!("─ {} ", line.display_text);
                let fill = "─".repeat(width.saturating_sub(label.chars().count() + 2));
                format!("╭{label}{fill}╮")
            }
        } else {
            let inner_width = width.saturating_sub(4).max(1);
            let raw = truncate_visible(&line.plain_text, inner_width);
            let padding = " ".repeat(inner_width.saturating_sub(raw.chars().count()));
            let highlighted = highlight_code_terminal(&raw, language, self.config.theme);
            format!("│ {highlighted}{padding} │")
        };

        queue!(
            stdout,
            SetForegroundColor(tokens.foreground),
            SetBackgroundColor(tokens.code_background),
            Print(frame),
            ResetColor
        )?;
        Ok(())
    }

    fn status_line(&self, width: usize) -> String {
        let warning = self.warning.clone().unwrap_or_default();
        let status = format!(
            " {} | rev={} | theme={} | watch={} {}",
            self.source.path().display(),
            self.document.revision,
            self.config.theme.as_str(),
            if self.config.watch { "on" } else { "off" },
            warning
        );
        fit_to_width(&status, width)
    }
}

fn fit_to_width(line: &str, width: usize) -> String {
    let mut fitted: String = line.chars().take(width).collect();
    let current = fitted.chars().count();
    if current < width {
        fitted.push_str(&" ".repeat(width - current));
    }
    fitted
}

fn content_width() -> u16 {
    article_wrap_width(terminal::size().map_or(80, |(cols, _)| cols))
}

fn article_origin(viewport_width: u16) -> u16 {
    viewport_width.saturating_sub(article_wrap_width(viewport_width)) / 2
}

fn article_wrap_width(viewport_width: u16) -> u16 {
    let usable = viewport_width.saturating_sub(8);
    if usable >= 24 { usable.min(96) } else { viewport_width.saturating_sub(2).max(1) }
}

fn truncate_visible(text: &str, width: usize) -> String {
    text.chars().take(width.max(1)).collect()
}

fn highlight_code_terminal(text: &str, language: Option<&str>, theme: crate::cli::Theme) -> String {
    if text.is_empty() {
        return String::new();
    }

    let syntax_set = syntax_set();
    let syntax = syntax_for_token(syntax_set, language);
    let mut highlighter = HighlightLines::new(syntax, syntect_theme(theme));
    highlighter
        .highlight_line(text, syntax_set)
        .map(|segments| as_24_bit_terminal_escaped(&segments[..], false))
        .unwrap_or_else(|_| text.to_string())
}

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syntax_for_token<'a>(syntax_set: &'a SyntaxSet, language: Option<&str>) -> &'a SyntaxReference {
    language
        .and_then(|token| syntax_set.find_syntax_by_token(token))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
}

fn syntect_theme(theme: crate::cli::Theme) -> &'static SyntectTheme {
    let themes = THEME_SET.get_or_init(ThemeSet::load_defaults);
    let preferred = match theme {
        crate::cli::Theme::Light => "InspiredGitHub",
        crate::cli::Theme::Dark => "base16-ocean.dark",
    };

    themes
        .themes
        .get(preferred)
        .or_else(|| themes.themes.values().next())
        .unwrap_or_else(|| FALLBACK_THEME.get_or_init(SyntectTheme::default))
}

fn visible_graphic_placements(
    rendered: &RenderedDocument,
    scroll: usize,
    content_height: usize,
) -> Vec<(u32, u32)> {
    rendered
        .graphics
        .iter()
        .enumerate()
        .filter(|(_, graphic)| {
            graphic.line_index >= scroll && graphic.line_index - scroll < content_height
        })
        .map(|(index, _)| {
            let id = (index + 2) as u32;
            (id, id)
        })
        .collect()
}

fn collect_graphics_commands(
    rendered: &RenderedDocument,
    viewport: GraphicViewport,
    previous_visible_placements: &[(u32, u32)],
    transmitted_graphics: &mut BTreeSet<u32>,
) -> (Vec<String>, Vec<(u32, u32)>) {
    let mut commands = Vec::new();
    let visible_placements =
        visible_graphic_placements(rendered, viewport.scroll, viewport.content_height);

    for (image_id, placement_id) in previous_visible_placements
        .iter()
        .copied()
        .filter(|placement| !visible_placements.contains(placement))
    {
        commands.push(encode_delete(DeleteCommand::Placement { image_id, placement_id }));
    }

    for (index, graphic) in rendered.graphics.iter().enumerate() {
        if graphic.line_index < viewport.scroll {
            continue;
        }
        let relative_row = graphic.line_index - viewport.scroll;
        if relative_row >= viewport.content_height {
            continue;
        }

        let Some(png_bytes) = graphic_png_bytes(graphic) else {
            continue;
        };

        let image_id = (index + 2) as u32;
        if transmitted_graphics.insert(image_id) {
            commands.push(encode_transmit_png(image_id, png_bytes));
        }

        let placement = KittyImagePlacement {
            image_id,
            placement_id: image_id,
            columns: graphic.width_cells.min(viewport.viewport_width),
            rows: graphic.height_cells,
            cursor_x: 0,
            cursor_y: 0,
            z_index: -1,
        };
        let graphic_x =
            viewport.article_x + viewport.article_width.saturating_sub(graphic.width_cells) / 2;
        commands.push(format!(
            "{}{}",
            ansi_cursor_move(graphic_x, relative_row as u16),
            encode_place(&placement)
        ));
    }

    (commands, visible_placements)
}

fn graphic_png_bytes(graphic: &RenderedGraphic) -> Option<&[u8]> {
    match &graphic.content {
        RenderedGraphicContent::Png(png_bytes) => Some(png_bytes),
        RenderedGraphicContent::Mermaid { png_bytes: Some(png_bytes), .. } => Some(png_bytes),
        RenderedGraphicContent::Mermaid { png_bytes: None, .. } => None,
    }
}

fn ansi_cursor_move(x: u16, y: u16) -> String {
    format!("\u{1b}[{};{}H", y + 1, x + 1)
}

fn resize_graphic_space(rendered: &mut RenderedDocument, graphic_index: usize, new_height: u16) {
    let line_index = rendered.graphics[graphic_index].line_index;
    let current_height = rendered.graphics[graphic_index].height_cells;
    if new_height == current_height {
        return;
    }

    if new_height > current_height {
        let extra = usize::from(new_height - current_height);
        rendered.lines.splice(
            line_index + usize::from(current_height)..line_index + usize::from(current_height),
            std::iter::repeat_n(
                RenderedLine {
                    plain_text: String::new(),
                    display_text: String::new(),
                    kind: RenderedLineKind::Blank,
                },
                extra,
            ),
        );
    } else {
        let remove_start = line_index + usize::from(new_height);
        let remove_end = line_index + usize::from(current_height);
        rendered.lines.drain(remove_start..remove_end);
    }

    let delta = new_height as isize - current_height as isize;
    rendered.graphics[graphic_index].height_cells = new_height;
    for graphic in rendered.graphics.iter_mut().skip(graphic_index + 1) {
        graphic.line_index = graphic.line_index.saturating_add_signed(delta);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GraphicViewport, article_wrap_width, collect_graphics_commands, visible_graphic_placements,
    };
    use std::fs;

    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use tempfile::NamedTempFile;

    use crate::{
        cli::Theme,
        core::config::{AppConfig, MermaidMode},
        io::fs::FileSystemDocumentSource,
        render::text::{RenderedDocument, RenderedGraphic},
        ui::terminal::TerminalViewer,
    };

    #[test]
    fn article_width_is_capped_for_readability() {
        assert_eq!(article_wrap_width(20), 18);
        assert_eq!(article_wrap_width(80), 72);
        assert_eq!(article_wrap_width(140), 96);
    }

    #[test]
    fn visible_graphic_placements_follow_scroll_window() {
        let rendered = RenderedDocument {
            lines: Vec::new(),
            graphics: vec![
                RenderedGraphic {
                    line_index: 1,
                    width_cells: 10,
                    height_cells: 4,
                    content: crate::render::text::RenderedGraphicContent::Png(vec![1]),
                },
                RenderedGraphic {
                    line_index: 8,
                    width_cells: 10,
                    height_cells: 4,
                    content: crate::render::text::RenderedGraphicContent::Png(vec![2]),
                },
            ],
        };

        assert_eq!(visible_graphic_placements(&rendered, 0, 5), vec![(2, 2)]);
        assert_eq!(visible_graphic_placements(&rendered, 5, 5), vec![(3, 3)]);
    }

    #[test]
    fn graphics_commands_only_transmit_png_payload_once() {
        let rendered = RenderedDocument {
            lines: Vec::new(),
            graphics: vec![RenderedGraphic {
                line_index: 1,
                width_cells: 10,
                height_cells: 4,
                content: crate::render::text::RenderedGraphicContent::Png(vec![1, 2, 3]),
            }],
        };
        let mut transmitted = std::collections::BTreeSet::new();

        let viewport = GraphicViewport {
            scroll: 0,
            content_height: 5,
            article_x: 0,
            article_width: 20,
            viewport_width: 80,
        };
        let (first_commands, _) =
            collect_graphics_commands(&rendered, viewport, &[], &mut transmitted);
        let (second_commands, _) =
            collect_graphics_commands(&rendered, viewport, &[], &mut transmitted);

        assert_eq!(first_commands.iter().filter(|command| command.contains("a=t")).count(), 1);
        assert_eq!(second_commands.iter().filter(|command| command.contains("a=t")).count(), 0);
        assert!(second_commands.iter().any(|command| command.contains("a=p")));
    }

    #[test]
    fn batched_scroll_events_apply_accumulated_offset() {
        let mut viewer = sample_viewer(&long_document(64));
        viewer.needs_redraw = false;

        let should_quit =
            viewer.apply_event_batch(vec![key_event('j'), key_event('j'), key_event('j')]);

        assert!(!should_quit);
        assert_eq!(viewer.scroll, 3);
        assert!(viewer.needs_redraw);
    }

    #[test]
    fn quit_in_batched_input_short_circuits_pending_scroll() {
        let mut viewer = sample_viewer(&long_document(64));
        viewer.needs_redraw = false;

        let should_quit =
            viewer.apply_event_batch(vec![key_event('j'), key_event('j'), key_event('q')]);

        assert!(should_quit);
        assert_eq!(viewer.scroll, 0);
        assert!(!viewer.needs_redraw);
    }

    fn sample_viewer(source: &str) -> TerminalViewer {
        let file = match NamedTempFile::new() {
            Ok(file) => file,
            Err(error) => panic!("temp markdown should be created: {error}"),
        };
        if let Err(error) = fs::write(file.path(), source) {
            panic!("temp markdown should be written: {error}");
        }

        let path = file.into_temp_path().keep().unwrap_or_else(|error| {
            panic!("temp markdown should persist for the test viewer: {error}")
        });
        let document = crate::render::markdown::parse_document(path.clone(), source)
            .unwrap_or_else(|error| panic!("document should parse: {error}"));

        TerminalViewer::new(
            AppConfig {
                path: path.clone(),
                watch: false,
                theme: Theme::Light,
                mermaid_mode: MermaidMode::Disabled,
            },
            FileSystemDocumentSource::new(path),
            document,
        )
    }

    fn long_document(paragraphs: usize) -> String {
        (0..paragraphs)
            .map(|index| format!("Paragraph {index}\n\nThis is line {index}."))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn key_event(ch: char) -> Event {
        let mut key = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
        key.kind = KeyEventKind::Press;
        Event::Key(key)
    }
}

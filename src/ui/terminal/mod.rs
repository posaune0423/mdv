use std::{
    cmp,
    collections::BTreeSet,
    io::{self, Write},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
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
mod graphics;
mod highlight;
mod layout;
#[cfg(test)]
mod tests;

use self::{
    graphics::{collect_graphics_commands, collect_page_viewport_commands, resize_graphic_space},
    highlight::highlight_code_terminal,
    layout::{
        article_origin, article_wrap_width, current_cell_metrics, fit_to_width,
        render_graphic_page, truncate_visible,
    },
};
#[cfg(test)]
use crate::render::text::render_document;
use crate::{
    core::{config::AppConfig, document::Document, theme::ThemeTokens},
    io::fs::FileSystemDocumentSource,
    io::kitty_graphics::{DeleteCommand, encode_delete},
    io::{browser, image_decoder::ImageDecoder, mermaid_cli::MermaidCliRenderer},
    render::{
        markdown::parse_document,
        text::{
            RenderedDocument, RenderedGraphicContent, RenderedLine, RenderedLineKind,
            scaled_graphic_height,
        },
    },
    ui::page_graphics::{GraphicPage, total_rows},
};

#[derive(Clone, Copy)]
struct GraphicViewport {
    scroll: usize,
    content_height: usize,
    article_x: u16,
    article_width: u16,
    viewport_width: u16,
    cell_metrics: CellMetrics,
}

#[derive(Clone, Copy)]
struct CellMetrics {
    width_px: f32,
    height_px: f32,
}

pub struct TerminalViewer {
    config: AppConfig,
    source: FileSystemDocumentSource,
    document: Document,
    source_text: String,
    rendered: Option<RenderedDocument>,
    graphic_page: Option<GraphicPage>,
    scroll: usize,
    last_modified: Option<SystemTime>,
    warning: Option<String>,
    needs_redraw: bool,
    pending_layout_refresh: bool,
    clear_all_graphics: bool,
    visible_placements: Vec<(u32, u32)>,
    transmitted_graphics: BTreeSet<u32>,
    mermaid_renderer: MermaidCliRenderer,
    cell_metrics: CellMetrics,
}

#[must_use]
pub fn is_supported_terminal(term_program: Option<&str>, term: Option<&str>) -> bool {
    term_program.is_some_and(|value| value.eq_ignore_ascii_case("ghostty"))
        || term.is_some_and(|value| value.contains("kitty"))
}

impl TerminalViewer {
    pub fn try_new(
        config: AppConfig,
        source: FileSystemDocumentSource,
        document: Document,
        source_text: String,
    ) -> Result<Self> {
        let cell_metrics = current_cell_metrics();
        let last_modified = source.modified_at().ok();
        let graphic_page = render_graphic_page(&config, &document, &source_text, cell_metrics)
            .with_context(|| {
                format!("interactive graphic render failed for {}", source.path().display())
            })?;

        Ok(Self {
            config,
            source,
            document,
            source_text,
            rendered: None,
            graphic_page: Some(graphic_page),
            scroll: 0,
            last_modified,
            warning: None,
            needs_redraw: true,
            pending_layout_refresh: false,
            clear_all_graphics: false,
            visible_placements: Vec::new(),
            transmitted_graphics: BTreeSet::new(),
            mermaid_renderer: MermaidCliRenderer::from_env(),
            cell_metrics,
        })
    }

    #[cfg(test)]
    #[must_use]
    pub fn new_for_tests(
        config: AppConfig,
        source: FileSystemDocumentSource,
        document: Document,
        source_text: String,
    ) -> Self {
        let cell_metrics = current_cell_metrics();
        let last_modified = source.modified_at().ok();
        let (graphic_page, rendered, warning) =
            match render_graphic_page(&config, &document, &source_text, cell_metrics) {
                Ok(page) => (Some(page), None, None),
                Err(error) => (
                    None,
                    Some(render_document(
                        &document,
                        config.theme,
                        config.mermaid_mode,
                        article_wrap_width(terminal::size().map_or(80, |(cols, _)| cols)),
                        cell_metrics.width_px / cell_metrics.height_px,
                    )),
                    Some(format!("graphic mode unavailable: {error}")),
                ),
            };

        Self {
            config,
            source,
            document,
            source_text,
            rendered,
            graphic_page,
            scroll: 0,
            last_modified,
            warning,
            needs_redraw: true,
            pending_layout_refresh: false,
            clear_all_graphics: false,
            visible_placements: Vec::new(),
            transmitted_graphics: BTreeSet::new(),
            mermaid_renderer: MermaidCliRenderer::from_env(),
            cell_metrics,
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
                    self.cell_metrics = current_cell_metrics();
                    self.pending_layout_refresh = true;
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
        if self.pending_layout_refresh {
            self.rerender();
            let _ = self.set_scroll(cmp::min(self.scroll, self.max_scroll()));
            self.pending_layout_refresh = false;
        }

        let mut stdout = io::stdout().lock();
        let (width, height) = terminal::size()?;
        let content_height = usize::from(height.saturating_sub(1));
        let tokens = ThemeTokens::for_theme(self.config.theme);
        let article_width = article_wrap_width(width);
        let article_x = article_origin(width);

        for row in 0..content_height {
            queue!(stdout, cursor::MoveTo(0, row as u16), Clear(ClearType::CurrentLine))?;
            if let Some(rendered) = &self.rendered {
                let visible_end =
                    rendered.lines.len().min(self.scroll.saturating_add(content_height));
                let visible_lines = &rendered.lines[self.scroll..visible_end];
                let Some(line) = visible_lines.get(row) else {
                    continue;
                };
                queue!(stdout, cursor::MoveTo(article_x, row as u16))?;
                self.draw_line(&mut stdout, line, usize::from(article_width), tokens)?;
            }
        }

        let (graphic_commands, visible_placement_ids) = if let Some(page) = &self.graphic_page {
            collect_page_viewport_commands(
                page,
                self.scroll,
                content_height,
                self.cell_metrics,
                &self.visible_placements,
                &mut self.transmitted_graphics,
            )?
        } else if let Some(rendered) = &self.rendered {
            collect_graphics_commands(
                rendered,
                GraphicViewport {
                    scroll: self.scroll,
                    content_height,
                    article_x,
                    article_width,
                    viewport_width: width,
                    cell_metrics: self.cell_metrics,
                },
                &self.visible_placements,
                &mut self.transmitted_graphics,
            )
        } else {
            (Vec::new(), Vec::new())
        };

        if self.clear_all_graphics {
            write!(stdout, "{}", encode_delete(DeleteCommand::AllVisiblePlacements))?;
            self.clear_all_graphics = false;
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
        if let Some(page) = &self.graphic_page {
            total_rows(page, self.cell_metrics.height_px).saturating_sub(self.page_height())
        } else if let Some(rendered) = &self.rendered {
            rendered.lines.len().saturating_sub(self.page_height())
        } else {
            0
        }
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
                    self.source_text = content;
                    self.rerender();
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
        match render_graphic_page(
            &self.config,
            &self.document,
            &self.source_text,
            self.cell_metrics,
        ) {
            Ok(graphic_page) => {
                self.graphic_page = Some(graphic_page);
                self.warning = None;
            }
            Err(error) => {
                self.warning = Some(format!("graphic render failed: {error}"));
            }
        }
        self.transmitted_graphics.clear();
        self.visible_placements.clear();
        self.clear_all_graphics = true;
    }

    fn set_scroll(&mut self, next_scroll: usize) -> bool {
        if next_scroll == self.scroll {
            return false;
        }
        self.scroll = next_scroll;
        true
    }

    fn render_next_visible_graphic(&mut self) -> bool {
        if self.graphic_page.is_some() {
            return false;
        }

        let content_height = self.page_height();
        let Some(rendered) = self.rendered.as_mut() else {
            return false;
        };
        let visible_end = self.scroll.saturating_add(content_height);
        let Some((graphic_index, source)) =
            rendered.graphics.iter().enumerate().find_map(|(index, graphic)| {
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

        let target_width_px = (f32::from(rendered.graphics[graphic_index].width_cells)
            * self.cell_metrics.width_px)
            .round()
            .max(1.0) as u32;

        match self.mermaid_renderer.render_png_sized(
            &source,
            Some(target_width_px),
            Some(2.0),
            self.config.theme,
        ) {
            Ok(rendered_png) => {
                let image_decoder = ImageDecoder::new();
                let natural_size = image_decoder.dimensions_from_png_bytes(&rendered_png).ok();
                let new_height = natural_size
                    .map(|(width, height)| {
                        scaled_graphic_height(
                            rendered.graphics[graphic_index].width_cells,
                            self.cell_metrics.width_px / self.cell_metrics.height_px,
                            width,
                            height,
                        )
                    })
                    .unwrap_or(rendered.graphics[graphic_index].height_cells);

                resize_graphic_space(rendered, graphic_index, new_height);
                if let RenderedGraphicContent::Mermaid { png_bytes, failed, .. } =
                    &mut rendered.graphics[graphic_index].content
                {
                    *png_bytes = Some(rendered_png);
                    *failed = false;
                }
                if let Some((width, height)) = natural_size {
                    rendered.graphics[graphic_index].natural_width_px = width;
                    rendered.graphics[graphic_index].natural_height_px = height;
                }
                self.warning = None;
            }
            Err(error) => {
                if let RenderedGraphicContent::Mermaid { failed, .. } =
                    &mut rendered.graphics[graphic_index].content
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

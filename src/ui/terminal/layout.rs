use std::path::Path;

use anyhow::Result;
use crossterm::terminal;
use tracing::info_span;

use crate::{
    core::{config::AppConfig, document::Document},
    io::webkit_snapshot::render_html_to_png,
    render::github_html::build_github_html,
    ui::{
        page_graphics::{GraphicPage, build_graphic_page},
        terminal::CellMetrics,
    },
};

const GRAPHIC_PAGE_ZOOM: f32 = 1.55;

pub(super) fn render_graphic_page(
    config: &AppConfig,
    document: &Document,
    source_text: &str,
    cell_metrics: CellMetrics,
) -> Result<GraphicPage> {
    let _span = info_span!("startup.render_graphic_page").entered();
    if cfg!(test) {
        anyhow::bail!("graphic mode disabled during tests");
    }

    let viewport_width = terminal::size().map_or(80, |(width, _)| width).max(1);
    let display_width_px =
        (f32::from(viewport_width) * cell_metrics.width_px).round().max(1.0) as u32;
    let snapshot_width_px = ((display_width_px as f32) / GRAPHIC_PAGE_ZOOM).round().max(1.0) as u32;
    let base_dir = document
        .path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let html = {
        let _span = info_span!("startup.build_github_html").entered();
        build_github_html(source_text, base_dir, config.theme, config.mermaid_mode)?
    };
    let snapshot = {
        let _span = info_span!("startup.render_html_to_png").entered();
        render_html_to_png(&html, base_dir, snapshot_width_px)?
    };

    {
        let _span = info_span!("startup.build_graphic_page").entered();
        build_graphic_page(&snapshot.png_bytes, viewport_width, display_width_px)
    }
}

pub(super) fn fit_to_width(line: &str, width: usize) -> String {
    let mut fitted: String = line.chars().take(width).collect();
    let current = fitted.chars().count();
    if current < width {
        fitted.push_str(&" ".repeat(width - current));
    }
    fitted
}

pub(super) fn current_cell_metrics() -> CellMetrics {
    let Ok(window) = terminal::window_size() else {
        return CellMetrics { width_px: 8.0, height_px: 16.0 };
    };
    if window.columns == 0 || window.rows == 0 || window.width == 0 || window.height == 0 {
        return CellMetrics { width_px: 8.0, height_px: 16.0 };
    }

    let cell_width = f32::from(window.width) / f32::from(window.columns);
    let cell_height = f32::from(window.height) / f32::from(window.rows);
    if cell_width <= 0.0 || cell_height <= 0.0 {
        CellMetrics { width_px: 8.0, height_px: 16.0 }
    } else {
        CellMetrics { width_px: cell_width, height_px: cell_height }
    }
}

pub(super) fn article_origin(viewport_width: u16) -> u16 {
    viewport_width.saturating_sub(article_wrap_width(viewport_width)) / 2
}

pub(super) fn article_wrap_width(viewport_width: u16) -> u16 {
    let usable = viewport_width.saturating_sub(8);
    if usable >= 24 { usable.min(96) } else { viewport_width.saturating_sub(2).max(1) }
}

pub(super) fn truncate_visible(text: &str, width: usize) -> String {
    text.chars().take(width.max(1)).collect()
}

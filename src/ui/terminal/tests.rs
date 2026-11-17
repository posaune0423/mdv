use std::fs;
use std::io::Cursor;
use std::sync::mpsc;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use tempfile::NamedTempFile;

use crate::{
    cli::Theme,
    core::config::{AppConfig, MermaidMode},
    io::fs::FileSystemDocumentSource,
    render::text::{
        RenderedDocument, RenderedGraphic, RenderedGraphicContent, RenderedLine, RenderedLineKind,
    },
    ui::page_graphics::build_graphic_page,
};

use super::{
    CellMetrics, GraphicViewport, TerminalViewer,
    graphics::{
        PageGraphics, collect_graphics_commands, fit_graphic_placement, visible_graphic_placements,
    },
    layout::article_wrap_width,
};

#[test]
fn article_width_is_capped_for_readability() {
    assert_eq!(article_wrap_width(120), 96);
    assert_eq!(article_wrap_width(20), 18);
}

#[test]
fn visible_graphic_placements_follow_scroll_window() {
    let rendered = RenderedDocument {
        lines: vec![],
        graphics: vec![
            RenderedGraphic {
                line_index: 1,
                width_cells: 10,
                height_cells: 4,
                natural_width_px: 100,
                natural_height_px: 50,
                content: RenderedGraphicContent::Png(vec![1]),
            },
            RenderedGraphic {
                line_index: 8,
                width_cells: 10,
                height_cells: 4,
                natural_width_px: 100,
                natural_height_px: 50,
                content: RenderedGraphicContent::Png(vec![2]),
            },
        ],
    };

    assert_eq!(visible_graphic_placements(&rendered, 0, 5), vec![(2, 2)]);
    assert_eq!(visible_graphic_placements(&rendered, 5, 5), vec![(3, 3)]);
}

#[test]
fn graphics_commands_only_transmit_png_payload_once() {
    let rendered = RenderedDocument {
        lines: vec![RenderedLine {
            plain_text: String::new(),
            display_text: String::new(),
            spans: Vec::new(),
            kind: RenderedLineKind::Blank,
        }],
        graphics: vec![RenderedGraphic {
            line_index: 1,
            width_cells: 10,
            height_cells: 4,
            natural_width_px: 100,
            natural_height_px: 50,
            content: RenderedGraphicContent::Png(vec![1, 2, 3]),
        }],
    };
    let mut transmitted = std::collections::BTreeSet::new();
    let viewport = GraphicViewport {
        scroll: 0,
        content_height: 10,
        article_x: 0,
        article_width: 20,
        viewport_width: 80,
        cell_metrics: CellMetrics { width_px: 8.0, height_px: 16.0 },
    };
    let (first_commands, _) = collect_graphics_commands(&rendered, viewport, &[], &mut transmitted);
    let (second_commands, _) =
        collect_graphics_commands(&rendered, viewport, &[], &mut transmitted);

    assert_eq!(first_commands.iter().filter(|command| command.contains("a=t")).count(), 1);
    assert_eq!(second_commands.iter().filter(|command| command.contains("a=t")).count(), 0);
    assert!(second_commands.iter().any(|command| command.contains("a=p")));
}

#[test]
fn page_scroll_reuses_resident_image_and_changes_source_crop() {
    let image = ImageBuffer::from_pixel(8, 40, Rgba([255, 0, 0, 255]));
    let mut png_bytes = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture image should encode: {error}"));
    let page = build_graphic_page(&png_bytes, 16, 8)
        .unwrap_or_else(|error| panic!("graphic page should decode: {error}"));
    let mut graphics = PageGraphics::default();

    let (first_commands, first_placements) = graphics
        .commands(&page, 0.0, 4, CellMetrics { width_px: 8.0, height_px: 2.0 }, &[])
        .unwrap_or_else(|error| panic!("viewport commands should build: {error}"));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while graphics.has_pending_work() {
        assert!(std::time::Instant::now() < deadline, "resident preparation did not finish");
        graphics
            .maintenance()
            .unwrap_or_else(|error| panic!("resident preparation should upload: {error}"));
        std::thread::yield_now();
    }
    let (second_commands, _) = graphics
        .commands(&page, 1.0, 4, CellMetrics { width_px: 8.0, height_px: 2.0 }, &first_placements)
        .unwrap_or_else(|error| panic!("viewport commands should rebuild after scroll: {error}"));

    assert_eq!(first_commands.iter().filter(|command| command.contains("a=t")).count(), 1);
    assert_eq!(second_commands.iter().filter(|command| command.contains("a=t")).count(), 0);
    assert!(first_commands.iter().any(|command| command.contains("h=8")));
    assert!(second_commands.iter().any(|command| command.contains("h=8")));
    assert!(second_commands.iter().any(|command| command.contains("x=0,y=2")));
}

#[test]
fn fit_graphic_placement_stays_within_reserved_box() {
    let graphic = RenderedGraphic {
        line_index: 0,
        width_cells: 20,
        height_cells: 6,
        natural_width_px: 1200,
        natural_height_px: 200,
        content: RenderedGraphicContent::Png(vec![1]),
    };

    let (columns, rows, _, _) =
        fit_graphic_placement(&graphic, CellMetrics { width_px: 8.0, height_px: 16.0 });

    assert!(columns <= graphic.width_cells);
    assert!(rows <= graphic.height_cells);
    assert!(columns > 0);
    assert!(rows > 0);
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
fn wheel_scroll_moves_the_document_and_coalesces_direction_changes() {
    let mut viewer = sample_viewer(&long_document(64));
    viewer.needs_redraw = false;
    let wheel = |kind| {
        Event::Mouse(MouseEvent { kind, column: 10, row: 10, modifiers: KeyModifiers::NONE })
    };
    assert!(!viewer.apply_event_batch([
        wheel(MouseEventKind::ScrollDown),
        wheel(MouseEventKind::ScrollDown),
        wheel(MouseEventKind::ScrollUp)
    ]));
    assert_eq!(viewer.scroll, 3);
    assert!(viewer.needs_redraw);
}

#[test]
fn held_navigation_keys_continue_scrolling_but_releases_do_not() {
    let mut viewer = sample_viewer(&long_document(64));
    let mut repeat = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    repeat.kind = KeyEventKind::Repeat;
    let mut release = repeat;
    release.kind = KeyEventKind::Release;
    assert!(!viewer.apply_event_batch([
        Event::Key(repeat),
        Event::Key(repeat),
        Event::Key(release)
    ]));
    assert_eq!(viewer.scroll, 2);
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

#[test]
fn quit_short_circuits_reload_and_releases_queued_image_deletes() {
    let mut viewer = sample_viewer(&long_document(64));
    let warning = viewer.warning.clone();
    fs::remove_file(viewer.source.path())
        .unwrap_or_else(|error| panic!("remove source for reload check: {error}"));
    viewer.pending_image_deletes.push(crate::io::kitty_graphics::encode_delete(
        crate::io::kitty_graphics::DeleteCommand::Image { image_id: 1 },
    ));
    assert!(viewer.apply_event_batch([key_event('r'), key_event('q')]));
    assert_eq!(viewer.warning, warning, "quit should bypass the queued reload");
    let deletes = viewer.release_page_images();
    assert!(deletes.iter().any(|command| command.contains("a=d,d=I,i=1")));
    assert!(viewer.release_page_images().is_empty());
}

#[test]
fn resize_events_defer_layout_refresh_until_draw() {
    let mut viewer = sample_viewer(&long_document(8));
    viewer.needs_redraw = false;

    let should_quit = viewer.apply_event_batch(vec![Event::Resize(120, 40)]);

    assert!(!should_quit);
    assert!(viewer.pending_layout_refresh);
    assert!(viewer.needs_redraw);
}

#[test]
fn completed_pending_graphic_page_replaces_the_initial_page_and_schedules_redraw() {
    let mut viewer = sample_viewer(&long_document(64));
    let initial = graphic_page_with_height(40);
    let completed = graphic_page_with_height(80);
    let (sender, receiver) = mpsc::channel();
    viewer.graphic_page = Some(initial);
    viewer.pending_graphic_page = Some(super::layout::PendingGraphicPage::new(receiver));
    viewer.needs_redraw = false;
    viewer.visible_placements = vec![(7, 7)];
    viewer.transmitted_graphics.insert(7);
    viewer.scroll = usize::MAX;
    sender
        .send(Ok(completed.clone()))
        .unwrap_or_else(|error| panic!("send completed page: {error}"));

    viewer.poll_pending_graphic_page();

    assert_eq!(viewer.graphic_page, Some(completed));
    assert!(viewer.pending_graphic_page.is_none());
    assert!(viewer.needs_redraw);
    assert!(viewer.clear_all_graphics);
    assert!(viewer.visible_placements.is_empty());
    assert!(viewer.transmitted_graphics.is_empty());
    assert_eq!(viewer.scroll, viewer.max_scroll());
}

#[test]
fn source_refresh_and_resize_discard_stale_pending_graphic_pages() -> anyhow::Result<()> {
    let mut viewer = sample_viewer("# Before\n");
    let (source_sender, source_receiver) = mpsc::channel();
    viewer.pending_graphic_page = Some(super::layout::PendingGraphicPage::new(source_receiver));
    fs::write(viewer.source.path(), "# After\n")?;

    assert!(viewer.maybe_reload());
    assert!(viewer.pending_graphic_page.is_none());
    assert!(source_sender.send(Ok(graphic_page_with_height(80))).is_err());
    viewer.poll_pending_graphic_page();
    assert!(viewer.graphic_page.is_none(), "stale source result must not replace the new render");

    let (resize_sender, resize_receiver) = mpsc::channel();
    viewer.pending_graphic_page = Some(super::layout::PendingGraphicPage::new(resize_receiver));
    assert!(!viewer.apply_event_batch([Event::Resize(120, 40)]));
    assert!(viewer.pending_graphic_page.is_none());
    assert!(resize_sender.send(Ok(graphic_page_with_height(80))).is_err());
    viewer.poll_pending_graphic_page();
    assert!(viewer.graphic_page.is_none(), "stale resize result must not replace the current page");
    Ok(())
}

#[test]
fn dropping_a_pending_page_cancels_its_background_render() {
    let (_, receiver) = mpsc::channel();
    let pending = super::layout::PendingGraphicPage::new(receiver);
    let cancelled = pending.cancelled.clone();
    assert!(!cancelled.load(std::sync::atomic::Ordering::Relaxed));
    drop(pending);
    assert!(cancelled.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn try_new_degrades_gracefully_when_graphic_mode_unavailable() {
    let file = NamedTempFile::new()
        .unwrap_or_else(|error| panic!("temp markdown should be created: {error}"));
    let source = "# Title\n\nParagraph.\n";
    if let Err(error) = fs::write(file.path(), source) {
        panic!("temp markdown should be written: {error}");
    }

    let path = file.into_temp_path().keep().unwrap_or_else(|error| {
        panic!("temp markdown should persist for the test viewer: {error}")
    });
    let document = crate::render::markdown::parse_document(path.clone(), source)
        .unwrap_or_else(|error| panic!("document should parse: {error}"));

    let viewer = TerminalViewer::try_new(
        AppConfig {
            path: path.clone(),
            watch: false,
            theme: Theme::Light,
            mermaid_mode: MermaidMode::Disabled,
        },
        FileSystemDocumentSource::new(path),
        document,
        source.to_string(),
    )
    .unwrap_or_else(|error| panic!("viewer should degrade gracefully, not fail: {error}"));

    assert!(viewer.graphic_page.is_none(), "graphic page should be None in test mode");
    assert!(viewer.rendered.is_some(), "text-mode fallback should be populated");
    assert!(viewer.warning.is_some(), "warning should be set when graphic mode is unavailable");
}

fn sample_viewer(source: &str) -> TerminalViewer {
    let file = NamedTempFile::new()
        .unwrap_or_else(|error| panic!("temp markdown should be created: {error}"));
    if let Err(error) = fs::write(file.path(), source) {
        panic!("temp markdown should be written: {error}");
    }

    let path = file.into_temp_path().keep().unwrap_or_else(|error| {
        panic!("temp markdown should persist for the test viewer: {error}")
    });
    let document = crate::render::markdown::parse_document(path.clone(), source)
        .unwrap_or_else(|error| panic!("document should parse: {error}"));

    TerminalViewer::new_for_tests(
        AppConfig {
            path: path.clone(),
            watch: false,
            theme: Theme::Light,
            mermaid_mode: MermaidMode::Disabled,
        },
        FileSystemDocumentSource::new(path),
        document,
        source.to_string(),
    )
}

fn graphic_page_with_height(height: u32) -> crate::ui::page_graphics::GraphicPage {
    let image = ImageBuffer::from_pixel(8, height, Rgba([0, 0, 0, 255]));
    crate::ui::page_graphics::GraphicPage {
        width_cells: 1,
        display_width_px: 8,
        display_height_px: height,
        image_width_px: 8,
        image_height_px: height,
        image: std::sync::Arc::new(image),
    }
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

#[test]
fn file_refresh_schedules_a_redraw_without_input() -> anyhow::Result<()> {
    let mut viewer = sample_viewer("# Before\n");
    viewer.needs_redraw = false;
    let before = viewer.source.modified_at()?;
    fs::write(viewer.source.path(), "# After\n")?;
    fs::File::options()
        .write(true)
        .open(viewer.source.path())?
        .set_modified(before + std::time::Duration::from_secs(1))?;

    assert!(viewer.maybe_reload());
    assert_eq!(viewer.source_text, "# After\n");
    assert!(viewer.needs_redraw, "a file change must be visible without keyboard input");
    Ok(())
}

#[test]
fn file_refresh_detects_equal_and_backward_timestamps() -> anyhow::Result<()> {
    let mut viewer = sample_viewer("# First\n");
    let original = viewer.source.modified_at()?;
    for (content, modified) in
        [("# Other\n", original), ("# Third\n", original - std::time::Duration::from_secs(1))]
    {
        fs::write(viewer.source.path(), content)?;
        fs::File::options().write(true).open(viewer.source.path())?.set_modified(modified)?;
        assert!(viewer.maybe_reload(), "content changes must not depend on increasing mtime");
        assert_eq!(viewer.source_text, content);
    }
    viewer.needs_redraw = false;
    assert!(!viewer.maybe_reload());
    assert!(!viewer.needs_redraw, "unchanged content should not repaint");
    Ok(())
}

#[test]
fn file_refresh_recovers_after_atomic_save_and_clamps_scroll() -> anyhow::Result<()> {
    let mut viewer = sample_viewer(&long_document(50));
    viewer.scroll = viewer.max_scroll();
    let original = viewer.source.modified_at()?;
    let path = viewer.source.path().clone();
    fs::remove_file(&path)?;
    assert!(!viewer.maybe_reload());
    fs::write(&path, "# Replacement\n")?;
    fs::File::options().write(true).open(&path)?.set_modified(original)?;
    assert!(viewer.maybe_reload());
    assert_eq!(viewer.source_text, "# Replacement\n");
    assert_eq!(viewer.scroll, viewer.max_scroll(), "reload must clamp to the current viewport");
    Ok(())
}

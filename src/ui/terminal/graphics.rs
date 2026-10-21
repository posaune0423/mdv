use std::collections::BTreeSet;

use crate::{
    io::kitty_graphics::{
        DeleteCommand, KittyImagePlacement, encode_delete, encode_place, encode_transmit_png,
    },
    render::text::{
        RenderedDocument, RenderedGraphic, RenderedGraphicContent, RenderedLine, RenderedLineKind,
    },
    ui::terminal::{CellMetrics, GraphicViewport},
};

mod resident;
pub(super) use resident::PageGraphics;

type GraphicCommands = (Vec<String>, Vec<(u32, u32)>);

pub(super) fn collect_graphics_commands(
    rendered: &RenderedDocument,
    viewport: GraphicViewport,
    previous_visible_placements: &[(u32, u32)],
    transmitted_graphics: &mut BTreeSet<u32>,
) -> GraphicCommands {
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

        let (columns, rows, offset_x, offset_y) =
            fit_graphic_placement(graphic, viewport.cell_metrics);
        let placement = KittyImagePlacement {
            image_id,
            placement_id: image_id,
            columns: columns.min(viewport.viewport_width),
            rows,
            source_x_px: 0,
            source_y_px: 0,
            source_width_px: graphic.natural_width_px,
            source_height_px: graphic.natural_height_px,
            cursor_x: 0,
            cursor_y: 0,
            z_index: -1,
        };
        let graphic_x = viewport.article_x
            + viewport.article_width.saturating_sub(graphic.width_cells) / 2
            + offset_x;
        commands.push(format!(
            "{}{}",
            ansi_cursor_move(graphic_x, relative_row as u16 + offset_y),
            encode_place(&placement)
        ));
    }

    (commands, visible_placements)
}

pub(super) fn fit_graphic_placement(
    graphic: &RenderedGraphic,
    cell_metrics: CellMetrics,
) -> (u16, u16, u16, u16) {
    let box_columns = graphic.width_cells.max(1);
    let box_rows = graphic.height_cells.max(1);
    let image_width = graphic.natural_width_px.max(1) as f32;
    let image_height = graphic.natural_height_px.max(1) as f32;
    let image_aspect = image_width / image_height;
    let box_width_px = f32::from(box_columns) * cell_metrics.width_px;
    let box_height_px = f32::from(box_rows) * cell_metrics.height_px;
    let box_aspect = box_width_px / box_height_px.max(1.0);

    let (columns, rows) = if image_aspect >= box_aspect {
        let fitted_rows = ((box_width_px / image_aspect) / cell_metrics.height_px).floor() as u16;
        (box_columns, fitted_rows.clamp(1, box_rows))
    } else {
        let fitted_columns =
            ((box_height_px * image_aspect) / cell_metrics.width_px).floor() as u16;
        (fitted_columns.clamp(1, box_columns), box_rows)
    };

    let offset_x = box_columns.saturating_sub(columns) / 2;
    let offset_y = box_rows.saturating_sub(rows) / 2;
    (columns, rows, offset_x, offset_y)
}

pub(super) fn resize_graphic_space(
    rendered: &mut RenderedDocument,
    graphic_index: usize,
    new_height: u16,
) {
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
                    spans: Vec::new(),
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

pub(super) fn visible_graphic_placements(
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};

    use crate::ui::page_graphics::build_graphic_page;

    use super::{CellMetrics, PageGraphics};

    fn page() -> crate::ui::page_graphics::GraphicPage {
        let image = ImageBuffer::from_fn(8, 160, |x, y| Rgba([x as u8, y as u8, 40, 255]));
        let mut png = Vec::new();
        DynamicImage::ImageRgba8(image)
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap_or_else(|error| panic!("fixture should encode: {error}"));
        build_graphic_page(&png, 16, 8)
            .unwrap_or_else(|error| panic!("fixture page should decode: {error}"))
    }

    fn wait_for_preparation(graphics: &mut PageGraphics) -> Vec<String> {
        while graphics.has_pending_work() {
            let commands = graphics
                .maintenance()
                .unwrap_or_else(|error| panic!("resident preparation should complete: {error}"));
            if !commands.is_empty() || !graphics.has_pending_work() {
                return commands;
            }
            std::thread::yield_now();
        }
        Vec::new()
    }

    #[test]
    fn resident_window_reuses_an_exact_crop_during_fractional_scroll() {
        let page = page();
        let mut graphics = PageGraphics::default();
        let metrics = CellMetrics { width_px: 8.0, height_px: 2.0 };

        let (first, visible) = graphics
            .commands(&page, 0.0, 4, metrics, &[])
            .unwrap_or_else(|error| panic!("first resident window should render: {error}"));
        let initial_payload = first
            .iter()
            .find(|command| command.contains("a=t"))
            .and_then(|command| command.split_once(';'))
            .map_or("", |(_, payload)| payload.trim_end_matches("\u{1b}\\"));
        let initial = image::load_from_memory(
            &STANDARD
                .decode(initial_payload)
                .unwrap_or_else(|error| panic!("initial payload should decode: {error}")),
        )
        .unwrap_or_else(|error| panic!("initial png should decode: {error}"))
        .to_rgba8();
        assert_eq!(initial.dimensions(), (8, 8));
        assert_eq!(*initial.get_pixel(7, 7), Rgba([7, 7, 40, 255]));
        let prepared = wait_for_preparation(&mut graphics);
        assert_eq!(prepared.iter().filter(|command| command.contains("a=t")).count(), 1);
        assert!(!prepared.iter().any(|command| command.contains("a=p")));
        let (second, _) = graphics
            .commands(&page, 1.25, 4, metrics, &visible)
            .unwrap_or_else(|error| panic!("resident window should render: {error}"));

        assert_eq!(first.iter().filter(|command| command.contains("a=t")).count(), 1);
        assert_eq!(second.iter().filter(|command| command.contains("a=t")).count(), 0);
        assert!(second.iter().any(|command| command.contains("y=3,w=8,h=8")));

        let payload = prepared
            .iter()
            .find(|command| command.contains("a=t"))
            .and_then(|command| command.split_once(';'))
            .map_or("", |(_, payload)| payload.trim_end_matches("\u{1b}\\"));
        let uploaded = image::load_from_memory(
            &STANDARD
                .decode(payload)
                .unwrap_or_else(|error| panic!("resident payload should decode: {error}")),
        )
        .unwrap_or_else(|error| panic!("resident png should decode: {error}"))
        .to_rgba8();
        assert_eq!(*uploaded.get_pixel(7, 23), Rgba([7, 23, 40, 255]));
        let visible = image::imageops::crop_imm(&uploaded, 0, 3, 8, 8).to_image();
        let expected = image::imageops::crop_imm(page.image.as_ref(), 0, 3, 8, 8).to_image();
        assert_eq!(visible, expected, "fractional placement must retain source pixels");
    }

    #[test]
    fn resident_windows_evict_old_image_data_after_the_cache_is_full() {
        let page = page();
        let mut graphics = PageGraphics::default();
        let metrics = CellMetrics { width_px: 8.0, height_px: 2.0 };

        let (_, first_visible) = graphics
            .commands(&page, 0.0, 4, metrics, &[])
            .unwrap_or_else(|error| panic!("first resident window should render: {error}"));
        let (_, second_visible) = graphics
            .commands(&page, 24.0, 4, metrics, &first_visible)
            .unwrap_or_else(|error| panic!("second resident window should render: {error}"));
        let (third, _) = graphics
            .commands(&page, 48.0, 4, metrics, &second_visible)
            .unwrap_or_else(|error| panic!("third resident window should render: {error}"));

        assert!(third.iter().any(|command| command.contains("a=t")));
        assert!(third.iter().any(|command| command.contains("a=d,d=I")));
        assert_eq!(graphics.reset().len(), 2, "the cache retains only two resident images");
    }

    #[test]
    fn reset_deletes_each_owned_resident_image() {
        let page = page();
        let mut graphics = PageGraphics::default();
        let metrics = CellMetrics { width_px: 8.0, height_px: 2.0 };
        let (_, first_visible) = graphics
            .commands(&page, 0.0, 4, metrics, &[])
            .unwrap_or_else(|error| panic!("first resident window should render: {error}"));
        let _ = graphics
            .commands(&page, 24.0, 4, metrics, &first_visible)
            .unwrap_or_else(|error| panic!("second resident window should render: {error}"));

        let deletes = graphics.reset();

        assert_eq!(deletes.len(), 2);
        assert!(deletes.iter().all(|command| command.contains("a=d,d=I")));
    }

    #[test]
    fn a_replaced_page_invalidates_resident_image_data() {
        let first_page = page();
        let second_page = page();
        let mut graphics = PageGraphics::default();
        let metrics = CellMetrics { width_px: 8.0, height_px: 2.0 };
        let (_, visible) = graphics
            .commands(&first_page, 0.0, 4, metrics, &[])
            .unwrap_or_else(|error| panic!("first page should render: {error}"));

        let (commands, _) = graphics
            .commands(&second_page, 0.0, 4, metrics, &visible)
            .unwrap_or_else(|error| panic!("replaced page should render: {error}"));

        assert_eq!(commands.iter().filter(|command| command.contains("a=d,d=I")).count(), 1);
        assert_eq!(commands.iter().filter(|command| command.contains("a=t")).count(), 1);
    }

    #[test]
    fn maintenance_uploads_a_prepared_hidden_window_without_a_placement() {
        let page = page();
        let mut graphics = PageGraphics::default();
        let metrics = CellMetrics { width_px: 8.0, height_px: 2.0 };
        let _ = graphics
            .commands(&page, 0.0, 4, metrics, &[])
            .unwrap_or_else(|error| panic!("first resident window should render: {error}"));

        assert!(graphics.has_pending_work());
        let prepared = wait_for_preparation(&mut graphics);

        assert_eq!(prepared.iter().filter(|command| command.contains("a=t")).count(), 1);
        assert!(!prepared.iter().any(|command| command.contains("a=p")));
        let payload = prepared
            .iter()
            .find(|command| command.contains("a=t"))
            .and_then(|command| command.split_once(';'))
            .map_or("", |(_, payload)| payload.trim_end_matches("\u{1b}\\"));
        let uploaded = image::load_from_memory(
            &STANDARD
                .decode(payload)
                .unwrap_or_else(|error| panic!("prepared payload should decode: {error}")),
        )
        .unwrap_or_else(|error| panic!("prepared png should decode: {error}"))
        .to_rgba8();
        assert_eq!(*uploaded.get_pixel(7, 23), Rgba([7, 23, 40, 255]));
        assert_eq!(
            graphics.reset().len(),
            2,
            "one active and one prepared resident remain bounded"
        );
    }

    #[test]
    fn reset_discards_an_inflight_preparation() {
        let page = page();
        let mut graphics = PageGraphics::default();
        let metrics = CellMetrics { width_px: 8.0, height_px: 2.0 };
        let _ = graphics
            .commands(&page, 0.0, 4, metrics, &[])
            .unwrap_or_else(|error| panic!("first resident window should render: {error}"));

        assert!(graphics.has_pending_work());
        let _ = graphics.reset();

        assert!(!graphics.has_pending_work());
        assert!(
            graphics
                .maintenance()
                .unwrap_or_else(|error| panic!("stale preparation should not fail: {error}"))
                .is_empty()
        );
    }
}

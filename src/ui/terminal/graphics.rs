use std::collections::BTreeSet;

use crate::{
    io::kitty_graphics::{
        DeleteCommand, KittyImagePlacement, encode_delete, encode_place, encode_transmit_png,
    },
    render::text::{
        RenderedDocument, RenderedGraphic, RenderedGraphicContent, RenderedLine, RenderedLineKind,
    },
    ui::{
        page_graphics::{GraphicPage, viewport_slice},
        terminal::{CellMetrics, GraphicViewport},
    },
};

const PAGE_IMAGE_ID: u32 = 1;

pub(super) fn collect_page_viewport_commands(
    page: &GraphicPage,
    scroll: usize,
    content_height: usize,
    cell_metrics: CellMetrics,
    previous_visible_placements: &[(u32, u32)],
    transmitted_graphics: &mut BTreeSet<u32>,
) -> (Vec<String>, Vec<(u32, u32)>) {
    let mut commands = Vec::new();
    let slice = viewport_slice(page, scroll, content_height, cell_metrics.height_px);
    let visible_placements =
        if slice.rows == 0 { Vec::new() } else { vec![(PAGE_IMAGE_ID, PAGE_IMAGE_ID)] };

    for (image_id, placement_id) in previous_visible_placements
        .iter()
        .copied()
        .filter(|placement| !visible_placements.contains(placement))
    {
        commands.push(encode_delete(DeleteCommand::Placement { image_id, placement_id }));
    }

    if slice.rows > 0 {
        if transmitted_graphics.insert(PAGE_IMAGE_ID) {
            commands.push(encode_transmit_png(PAGE_IMAGE_ID, &page.png_bytes));
        }

        let placement = KittyImagePlacement {
            image_id: PAGE_IMAGE_ID,
            placement_id: PAGE_IMAGE_ID,
            columns: page.width_cells,
            rows: slice.rows,
            source_x_px: 0,
            source_y_px: slice.source_y_px,
            source_width_px: page.image_width_px,
            source_height_px: slice.source_height_px,
            cursor_x: 0,
            cursor_y: 0,
            z_index: -1,
        };
        commands.push(format!("{}{}", ansi_cursor_move(0, 0), encode_place(&placement)));
    }

    (commands, visible_placements)
}

pub(super) fn collect_graphics_commands(
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

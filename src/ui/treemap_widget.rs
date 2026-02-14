use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use std::collections::HashMap;

use crate::format::{format_bytes, truncate_unicode};
use crate::ui::theme::{BorderStyle, ColoredTreemapRect, Theme};

const LUMINANCE_BLACK_TEXT_THRESHOLD: f64 = 130.0;

pub struct TreemapWidget<'a> {
    rects: &'a [ColoredTreemapRect],
    selected_index: usize,
    min_label_width: u16,
    min_label_height: u16,
    _border_style: BorderStyle,
    theme: &'a Theme,
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    rects: &[ColoredTreemapRect],
    selected_index: usize,
    min_label_width: u16,
    min_label_height: u16,
    border_style: BorderStyle,
    theme: &Theme,
) {
    let widget = TreemapWidget {
        rects,
        selected_index,
        min_label_width,
        min_label_height,
        _border_style: border_style,
        theme,
    };
    frame.render_widget(widget, area);
}

impl<'a> Widget for TreemapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "perf-tracing")]
        let _render_span = tracing::debug_span!(
            "ui.treemap_widget.render",
            rects = self.rects.len(),
            width = area.width,
            height = area.height
        )
        .entered();

        if self.rects.is_empty() {
            let msg = "No process data";
            if area.width as usize >= msg.len() && area.height >= 1 {
                let x = area.x + (area.width - msg.len() as u16) / 2;
                let y = area.y + area.height / 2;
                buf.set_string(x, y, msg, Style::default().fg(Color::DarkGray));
            }
            return;
        }

        // Treemap rectangles are mathematically flush; seams are visual only.
        // Draw in ordered passes to avoid overwriting and wide gutters.
        let selected_rect = self
            .rects
            .get(self.selected_index)
            .and_then(|r| tile_rect(area, &r.rect));

        // Pass 1: paint tile backgrounds.
        for trect in self.rects {
            let Some(term_rect) = tile_rect(area, &trect.rect) else {
                continue;
            };
            fill_rect(buf, term_rect, Style::default().bg(trect.color));
        }

        // Pass 2: draw shared plain seams for unselected tiles.
        let separator_color = self.theme.surface_bg;
        let seam_rects: Vec<Rect> = self
            .rects
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != self.selected_index)
            .filter_map(|(_, trect)| tile_rect(area, &trect.rect))
            .collect();
        draw_seam_grid(buf, area, &seam_rects, Style::default().fg(separator_color));

        // Pass 3: render labels on top of fills and seams.
        for (i, trect) in self.rects.iter().enumerate() {
            let _is_selected = i == self.selected_index;
            let Some(term_rect) = tile_rect(area, &trect.rect) else {
                continue;
            };

            let bg_color = trect.color;
            let fg_color = contrast_color(bg_color);

            let (label_x, label_max_w) = if term_rect.width >= 4 {
                (term_rect.x + 2, term_rect.width.saturating_sub(3))
            } else if term_rect.width >= 3 {
                (term_rect.x + 1, term_rect.width.saturating_sub(2))
            } else if term_rect.width >= 2 {
                (term_rect.x + 1, term_rect.width.saturating_sub(1))
            } else {
                (term_rect.x, term_rect.width)
            };
            let label_y = if term_rect.height >= 3 {
                term_rect.y + 1
            } else {
                term_rect.y
            };

            if term_rect.width >= self.min_label_width && term_rect.height >= self.min_label_height
            {
                if label_max_w >= 5 {
                    let label = truncate_unicode(&trect.label, label_max_w as usize);
                    let style = Style::default()
                        .fg(fg_color)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD);
                    buf.set_string(label_x, label_y, &label, style);
                }

                let value_y = label_y + 1;
                if value_y < term_rect.y + term_rect.height && label_max_w >= 8 {
                    let value_str = format_bytes(trect.value);
                    let value = truncate_unicode(&value_str, label_max_w as usize);
                    let style = Style::default().fg(fg_color).bg(bg_color);
                    buf.set_string(label_x, value_y, &value, style);
                }
            }
        }

        // Pass 4: draw selected heavy border last so neighbors cannot overwrite it.
        if let (Some(term_rect), Some(trect)) = (selected_rect, self.rects.get(self.selected_index))
            && term_rect.width >= 3
            && term_rect.height >= 3
        {
            let border_style = Style::default()
                .fg(self.theme.selection_border)
                .bg(trect.color)
                .add_modifier(Modifier::BOLD);
            draw_heavy_border(buf, term_rect, border_style);
        }
    }
}

fn tile_rect(area: Rect, logical: &crate::treemap::node::LayoutRect) -> Option<Rect> {
    let x = area.x + logical.x.round() as u16;
    let y = area.y + logical.y.round() as u16;
    let w = logical.width.round() as u16;
    let h = logical.height.round() as u16;

    if w == 0 || h == 0 {
        return None;
    }

    let x2 = (x + w).min(area.x + area.width);
    let y2 = (y + h).min(area.y + area.height);
    if x >= x2 || y >= y2 {
        return None;
    }

    Some(Rect::new(x, y, x2 - x, y2 - y))
}

fn fill_rect(buf: &mut Buffer, rect: Rect, style: Style) {
    for row in rect.y..rect.y + rect.height {
        for col in rect.x..rect.x + rect.width {
            if let Some(cell) = buf.cell_mut((col, row)) {
                cell.set_style(style);
                cell.set_char(' ');
            }
        }
    }
}

const MASK_N: u8 = 0b0001;
const MASK_E: u8 = 0b0010;
const MASK_S: u8 = 0b0100;
const MASK_W: u8 = 0b1000;

fn draw_seam_grid(buf: &mut Buffer, area: Rect, rects: &[Rect], style: Style) {
    let mut seam_masks: HashMap<(u16, u16), u8> = HashMap::new();

    for rect in rects {
        if rect.width < 2 || rect.height < 2 {
            continue;
        }

        let x1 = rect.x;
        let y1 = rect.y;
        let x2 = rect.x + rect.width - 1;
        let y2 = rect.y + rect.height - 1;
        let outer_right = x2 + 1 == area.x + area.width;
        let outer_bottom = y2 + 1 == area.y + area.height;

        // Shared seams: top/left always; right/bottom only at treemap outer bounds.
        mark_horizontal_segment(&mut seam_masks, x1, x2, y1);
        mark_vertical_segment(&mut seam_masks, x1, y1, y2);
        if outer_right {
            mark_vertical_segment(&mut seam_masks, x2, y1, y2);
        }
        if outer_bottom {
            mark_horizontal_segment(&mut seam_masks, x1, x2, y2);
        }
    }

    connect_adjacent_seams(&mut seam_masks);

    for ((x, y), mask) in seam_masks {
        let ch = seam_glyph(mask);
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char(ch).set_style(style);
        }
    }
}

fn mark_horizontal_segment(seam_masks: &mut HashMap<(u16, u16), u8>, x1: u16, x2: u16, y: u16) {
    if x1 >= x2 {
        return;
    }
    for x in x1..x2 {
        connect_cells(seam_masks, (x, y), (x + 1, y));
    }
}

fn mark_vertical_segment(seam_masks: &mut HashMap<(u16, u16), u8>, x: u16, y1: u16, y2: u16) {
    if y1 >= y2 {
        return;
    }
    for y in y1..y2 {
        connect_cells(seam_masks, (x, y), (x, y + 1));
    }
}

fn connect_adjacent_seams(seam_masks: &mut HashMap<(u16, u16), u8>) {
    let points: Vec<(u16, u16)> = seam_masks.keys().copied().collect();
    for (x, y) in points {
        if let Some(right_mask) = seam_masks.get(&(x + 1, y)).copied() {
            let current_mask = seam_masks.get(&(x, y)).copied().unwrap_or(0);
            let current_horizontal = (current_mask & (MASK_E | MASK_W)) != 0;
            let right_horizontal = (right_mask & (MASK_E | MASK_W)) != 0;
            if current_horizontal || right_horizontal {
                connect_cells(seam_masks, (x, y), (x + 1, y));
            }
        }

        if let Some(down_mask) = seam_masks.get(&(x, y + 1)).copied() {
            let current_mask = seam_masks.get(&(x, y)).copied().unwrap_or(0);
            let current_vertical = (current_mask & (MASK_N | MASK_S)) != 0;
            let down_vertical = (down_mask & (MASK_N | MASK_S)) != 0;
            if current_vertical || down_vertical {
                connect_cells(seam_masks, (x, y), (x, y + 1));
            }
        }
    }
}

fn connect_cells(seam_masks: &mut HashMap<(u16, u16), u8>, a: (u16, u16), b: (u16, u16)) {
    let (ax, ay) = a;
    let (bx, by) = b;
    if ax == bx {
        if ay + 1 == by {
            *seam_masks.entry(a).or_default() |= MASK_S;
            *seam_masks.entry(b).or_default() |= MASK_N;
        } else if by + 1 == ay {
            *seam_masks.entry(a).or_default() |= MASK_N;
            *seam_masks.entry(b).or_default() |= MASK_S;
        }
    } else if ay == by {
        if ax + 1 == bx {
            *seam_masks.entry(a).or_default() |= MASK_E;
            *seam_masks.entry(b).or_default() |= MASK_W;
        } else if bx + 1 == ax {
            *seam_masks.entry(a).or_default() |= MASK_W;
            *seam_masks.entry(b).or_default() |= MASK_E;
        }
    }
}

fn seam_glyph(mask: u8) -> char {
    match mask {
        0 => ' ',
        m if m == (MASK_N | MASK_S) => '\u{2502}',
        m if m == (MASK_E | MASK_W) => '\u{2500}',
        m if m == (MASK_S | MASK_E) => '\u{250C}',
        m if m == (MASK_S | MASK_W) => '\u{2510}',
        m if m == (MASK_N | MASK_E) => '\u{2514}',
        m if m == (MASK_N | MASK_W) => '\u{2518}',
        m if m == (MASK_N | MASK_S | MASK_E) => '\u{251C}',
        m if m == (MASK_N | MASK_S | MASK_W) => '\u{2524}',
        m if m == (MASK_E | MASK_W | MASK_S) => '\u{252C}',
        m if m == (MASK_E | MASK_W | MASK_N) => '\u{2534}',
        m if m == (MASK_N | MASK_E | MASK_S | MASK_W) => '\u{253C}',
        m if (m & (MASK_N | MASK_S)) != 0 => '\u{2502}',
        _ => '\u{2500}',
    }
}

fn contrast_color(bg: Color) -> Color {
    if let Some((r, g, b)) = color_to_rgb(bg) {
        let luminance = color_luminance(r, g, b);
        if luminance >= LUMINANCE_BLACK_TEXT_THRESHOLD {
            Color::Black
        } else {
            Color::White
        }
    } else {
        Color::White
    }
}

fn color_luminance(r: u8, g: u8, b: u8) -> f64 {
    0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64
}

fn color_to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(index) => Some(ansi256_to_rgb(index)),
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((205, 49, 49)),
        Color::Green => Some((13, 188, 121)),
        Color::Yellow => Some((229, 229, 16)),
        Color::Blue => Some((36, 114, 200)),
        Color::Magenta => Some((188, 63, 188)),
        Color::Cyan => Some((17, 168, 205)),
        Color::Gray => Some((229, 229, 229)),
        Color::DarkGray => Some((102, 102, 102)),
        Color::LightRed => Some((241, 76, 76)),
        Color::LightGreen => Some((35, 209, 139)),
        Color::LightYellow => Some((245, 245, 67)),
        Color::LightBlue => Some((59, 142, 234)),
        Color::LightMagenta => Some((214, 112, 214)),
        Color::LightCyan => Some((41, 184, 219)),
        Color::White => Some((255, 255, 255)),
        Color::Reset => None,
    }
}

fn ansi256_to_rgb(index: u8) -> (u8, u8, u8) {
    const ANSI_16: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];

    match index {
        0..=15 => ANSI_16[index as usize],
        16..=231 => {
            let idx = index - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            let level = |n: u8| if n == 0 { 0 } else { 55 + 40 * n };
            (level(r), level(g), level(b))
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

fn draw_heavy_border(buf: &mut Buffer, rect: Rect, style: Style) {
    let x1 = rect.x;
    let y1 = rect.y;
    let x2 = rect.x + rect.width - 1;
    let y2 = rect.y + rect.height - 1;

    if let Some(c) = buf.cell_mut((x1, y1)) {
        c.set_char('\u{250F}').set_style(style);
    }
    if let Some(c) = buf.cell_mut((x2, y1)) {
        c.set_char('\u{2513}').set_style(style);
    }
    if let Some(c) = buf.cell_mut((x1, y2)) {
        c.set_char('\u{2517}').set_style(style);
    }
    if let Some(c) = buf.cell_mut((x2, y2)) {
        c.set_char('\u{251B}').set_style(style);
    }

    for col in (x1 + 1)..x2 {
        if let Some(c) = buf.cell_mut((col, y1)) {
            c.set_char('\u{2501}').set_style(style);
        }
        if let Some(c) = buf.cell_mut((col, y2)) {
            c.set_char('\u{2501}').set_style(style);
        }
    }

    for row in (y1 + 1)..y2 {
        if let Some(c) = buf.cell_mut((x1, row)) {
            c.set_char('\u{2503}').set_style(style);
        }
        if let Some(c) = buf.cell_mut((x2, row)) {
            c.set_char('\u{2503}').set_style(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::treemap::node::LayoutRect;
    use crate::ui::theme::Theme;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn contrast_uses_black_for_bright_rgb() {
        assert_eq!(contrast_color(Color::Rgb(251, 146, 60)), Color::Black);
    }

    #[test]
    fn contrast_uses_white_for_dark_rgb() {
        assert_eq!(contrast_color(Color::Rgb(49, 50, 68)), Color::White);
    }

    #[test]
    fn contrast_uses_black_for_bright_indexed() {
        assert_eq!(contrast_color(Color::Indexed(226)), Color::Black);
    }

    #[test]
    fn contrast_uses_white_for_dark_indexed() {
        assert_eq!(contrast_color(Color::Indexed(17)), Color::White);
    }

    fn render_test_buffer(
        rects: &[ColoredTreemapRect],
        selected: usize,
        area: Rect,
        min_label_width: u16,
        min_label_height: u16,
    ) -> Buffer {
        let mut buf = Buffer::empty(area);
        let theme = Theme::dark();
        let widget = TreemapWidget {
            rects,
            selected_index: selected,
            min_label_width,
            min_label_height,
            _border_style: BorderStyle::Thin,
            theme: &theme,
        };
        widget.render(area, &mut buf);
        buf
    }

    #[test]
    fn shared_seam_has_no_blank_spacer_column() {
        let rects = vec![
            ColoredTreemapRect {
                rect: LayoutRect::new(0.0, 0.0, 4.0, 4.0),
                id: 1,
                label: "a".into(),
                value: 1,
                color: Color::Rgb(96, 165, 250),
            },
            ColoredTreemapRect {
                rect: LayoutRect::new(4.0, 0.0, 4.0, 4.0),
                id: 2,
                label: "b".into(),
                value: 1,
                color: Color::Rgb(251, 146, 60),
            },
        ];
        let area = Rect::new(0, 0, 8, 4);
        let buf = render_test_buffer(&rects, usize::MAX, area, 99, 99);
        let seam_symbol = buf.cell((4, 1)).unwrap().symbol();
        assert_eq!(seam_symbol, "│");
    }

    #[test]
    fn selected_heavy_border_overrides_shared_seam() {
        let rects = vec![
            ColoredTreemapRect {
                rect: LayoutRect::new(0.0, 0.0, 4.0, 4.0),
                id: 1,
                label: "a".into(),
                value: 1,
                color: Color::Rgb(96, 165, 250),
            },
            ColoredTreemapRect {
                rect: LayoutRect::new(4.0, 0.0, 4.0, 4.0),
                id: 2,
                label: "b".into(),
                value: 1,
                color: Color::Rgb(251, 146, 60),
            },
        ];
        let area = Rect::new(0, 0, 8, 4);
        let buf = render_test_buffer(&rects, 1, area, 99, 99);
        let seam_symbol = buf.cell((4, 1)).unwrap().symbol();
        assert_eq!(seam_symbol, "┃");
    }

    #[test]
    fn seams_use_connected_junction_glyphs() {
        let rects = vec![
            ColoredTreemapRect {
                rect: LayoutRect::new(0.0, 0.0, 12.0, 4.0),
                id: 1,
                label: "a".into(),
                value: 1,
                color: Color::Rgb(96, 165, 250),
            },
            ColoredTreemapRect {
                rect: LayoutRect::new(0.0, 4.0, 6.0, 4.0),
                id: 2,
                label: "b".into(),
                value: 1,
                color: Color::Rgb(251, 146, 60),
            },
            ColoredTreemapRect {
                rect: LayoutRect::new(6.0, 4.0, 6.0, 4.0),
                id: 3,
                label: "c".into(),
                value: 1,
                color: Color::Rgb(45, 212, 191),
            },
        ];
        let area = Rect::new(0, 0, 12, 8);
        let buf = render_test_buffer(&rects, usize::MAX, area, 99, 99);
        assert_eq!(buf.cell((6, 4)).unwrap().symbol(), "┬");
    }

    #[test]
    fn labels_have_left_breathing_room() {
        let rects = vec![ColoredTreemapRect {
            rect: LayoutRect::new(0.0, 0.0, 10.0, 4.0),
            id: 1,
            label: "alpha".into(),
            value: 1_000_000,
            color: Color::Rgb(96, 165, 250),
        }];
        let area = Rect::new(0, 0, 10, 4);
        let buf = render_test_buffer(&rects, usize::MAX, area, 1, 1);

        // Row 1 is the label row. x=1 should stay blank; text begins at x=2.
        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 1)).unwrap().symbol(), "a");
    }
}

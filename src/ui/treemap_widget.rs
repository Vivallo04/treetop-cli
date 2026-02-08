use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use crate::format::{format_bytes, truncate_unicode};
use crate::ui::theme::{BorderStyle, ColoredTreemapRect, Theme};

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

        for (i, trect) in self.rects.iter().enumerate() {
            let is_selected = i == self.selected_index;

            let x = area.x + trect.rect.x.round() as u16;
            let y = area.y + trect.rect.y.round() as u16;
            let w = trect.rect.width.round() as u16;
            let h = trect.rect.height.round() as u16;

            if w == 0 || h == 0 {
                continue;
            }

            let x2 = (x + w).min(area.x + area.width);
            let y2 = (y + h).min(area.y + area.height);
            if x >= x2 || y >= y2 {
                continue;
            }
            let w = x2 - x;
            let h = y2 - y;

            let term_rect = Rect::new(x, y, w, h);

            let bg_color = trect.color;
            let fg_color = contrast_color(bg_color);
            let separator_color = self.theme.surface_bg;
            let bg_style = Style::default().bg(bg_color);
            for row in term_rect.y..term_rect.y + term_rect.height {
                for col in term_rect.x..term_rect.x + term_rect.width {
                    if let Some(cell) = buf.cell_mut((col, row)) {
                        cell.set_style(bg_style);
                        cell.set_char(' ');
                    }
                }
            }

            if w >= 3 && h >= 3 {
                let border_style = if is_selected {
                    Style::default()
                        .fg(self.theme.selection_border)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(separator_color).bg(bg_color)
                };
                if is_selected {
                    draw_heavy_border(buf, term_rect, border_style);
                } else {
                    draw_border(buf, term_rect, border_style);
                }
            }

            let (label_x, label_max_w) = if term_rect.width >= 3 {
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
    }
}

fn contrast_color(bg: Color) -> Color {
    match bg {
        Color::Rgb(r, g, b) => {
            let luminance = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
            if luminance >= 115.0 {
                Color::Black
            } else {
                Color::White
            }
        }
        Color::White
        | Color::Cyan
        | Color::Yellow
        | Color::Green
        | Color::Red
        | Color::Magenta
        | Color::LightRed
        | Color::LightGreen
        | Color::LightYellow
        | Color::LightMagenta
        | Color::LightCyan
        | Color::LightBlue => Color::Black,
        _ => Color::White,
    }
}

fn draw_border(buf: &mut Buffer, rect: Rect, style: Style) {
    let x1 = rect.x;
    let y1 = rect.y;
    let x2 = rect.x + rect.width - 1;
    let y2 = rect.y + rect.height - 1;

    if let Some(c) = buf.cell_mut((x1, y1)) {
        c.set_char('\u{250C}').set_style(style);
    }
    if let Some(c) = buf.cell_mut((x2, y1)) {
        c.set_char('\u{2510}').set_style(style);
    }
    if let Some(c) = buf.cell_mut((x1, y2)) {
        c.set_char('\u{2514}').set_style(style);
    }
    if let Some(c) = buf.cell_mut((x2, y2)) {
        c.set_char('\u{2518}').set_style(style);
    }

    for col in (x1 + 1)..x2 {
        if let Some(c) = buf.cell_mut((col, y1)) {
            c.set_char('\u{2500}').set_style(style);
        }
        if let Some(c) = buf.cell_mut((col, y2)) {
            c.set_char('\u{2500}').set_style(style);
        }
    }

    for row in (y1 + 1)..y2 {
        if let Some(c) = buf.cell_mut((x1, row)) {
            c.set_char('\u{2502}').set_style(style);
        }
        if let Some(c) = buf.cell_mut((x2, row)) {
            c.set_char('\u{2502}').set_style(style);
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

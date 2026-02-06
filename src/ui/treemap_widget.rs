use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::treemap::color::Theme;
use crate::treemap::node::TreemapRect;

pub struct TreemapWidget<'a> {
    rects: &'a [TreemapRect],
    selected_index: usize,
    min_label_width: u16,
    min_label_height: u16,
    theme: &'a Theme,
}

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    rects: &[TreemapRect],
    selected_index: usize,
    min_label_width: u16,
    min_label_height: u16,
    theme: &Theme,
) {
    let widget = TreemapWidget {
        rects,
        selected_index,
        min_label_width,
        min_label_height,
        theme,
    };
    frame.render_widget(widget, area);
}

impl<'a> Widget for TreemapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
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
            // Inset by 1 cell on right and bottom to create visual gap between rects
            let w = if x2 - x > 2 { x2 - x - 1 } else { x2 - x };
            let h = if y2 - y > 1 { y2 - y - 1 } else { y2 - y };

            let term_rect = Rect::new(x, y, w, h);

            // Fill background
            let bg_color = trect.color;
            let fg_color = contrast_color(bg_color);
            let bg_style = Style::default().bg(bg_color);
            for row in term_rect.y..term_rect.y + term_rect.height {
                for col in term_rect.x..term_rect.x + term_rect.width {
                    if let Some(cell) = buf.cell_mut((col, row)) {
                        cell.set_style(bg_style);
                        cell.set_char(' ');
                    }
                }
            }

            // Draw rounded border for all rects that are large enough
            if w >= 3 && h >= 3 {
                let border_style = if is_selected {
                    Style::default()
                        .fg(self.theme.selection_border)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(self.theme.overlay_border)
                        .bg(bg_color)
                };
                draw_rounded_border(buf, term_rect, border_style);
            }

            // Label positioning — always inside border since all rects have them now
            let (label_x, label_max_w) = if w >= 3 {
                (x + 1, w.saturating_sub(2))
            } else if w >= 2 {
                (x + 1, w.saturating_sub(1))
            } else {
                (x, w)
            };
            let label_y = if h >= 3 { y + 1 } else { y };

            // Only render text if the rect meets minimum size thresholds
            if w >= self.min_label_width && h >= self.min_label_height {
                // Render process name (need at least 5 chars for a readable label)
                if label_max_w >= 5 {
                    let label = truncate(&trect.label, label_max_w as usize);
                    let style = Style::default()
                        .fg(fg_color)
                        .bg(bg_color)
                        .add_modifier(Modifier::BOLD);
                    buf.set_string(label_x, label_y, &label, style);
                }

                // Render memory value below label if space allows
                let value_y = label_y + 1;
                if value_y < y + h && label_max_w >= 8 {
                    let value_str = format_bytes(trect.value);
                    let value = truncate(&value_str, label_max_w as usize);
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
            if luminance > 128.0 {
                Color::Rgb(30, 30, 46)
            } else {
                Color::Rgb(205, 214, 244)
            }
        }
        Color::Yellow | Color::Green | Color::LightGreen | Color::LightYellow => Color::Black,
        _ => Color::White,
    }
}

fn draw_rounded_border(buf: &mut Buffer, rect: Rect, style: Style) {
    let x1 = rect.x;
    let y1 = rect.y;
    let x2 = rect.x + rect.width - 1;
    let y2 = rect.y + rect.height - 1;

    // Rounded corners
    if let Some(c) = buf.cell_mut((x1, y1)) {
        c.set_char('\u{256D}').set_style(style); // ╭
    }
    if let Some(c) = buf.cell_mut((x2, y1)) {
        c.set_char('\u{256E}').set_style(style); // ╮
    }
    if let Some(c) = buf.cell_mut((x1, y2)) {
        c.set_char('\u{2570}').set_style(style); // ╰
    }
    if let Some(c) = buf.cell_mut((x2, y2)) {
        c.set_char('\u{256F}').set_style(style); // ╯
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

fn truncate(s: &str, max_width: usize) -> String {
    if s.width() <= max_width {
        return s.to_string();
    }
    let mut result = String::new();
    let mut width = 0;
    for ch in s.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > max_width.saturating_sub(1) {
            result.push('\u{2026}');
            break;
        }
        result.push(ch);
        width += ch_width;
    }
    result
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

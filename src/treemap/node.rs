use ratatui::style::Color;

#[derive(Clone, Debug)]
pub struct TreemapItem {
    pub id: u32,
    pub label: String,
    pub value: u64,
}

#[derive(Clone, Debug)]
pub struct LayoutRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl LayoutRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn shorter_side(&self) -> f64 {
        self.width.min(self.height)
    }
}

#[derive(Clone, Debug)]
pub struct TreemapRect {
    pub rect: LayoutRect,
    pub id: u32,
    pub label: String,
    pub value: u64,
    pub color: Color,
}

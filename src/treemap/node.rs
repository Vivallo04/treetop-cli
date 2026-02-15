#[derive(Clone, Debug)]
pub struct TreemapItem {
    pub pid: u32,
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

    pub fn lerp(&self, target: &Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            x: self.x + (target.x - self.x) * t,
            y: self.y + (target.y - self.y) * t,
            width: self.width + (target.width - self.width) * t,
            height: self.height + (target.height - self.height) * t,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TreemapRect {
    pub rect: LayoutRect,
    pub pid: u32,
    pub label: String,
    pub value: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_endpoints() {
        let a = LayoutRect::new(0.0, 0.0, 10.0, 20.0);
        let b = LayoutRect::new(5.0, 10.0, 30.0, 40.0);

        let start = a.lerp(&b, 0.0);
        assert!((start.x - 0.0).abs() < 1e-10);
        assert!((start.width - 10.0).abs() < 1e-10);

        let end = a.lerp(&b, 1.0);
        assert!((end.x - 5.0).abs() < 1e-10);
        assert!((end.width - 30.0).abs() < 1e-10);
    }

    #[test]
    fn lerp_midpoint() {
        let a = LayoutRect::new(0.0, 0.0, 10.0, 20.0);
        let b = LayoutRect::new(10.0, 20.0, 30.0, 40.0);

        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-10);
        assert!((mid.y - 10.0).abs() < 1e-10);
        assert!((mid.width - 20.0).abs() < 1e-10);
        assert!((mid.height - 30.0).abs() < 1e-10);
    }
}

//! Rectangles in global, top-left-origin screen coordinates.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn max_x(&self) -> f64 {
        self.x + self.width
    }

    pub fn max_y(&self) -> f64 {
        self.y + self.height
    }

    /// Equal after rounding every component to whole points.
    pub fn integral_eq(&self, other: &Rect) -> bool {
        let r = |r: &Rect| [r.x.round(), r.y.round(), r.width.round(), r.height.round()];
        r(self) == r(other)
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let w = self.max_x().min(other.max_x()) - x;
        let h = self.max_y().min(other.max_y()) - y;
        (w > 0.0 && h > 0.0).then(|| Rect::new(x, y, w, h))
    }

    pub fn intersection_area(&self, other: &Rect) -> f64 {
        self.intersection(other).map_or(0.0, |r| r.width * r.height)
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(
            x,
            y,
            self.max_x().max(other.max_x()) - x,
            self.max_y().max(other.max_y()) - y,
        )
    }

    /// A rectangle of this size — cut down to fit, if need be — in the middle of `within`.
    pub fn centred_in(&self, within: &Rect) -> Rect {
        let width = self.width.min(within.width);
        let height = self.height.min(within.height);
        Rect::new(
            within.x + (within.width - width) / 2.0,
            within.y + (within.height - height) / 2.0,
            width,
            height,
        )
    }

    pub fn union_all(rects: impl Iterator<Item = Rect>) -> Option<Rect> {
        rects.reduce(|a, b| a.union(&b))
    }
}

#[cfg(test)]
mod tests;

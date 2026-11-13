use image::RgbaImage;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    /// Snap pointer input to source pixels. Fractional DPI conversions can turn
    /// an integer edge into 1883.0001; floor/ceil cropping would add a pixel.
    pub fn from_viewport(x: f32, y: f32, scale: (f32, f32)) -> Self {
        Self::new((x / scale.0).round(), (y / scale.1).round())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn between(a: Point, b: Point) -> Self {
        Self {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
            w: (b.x - a.x).abs(),
            h: (b.y - a.y).abs(),
        }
    }
    pub fn right(self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(self) -> f32 {
        self.y + self.h
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }
    pub fn pixel_bounds(self, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let x = self.x.floor().clamp(0., width as f32) as u32;
        let y = self.y.floor().clamp(0., height as f32) as u32;
        let right = self.right().ceil().clamp(0., width as f32) as u32;
        let bottom = self.bottom().ceil().clamp(0., height as f32) as u32;
        (right > x && bottom > y).then_some((
            x,
            y,
            right.saturating_sub(x),
            bottom.saturating_sub(y),
        ))
    }
    pub fn handles(self) -> [Point; 8] {
        [
            Point::new(self.x, self.y),
            Point::new(self.x + self.w / 2., self.y),
            Point::new(self.right(), self.y),
            Point::new(self.right(), self.y + self.h / 2.),
            Point::new(self.right(), self.bottom()),
            Point::new(self.x + self.w / 2., self.bottom()),
            Point::new(self.x, self.bottom()),
            Point::new(self.x, self.y + self.h / 2.),
        ]
    }
    pub fn resize(self, handle: usize, p: Point) -> Self {
        let (mut x1, mut y1, mut x2, mut y2) = (self.x, self.y, self.right(), self.bottom());
        if [0, 6, 7].contains(&handle) {
            x1 = p.x;
        }
        if [2, 3, 4].contains(&handle) {
            x2 = p.x;
        }
        if [0, 1, 2].contains(&handle) {
            y1 = p.y;
        }
        if [4, 5, 6].contains(&handle) {
            y2 = p.y;
        }
        Self::between(Point::new(x1, y1), Point::new(x2, y2))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tool {
    Select,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Pen,
    Text,
    Eraser,
}

#[derive(Clone, Debug)]
pub struct Style {
    pub color: u32,
    pub width: f32,
    pub filled: bool,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub font_family: String,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            color: 0xf04452,
            width: 3.,
            filled: false,
            font_size: 24.,
            bold: false,
            italic: false,
            font_family: "sans-serif".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Annotation {
    pub tool: Tool,
    pub points: Vec<Point>,
    pub style: Style,
    pub text: String,
}
impl Annotation {
    pub fn new(tool: Tool, start: Point, style: Style) -> Self {
        Self {
            tool,
            points: vec![start, start],
            style,
            text: String::new(),
        }
    }
    pub fn extend(&mut self, p: Point) {
        if matches!(self.tool, Tool::Pen | Tool::Eraser) {
            let last = self.points.last().unwrap();
            if (last.x - p.x).hypot(last.y - p.y) > 0.7 {
                self.points.push(p);
            }
        } else {
            self.points[1] = p;
        }
    }
    pub fn bounds(&self) -> Rect {
        Rect::between(self.points[0], *self.points.last().unwrap())
    }
}

/// Commands, not pixels, form the undo history. Eraser strokes clear only the annotation layer.
pub struct Document {
    pub source: Arc<RgbaImage>,
    pub selection: Option<Rect>,
    history: Vec<Annotation>,
    cursor: usize,
}
impl Document {
    pub fn new(source: RgbaImage) -> Self {
        Self {
            source: Arc::new(source),
            selection: None,
            history: Vec::new(),
            cursor: 0,
        }
    }
    pub fn annotations(&self) -> &[Annotation] {
        &self.history[..self.cursor]
    }
    pub fn push(&mut self, annotation: Annotation) {
        self.history.truncate(self.cursor);
        self.history.push(annotation);
        self.cursor += 1;
    }
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_redo(&self) -> bool {
        self.cursor < self.history.len()
    }
    pub fn undo(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    pub fn redo(&mut self) {
        self.cursor = (self.cursor + 1).min(self.history.len());
    }
    pub fn clamp(&self, p: Point) -> Point {
        Point::new(
            p.x.clamp(0., self.source.width() as f32),
            p.y.clamp(0., self.source.height() as f32),
        )
    }
}

/// Keep both toolbars inside the current monitor, preferring below the selection.
pub fn toolbar_position(selection: Rect, screen: Rect, width: f32, height: f32) -> Point {
    let x = (selection.right() - width).clamp(
        screen.x + 8.,
        (screen.right() - width - 8.).max(screen.x + 8.),
    );
    let below = selection.bottom() + 10.;
    let y = if below + height <= screen.bottom() - 8. {
        below
    } else if selection.y - height - 10. >= screen.y + 8. {
        selection.y - height - 10.
    } else {
        (screen.bottom() - height - 8.).max(screen.y + 8.)
    };
    Point::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reverse_selection_and_pixel_clipping() {
        let r = Rect::between(Point::new(99.2, 80.1), Point::new(-2., 10.3));
        assert_eq!(r.pixel_bounds(90, 70), Some((0, 10, 90, 60)));
        assert_eq!(
            Rect {
                x: 100.,
                y: 0.,
                w: 10.,
                h: 10.
            }
            .pixel_bounds(90, 70),
            None
        );
    }
    #[test]
    fn fractional_dpi_selection_has_exact_pixel_edges() {
        // Real 175% / 100% desktop: 3840x3240, origin (0, -1080).
        let dpi = 1.75;
        let scale = ((3840. / dpi) / 3840., (3240. / dpi) / 3240.);
        let a = Point::from_viewport(1883. / dpi, 2147. / dpi, scale);
        let b = Point::from_viewport(1710. / dpi, 2050. / dpi, scale);
        assert_eq!(
            Rect::between(a, b).pixel_bounds(3840, 3240),
            Some((1710, 2050, 173, 97))
        );
        assert_eq!(Rect::between(a, b), Rect::between(b, a));
    }
    #[test]
    fn redo_branch_is_discarded() {
        let mut d = Document::new(RgbaImage::new(8, 8));
        let a = Annotation::new(Tool::Line, Point::default(), Style::default());
        d.push(a.clone());
        d.push(a.clone());
        d.undo();
        assert!(d.can_redo());
        d.push(a);
        assert!(!d.can_redo());
        assert_eq!(d.annotations().len(), 2);
        d.undo();
        d.undo();
        d.undo();
        assert!(!d.can_undo());
        d.redo();
        assert_eq!(d.annotations().len(), 1);
    }
    #[test]
    fn toolbar_avoids_screen_edge() {
        let p = toolbar_position(
            Rect {
                x: 1830.,
                y: 1010.,
                w: 90.,
                h: 70.,
            },
            Rect {
                x: 0.,
                y: 0.,
                w: 1920.,
                h: 1080.,
            },
            600.,
            100.,
        );
        assert!(p.x + 600. <= 1920.);
        assert!(p.y + 100. <= 1010.);
    }
}

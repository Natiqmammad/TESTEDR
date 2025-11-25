// Canvas implementation for Flutter rendering
// This provides the drawing API that layers use to paint their content

use super::{Matrix4, Offset, Rect, Size};

/// Color representation in RGBA format
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba(r, g, b, 255)
    }

    pub fn from_argb(argb: u32) -> Self {
        Self {
            a: ((argb >> 24) & 0xFF) as u8,
            r: ((argb >> 16) & 0xFF) as u8,
            g: ((argb >> 8) & 0xFF) as u8,
            b: (argb & 0xFF) as u8,
        }
    }

    pub fn to_argb(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn with_alpha(&self, alpha: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }

    pub fn with_opacity(&self, opacity: f64) -> Self {
        let alpha = (self.a as f64 * opacity.max(0.0).min(1.0)) as u8;
        self.with_alpha(alpha)
    }

    // Common colors
    pub fn transparent() -> Self {
        Self::from_rgba(0, 0, 0, 0)
    }

    pub fn black() -> Self {
        Self::from_rgb(0, 0, 0)
    }

    pub fn white() -> Self {
        Self::from_rgb(255, 255, 255)
    }

    pub fn red() -> Self {
        Self::from_rgb(255, 0, 0)
    }

    pub fn green() -> Self {
        Self::from_rgb(0, 255, 0)
    }

    pub fn blue() -> Self {
        Self::from_rgb(0, 0, 255)
    }

    pub fn yellow() -> Self {
        Self::from_rgb(255, 255, 0)
    }

    pub fn cyan() -> Self {
        Self::from_rgb(0, 255, 255)
    }

    pub fn magenta() -> Self {
        Self::from_rgb(255, 0, 255)
    }

    pub fn grey(value: u8) -> Self {
        Self::from_rgb(value, value, value)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::black()
    }
}

/// Paint style for drawing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintStyle {
    Fill,
    Stroke,
}

/// Blend mode for compositing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Clear,
    Src,
    Dst,
    SrcOver,
    DstOver,
    SrcIn,
    DstIn,
    SrcOut,
    DstOut,
    SrcAtop,
    DstAtop,
    Xor,
    Plus,
    Modulate,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Multiply,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::SrcOver
    }
}

/// Cap style for strokes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

impl Default for StrokeCap {
    fn default() -> Self {
        Self::Butt
    }
}

/// Join style for strokes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

impl Default for StrokeJoin {
    fn default() -> Self {
        Self::Miter
    }
}

/// Paint configuration for drawing operations
#[derive(Debug, Clone)]
pub struct Paint {
    pub color: Color,
    pub style: PaintStyle,
    pub stroke_width: f64,
    pub stroke_cap: StrokeCap,
    pub stroke_join: StrokeJoin,
    pub blend_mode: BlendMode,
    pub anti_alias: bool,
    pub filter_quality: FilterQuality,
}

impl Paint {
    pub fn new() -> Self {
        Self {
            color: Color::black(),
            style: PaintStyle::Fill,
            stroke_width: 1.0,
            stroke_cap: StrokeCap::default(),
            stroke_join: StrokeJoin::default(),
            blend_mode: BlendMode::default(),
            anti_alias: true,
            filter_quality: FilterQuality::default(),
        }
    }

    pub fn from_color(color: Color) -> Self {
        Self {
            color,
            ..Self::new()
        }
    }

    pub fn with_style(mut self, style: PaintStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    pub fn with_anti_alias(mut self, anti_alias: bool) -> Self {
        self.anti_alias = anti_alias;
        self
    }
}

impl Default for Paint {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter quality for image rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterQuality {
    None,
    Low,
    Medium,
    High,
}

impl Default for FilterQuality {
    fn default() -> Self {
        Self::Low
    }
}

/// Path for complex shapes
#[derive(Debug, Clone)]
pub struct Path {
    commands: Vec<PathCommand>,
    bounds: Option<Rect>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PathCommand {
    MoveTo(Offset),
    LineTo(Offset),
    QuadraticBezierTo(Offset, Offset),
    CubicBezierTo(Offset, Offset, Offset),
    ArcTo(Rect, f64, f64, bool),
    Close,
}

impl Path {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            bounds: None,
        }
    }

    pub fn move_to(&mut self, point: Offset) -> &mut Self {
        self.commands.push(PathCommand::MoveTo(point));
        self.bounds = None;
        self
    }

    pub fn line_to(&mut self, point: Offset) -> &mut Self {
        self.commands.push(PathCommand::LineTo(point));
        self.bounds = None;
        self
    }

    pub fn quadratic_bezier_to(&mut self, control: Offset, end: Offset) -> &mut Self {
        self.commands
            .push(PathCommand::QuadraticBezierTo(control, end));
        self.bounds = None;
        self
    }

    pub fn cubic_bezier_to(
        &mut self,
        control1: Offset,
        control2: Offset,
        end: Offset,
    ) -> &mut Self {
        self.commands
            .push(PathCommand::CubicBezierTo(control1, control2, end));
        self.bounds = None;
        self
    }

    pub fn arc_to(
        &mut self,
        rect: Rect,
        start_angle: f64,
        sweep_angle: f64,
        force_move_to: bool,
    ) -> &mut Self {
        self.commands.push(PathCommand::ArcTo(
            rect,
            start_angle,
            sweep_angle,
            force_move_to,
        ));
        self.bounds = None;
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.commands.push(PathCommand::Close);
        self
    }

    pub fn add_rect(&mut self, rect: Rect) -> &mut Self {
        self.move_to(rect.offset);
        self.line_to(Offset::new(rect.right(), rect.top()));
        self.line_to(Offset::new(rect.right(), rect.bottom()));
        self.line_to(Offset::new(rect.left(), rect.bottom()));
        self.close()
    }

    pub fn add_circle(&mut self, center: Offset, radius: f64) -> &mut Self {
        let rect = Rect::from_ltrb(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        );
        self.arc_to(rect, 0.0, 2.0 * std::f64::consts::PI, true)
    }

    pub fn add_oval(&mut self, rect: Rect) -> &mut Self {
        self.arc_to(rect, 0.0, 2.0 * std::f64::consts::PI, true)
    }

    pub fn bounds(&self) -> Option<Rect> {
        self.bounds
    }

    fn calculate_bounds(&mut self) {
        // TODO: Calculate actual bounds from path commands
        self.bounds = Some(Rect::from_ltrb(0.0, 0.0, 100.0, 100.0));
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

/// Canvas save/restore state
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CanvasState {
    transform: Matrix4,
    clip_rect: Option<Rect>,
    opacity: f64,
}

/// Canvas for drawing operations
pub struct Canvas {
    size: Size,
    commands: Vec<DrawCommand>,
    state_stack: Vec<CanvasState>,
    current_state: CanvasState,
}

#[derive(Debug, Clone)]
pub enum DrawCommand {
    Save,
    Restore,
    Transform(Matrix4),
    ClipRect(Rect),
    DrawRect(Rect, Paint),
    DrawCircle(Offset, f64, Paint),
    DrawPath(Path, Paint),
    DrawText(Offset, String, TextStyle),
    DrawImage(Offset, Image, Option<Paint>),
    DrawLine(Offset, Offset, Paint),
    DrawPoints(Vec<Offset>, Paint),
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            commands: Vec::new(),
            state_stack: Vec::new(),
            current_state: CanvasState {
                transform: Matrix4::identity(),
                clip_rect: None,
                opacity: 1.0,
            },
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn save(&mut self) {
        self.state_stack.push(self.current_state.clone());
        self.commands.push(DrawCommand::Save);
    }

    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.current_state = state;
            self.commands.push(DrawCommand::Restore);
        }
    }

    pub fn transform(&mut self, matrix: Matrix4) {
        self.current_state.transform = self.current_state.transform.multiply(&matrix);
        self.commands.push(DrawCommand::Transform(matrix));
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.transform(Matrix4::translation(dx, dy));
    }

    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.transform(Matrix4::scale(sx, sy));
    }

    pub fn rotate(&mut self, radians: f64) {
        self.transform(Matrix4::rotation_z(radians));
    }

    pub fn clip_rect(&mut self, rect: Rect) {
        self.current_state.clip_rect = Some(match self.current_state.clip_rect {
            Some(current) => current.intersection(&rect).unwrap_or(Rect::zero()),
            None => rect,
        });
        self.commands.push(DrawCommand::ClipRect(rect));
    }

    pub fn draw_rect(&mut self, rect: Rect, paint: Paint) {
        self.commands.push(DrawCommand::DrawRect(rect, paint));
    }

    pub fn draw_circle(&mut self, center: Offset, radius: f64, paint: Paint) {
        self.commands
            .push(DrawCommand::DrawCircle(center, radius, paint));
    }

    pub fn draw_path(&mut self, path: Path, paint: Paint) {
        self.commands.push(DrawCommand::DrawPath(path, paint));
    }

    pub fn draw_line(&mut self, from: Offset, to: Offset, paint: Paint) {
        self.commands.push(DrawCommand::DrawLine(from, to, paint));
    }

    pub fn draw_points(&mut self, points: Vec<Offset>, paint: Paint) {
        self.commands.push(DrawCommand::DrawPoints(points, paint));
    }

    pub fn draw_text(&mut self, offset: Offset, text: String, style: TextStyle) {
        self.commands
            .push(DrawCommand::DrawText(offset, text, style));
    }

    pub fn draw_image(&mut self, offset: Offset, image: Image, paint: Option<Paint>) {
        self.commands
            .push(DrawCommand::DrawImage(offset, image, paint));
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.state_stack.clear();
        self.current_state = CanvasState {
            transform: Matrix4::identity(),
            clip_rect: None,
            opacity: 1.0,
        };
    }
}

/// Text style for drawing text
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub color: Color,
    pub letter_spacing: f64,
    pub word_spacing: f64,
    pub height: f64,
}

impl TextStyle {
    pub fn new() -> Self {
        Self {
            font_family: "Roboto".to_string(),
            font_size: 14.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            color: Color::black(),
            letter_spacing: 0.0,
            word_spacing: 0.0,
            height: 1.0,
        }
    }

    pub fn with_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }

    pub fn with_family(mut self, family: String) -> Self {
        self.font_family = family;
        self
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

/// Image representation
#[derive(Debug, Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub format: ImageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    RGBA8888,
    RGB888,
    BGRA8888,
    BGR888,
    Gray8,
}

impl Image {
    pub fn new(width: u32, height: u32, format: ImageFormat) -> Self {
        let bytes_per_pixel = match format {
            ImageFormat::RGBA8888 | ImageFormat::BGRA8888 => 4,
            ImageFormat::RGB888 | ImageFormat::BGR888 => 3,
            ImageFormat::Gray8 => 1,
        };
        let data_size = (width * height * bytes_per_pixel) as usize;

        Self {
            width,
            height,
            data: vec![0; data_size],
            format,
        }
    }

    pub fn from_data(width: u32, height: u32, data: Vec<u8>, format: ImageFormat) -> Self {
        Self {
            width,
            height,
            data,
            format,
        }
    }

    pub fn size(&self) -> Size {
        Size::new(self.width as f64, self.height as f64)
    }
}

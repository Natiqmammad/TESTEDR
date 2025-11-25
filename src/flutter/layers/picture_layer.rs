// PictureLayer - A layer that contains drawing commands
// This layer represents a recorded sequence of drawing operations that can be replayed

use super::canvas::{DrawCommand, Image, Paint, Path, TextStyle};
use super::{Layer, LayerBase, Offset, PaintContext, PrerollContext, Rect};
use std::fmt::Debug;

/// A recorded sequence of drawing operations
#[derive(Debug, Clone)]
pub struct Picture {
    commands: Vec<DrawCommand>,
    bounds: Rect,
}

impl Picture {
    /// Create a new empty picture
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            bounds: Rect::zero(),
        }
    }

    /// Create a picture from draw commands
    pub fn from_commands(commands: Vec<DrawCommand>) -> Self {
        let bounds = Self::calculate_bounds(&commands);
        Self { commands, bounds }
    }

    /// Add a draw rect command
    pub fn draw_rect(&mut self, rect: Rect, paint: Paint) {
        self.commands.push(DrawCommand::DrawRect(rect, paint));
        self.update_bounds(rect);
    }

    /// Add a draw circle command
    pub fn draw_circle(&mut self, center: Offset, radius: f64, paint: Paint) {
        self.commands
            .push(DrawCommand::DrawCircle(center, radius, paint));
        let circle_bounds = Rect::from_ltrb(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        );
        self.update_bounds(circle_bounds);
    }

    /// Add a draw path command
    pub fn draw_path(&mut self, path: Path, paint: Paint) {
        self.commands
            .push(DrawCommand::DrawPath(path.clone(), paint));
        if let Some(path_bounds) = path.bounds() {
            self.update_bounds(path_bounds);
        }
    }

    /// Add a draw line command
    pub fn draw_line(&mut self, from: Offset, to: Offset, paint: Paint) {
        self.commands.push(DrawCommand::DrawLine(from, to, paint));
        let line_bounds = Rect::from_ltrb(
            from.x.min(to.x),
            from.y.min(to.y),
            from.x.max(to.x),
            from.y.max(to.y),
        );
        self.update_bounds(line_bounds);
    }

    /// Add a draw text command
    pub fn draw_text(&mut self, offset: Offset, text: &str, style: TextStyle) {
        self.commands.push(DrawCommand::DrawText(
            offset,
            text.to_string(),
            style.clone(),
        ));
        // Approximate text bounds (would need proper text metrics in real implementation)
        let approx_width = text.len() as f64 * style.font_size * 0.6;
        let approx_height = style.font_size * 1.2;
        let text_bounds = Rect::new(offset, super::Size::new(approx_width, approx_height));
        self.update_bounds(text_bounds);
    }

    /// Add a draw image command
    pub fn draw_image(&mut self, offset: Offset, image: Image, paint: Option<Paint>) {
        let image_size = image.size();
        self.commands
            .push(DrawCommand::DrawImage(offset, image, paint));
        let image_bounds = Rect::new(offset, image_size);
        self.update_bounds(image_bounds);
    }

    /// Get the commands
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Get the bounds
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Clear all commands
    pub fn clear(&mut self) {
        self.commands.clear();
        self.bounds = Rect::zero();
    }

    /// Update bounds with a new rect
    fn update_bounds(&mut self, rect: Rect) {
        if self.bounds.size.is_empty() {
            self.bounds = rect;
        } else {
            self.bounds = self.bounds.union(&rect);
        }
    }

    /// Calculate bounds from commands
    fn calculate_bounds(commands: &[DrawCommand]) -> Rect {
        let mut bounds = Rect::zero();

        for command in commands {
            match command {
                DrawCommand::DrawRect(rect, _) => {
                    if bounds.size.is_empty() {
                        bounds = *rect;
                    } else {
                        bounds = bounds.union(rect);
                    }
                }
                DrawCommand::DrawCircle(center, radius, _) => {
                    let circle_bounds = Rect::from_ltrb(
                        center.x - radius,
                        center.y - radius,
                        center.x + radius,
                        center.y + radius,
                    );
                    if bounds.size.is_empty() {
                        bounds = circle_bounds;
                    } else {
                        bounds = bounds.union(&circle_bounds);
                    }
                }
                DrawCommand::DrawLine(from, to, _) => {
                    let line_bounds = Rect::from_ltrb(
                        from.x.min(to.x),
                        from.y.min(to.y),
                        from.x.max(to.x),
                        from.y.max(to.y),
                    );
                    if bounds.size.is_empty() {
                        bounds = line_bounds;
                    } else {
                        bounds = bounds.union(&line_bounds);
                    }
                }
                _ => {}
            }
        }

        bounds
    }

    /// Check if the picture is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for Picture {
    fn default() -> Self {
        Self::new()
    }
}

/// A layer that draws a recorded picture
pub struct PictureLayer {
    base: LayerBase,
    offset: Offset,
    picture: Picture,
    is_complex: bool,
    will_change: bool,
}

impl PictureLayer {
    /// Create a new picture layer
    pub fn new(offset: Offset, picture: Picture) -> Self {
        let mut base = LayerBase::new();
        let bounds = picture.bounds().translate(&offset);
        base.set_bounds(bounds);

        Self {
            base,
            offset,
            picture,
            is_complex: false,
            will_change: false,
        }
    }

    /// Create an empty picture layer
    pub fn empty() -> Self {
        Self::new(Offset::zero(), Picture::new())
    }

    /// Set the picture
    pub fn set_picture(&mut self, picture: Picture) {
        self.picture = picture;
        let bounds = self.picture.bounds().translate(&self.offset);
        self.base.set_bounds(bounds);
        self.base.mark_needs_repaint();
    }

    /// Set the offset
    pub fn set_offset(&mut self, offset: Offset) {
        if self.offset != offset {
            self.offset = offset;
            let bounds = self.picture.bounds().translate(&self.offset);
            self.base.set_bounds(bounds);
            self.base.mark_needs_repaint();
        }
    }

    /// Get the picture
    pub fn picture(&self) -> &Picture {
        &self.picture
    }

    /// Get the offset
    pub fn offset(&self) -> Offset {
        self.offset
    }

    /// Set whether this picture is complex (hint for caching)
    pub fn set_is_complex(&mut self, is_complex: bool) {
        self.is_complex = is_complex;
    }

    /// Set whether this picture will change frequently (hint for caching)
    pub fn set_will_change(&mut self, will_change: bool) {
        self.will_change = will_change;
    }

    /// Check if the picture is complex
    pub fn is_complex(&self) -> bool {
        self.is_complex
    }

    /// Check if the picture will change frequently
    pub fn will_change(&self) -> bool {
        self.will_change
    }
}

impl Layer for PictureLayer {
    fn preroll(&mut self, _context: &PrerollContext) {
        // Picture layers don't need special preroll logic
        // Bounds are already calculated from the picture
        self.base.clear_needs_repaint();
    }

    fn paint(&self, context: &PaintContext) {
        if self.picture.is_empty() {
            return;
        }

        let canvas = context.canvas.clone();
        let mut canvas_guard = canvas.write().unwrap();

        // Save the canvas state
        canvas_guard.save();

        // Apply the offset transformation
        if self.offset.x != 0.0 || self.offset.y != 0.0 {
            canvas_guard.translate(self.offset.x, self.offset.y);
        }

        // Apply the current context transform
        canvas_guard.transform(context.transform.clone());

        // Replay the picture commands
        for command in self.picture.commands() {
            match command {
                DrawCommand::DrawRect(rect, paint) => {
                    let paint_with_opacity = if context.opacity < 1.0 {
                        let mut p = paint.clone();
                        p.color = p.color.with_opacity(context.opacity);
                        p
                    } else {
                        paint.clone()
                    };
                    canvas_guard.draw_rect(*rect, paint_with_opacity);
                }
                DrawCommand::DrawCircle(center, radius, paint) => {
                    let paint_with_opacity = if context.opacity < 1.0 {
                        let mut p = paint.clone();
                        p.color = p.color.with_opacity(context.opacity);
                        p
                    } else {
                        paint.clone()
                    };
                    canvas_guard.draw_circle(*center, *radius, paint_with_opacity);
                }
                DrawCommand::DrawPath(path, paint) => {
                    let paint_with_opacity = if context.opacity < 1.0 {
                        let mut p = paint.clone();
                        p.color = p.color.with_opacity(context.opacity);
                        p
                    } else {
                        paint.clone()
                    };
                    canvas_guard.draw_path(path.clone(), paint_with_opacity);
                }
                DrawCommand::DrawLine(from, to, paint) => {
                    let paint_with_opacity = if context.opacity < 1.0 {
                        let mut p = paint.clone();
                        p.color = p.color.with_opacity(context.opacity);
                        p
                    } else {
                        paint.clone()
                    };
                    canvas_guard.draw_line(*from, *to, paint_with_opacity);
                }
                DrawCommand::DrawText(offset, text, style) => {
                    let style_with_opacity = if context.opacity < 1.0 {
                        let mut s = style.clone();
                        s.color = s.color.with_opacity(context.opacity);
                        s
                    } else {
                        style.clone()
                    };
                    canvas_guard.draw_text(*offset, text.clone(), style_with_opacity);
                }
                DrawCommand::DrawImage(offset, image, paint) => {
                    let paint_with_opacity = paint.as_ref().map(|p| {
                        if context.opacity < 1.0 {
                            let mut p = p.clone();
                            p.color = p.color.with_opacity(context.opacity);
                            p
                        } else {
                            p.clone()
                        }
                    });
                    canvas_guard.draw_image(*offset, image.clone(), paint_with_opacity);
                }
                _ => {
                    // Handle other commands if needed
                }
            }
        }

        // Restore the canvas state
        canvas_guard.restore();
    }

    fn bounds(&self) -> Rect {
        self.base.bounds()
    }

    fn hit_test(&self, position: &Offset) -> bool {
        self.base.bounds().contains(position)
    }

    fn id(&self) -> u64 {
        self.base.id()
    }

    fn needs_repaint(&self) -> bool {
        self.base.needs_repaint()
    }

    fn mark_needs_repaint(&mut self) {
        self.base.mark_needs_repaint();
    }

    fn layer_type(&self) -> &'static str {
        "PictureLayer"
    }
}

impl Debug for PictureLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PictureLayer")
            .field("id", &self.base.id())
            .field("offset", &self.offset)
            .field("bounds", &self.base.bounds())
            .field("command_count", &self.picture.commands().len())
            .field("is_complex", &self.is_complex)
            .field("will_change", &self.will_change)
            .field("needs_repaint", &self.base.needs_repaint())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::super::canvas::Color;
    use super::*;

    #[test]
    fn test_picture_creation() {
        let mut picture = Picture::new();
        assert!(picture.is_empty());
        assert_eq!(picture.bounds(), Rect::zero());

        picture.draw_rect(
            Rect::from_ltrb(0.0, 0.0, 100.0, 100.0),
            Paint::from_color(Color::red()),
        );

        assert!(!picture.is_empty());
        assert_eq!(picture.commands().len(), 1);
        assert_eq!(picture.bounds(), Rect::from_ltrb(0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn test_picture_layer() {
        let mut picture = Picture::new();
        picture.draw_rect(
            Rect::from_ltrb(0.0, 0.0, 50.0, 50.0),
            Paint::from_color(Color::blue()),
        );

        let offset = Offset::new(10.0, 10.0);
        let layer = PictureLayer::new(offset, picture);

        assert_eq!(layer.offset(), offset);
        assert_eq!(layer.bounds(), Rect::from_ltrb(10.0, 10.0, 60.0, 60.0));
        assert!(!layer.is_complex());
        assert!(!layer.will_change());
    }

    #[test]
    fn test_picture_bounds_update() {
        let mut picture = Picture::new();

        picture.draw_rect(Rect::from_ltrb(0.0, 0.0, 50.0, 50.0), Paint::new());
        assert_eq!(picture.bounds(), Rect::from_ltrb(0.0, 0.0, 50.0, 50.0));

        picture.draw_rect(Rect::from_ltrb(25.0, 25.0, 75.0, 75.0), Paint::new());
        assert_eq!(picture.bounds(), Rect::from_ltrb(0.0, 0.0, 75.0, 75.0));

        picture.draw_circle(Offset::new(100.0, 50.0), 25.0, Paint::new());
        assert_eq!(picture.bounds(), Rect::from_ltrb(0.0, 0.0, 125.0, 75.0));
    }
}

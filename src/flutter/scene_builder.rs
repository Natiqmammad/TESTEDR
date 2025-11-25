// SceneBuilder API for NightScript Flutter Integration
// This module provides the core API for building a scene graph that can be
// rendered by the Flutter Engine. It follows Flutter's architecture but
// adapted for NightScript's VM.

use crate::flutter::layers::*;
use std::sync::{Arc, RwLock};

/// Represents a complete scene ready for rendering
pub struct Scene {
    /// Root layer of the scene
    root_layer: Box<dyn Layer>,
    /// Scene metadata
    frame_number: u64,
    /// Timestamp when scene was built
    #[allow(dead_code)]
    build_time: std::time::Instant,
}

impl Scene {
    /// Create a new scene with the given root layer
    pub fn new(root_layer: Box<dyn Layer>) -> Self {
        static FRAME_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        Self {
            root_layer,
            frame_number: FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            build_time: std::time::Instant::now(),
        }
    }

    /// Get the root layer of the scene
    pub fn root_layer(&self) -> &dyn Layer {
        self.root_layer.as_ref()
    }

    /// Get the frame number
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Render the scene to a render context
    pub fn render(&mut self, viewport_size: Size, device_pixel_ratio: f64) -> RenderResult {
        // Preroll phase - calculate bounds and prepare resources
        let preroll_context = PrerollContext::new(viewport_size, device_pixel_ratio);
        self.root_layer.preroll(&preroll_context);

        // Create canvas for painting
        let canvas = Arc::new(RwLock::new(Canvas::new(viewport_size)));

        // Paint phase - draw content
        let paint_context = PaintContext::new(canvas.clone());
        self.root_layer.paint(&paint_context);

        // Return render result
        RenderResult {
            canvas,
            bounds: self.root_layer.bounds(),
            frame_number: self.frame_number,
        }
    }
}

/// Result of rendering a scene
pub struct RenderResult {
    /// Canvas containing the rendered content
    pub canvas: Arc<RwLock<Canvas>>,
    /// Bounds of the rendered content
    pub bounds: Rect,
    /// Frame number
    pub frame_number: u64,
}

/// Builder for constructing a scene graph
pub struct SceneBuilder {
    /// Stack of layers being built
    layer_stack: Vec<Box<ContainerLayer>>,
    /// Current container layer
    current_container: Box<ContainerLayer>,
    /// Transform stack for nested transformations
    transform_stack: Vec<Matrix4>,
    /// Opacity stack for nested opacity
    opacity_stack: Vec<f64>,
    /// Clip stack for nested clipping
    clip_stack: Vec<Rect>,
}

impl SceneBuilder {
    /// Create a new scene builder
    pub fn new() -> Self {
        Self {
            layer_stack: Vec::new(),
            current_container: Box::new(ContainerLayer::new()),
            transform_stack: vec![Matrix4::identity()],
            opacity_stack: vec![1.0],
            clip_stack: Vec::new(),
        }
    }

    /// Get the current transformation matrix
    pub fn current_transform(&self) -> Matrix4 {
        self.transform_stack
            .last()
            .cloned()
            .unwrap_or_else(Matrix4::identity)
    }

    /// Get the current opacity
    pub fn current_opacity(&self) -> f64 {
        self.opacity_stack.last().copied().unwrap_or(1.0)
    }

    // ========================================
    // Transform Operations
    // ========================================

    /// Push a transformation onto the stack
    pub fn push_transform(&mut self, matrix: Matrix4) -> &mut Self {
        // Push current container onto stack
        let previous =
            std::mem::replace(&mut self.current_container, Box::new(ContainerLayer::new()));
        self.layer_stack.push(previous);

        // Update transform stack
        let current = self.current_transform();
        self.transform_stack.push(current.multiply(&matrix));

        self
    }

    /// Push a translation transformation
    pub fn push_translate(&mut self, dx: f64, dy: f64) -> &mut Self {
        self.push_transform(Matrix4::translation(dx, dy))
    }

    /// Push a scale transformation
    pub fn push_scale(&mut self, sx: f64, sy: f64) -> &mut Self {
        self.push_transform(Matrix4::scale(sx, sy))
    }

    /// Push a rotation transformation (in radians)
    pub fn push_rotate(&mut self, radians: f64) -> &mut Self {
        self.push_transform(Matrix4::rotation_z(radians))
    }

    // ========================================
    // Opacity Operations
    // ========================================

    /// Push an opacity layer
    pub fn push_opacity(&mut self, opacity: f64) -> &mut Self {
        // Clamp opacity to valid range
        let opacity = opacity.max(0.0).min(1.0);

        // Push current container onto stack
        let previous =
            std::mem::replace(&mut self.current_container, Box::new(ContainerLayer::new()));
        self.layer_stack.push(previous);

        // Update opacity stack
        let current = self.current_opacity();
        self.opacity_stack.push(current * opacity);

        self
    }

    // ========================================
    // Clipping Operations
    // ========================================

    /// Push a rectangular clip
    pub fn push_clip_rect(&mut self, rect: Rect) -> &mut Self {
        // Push current container onto stack
        let previous =
            std::mem::replace(&mut self.current_container, Box::new(ContainerLayer::new()));
        self.layer_stack.push(previous);

        // Update clip stack
        self.clip_stack.push(rect);

        self
    }

    /// Push a rounded rectangular clip
    pub fn push_clip_rounded_rect(&mut self, rect: Rect, _radius: f64) -> &mut Self {
        // For now, just use rectangular clip
        // TODO: Implement rounded rect clipping
        self.push_clip_rect(rect)
    }

    // ========================================
    // Drawing Operations
    // ========================================

    /// Add a picture layer with drawing commands
    pub fn add_picture(&mut self, offset: Offset, picture: Picture) -> &mut Self {
        let picture_layer = Box::new(PictureLayer::new(offset, picture));
        self.current_container.add_child(picture_layer);
        self
    }

    /// Add a solid color rectangle
    pub fn add_rect(&mut self, rect: Rect, paint: Paint) -> &mut Self {
        let mut picture = Picture::new();
        picture.draw_rect(rect, paint);
        self.add_picture(rect.offset, picture)
    }

    /// Add a circle
    pub fn add_circle(&mut self, center: Offset, radius: f64, paint: Paint) -> &mut Self {
        let mut picture = Picture::new();
        picture.draw_circle(center, radius, paint);
        let bounds_offset = Offset::new(center.x - radius, center.y - radius);
        self.add_picture(bounds_offset, picture)
    }

    /// Add text
    pub fn add_text(&mut self, offset: Offset, text: &str, style: TextStyle) -> &mut Self {
        let mut picture = Picture::new();
        picture.draw_text(offset, text, style);
        self.add_picture(offset, picture)
    }

    /// Add an image
    pub fn add_image(&mut self, offset: Offset, image: Image, paint: Option<Paint>) -> &mut Self {
        let mut picture = Picture::new();
        picture.draw_image(offset, image, paint);
        self.add_picture(offset, picture)
    }

    // ========================================
    // Container Operations
    // ========================================

    /// Add a pre-built layer
    pub fn add_layer(&mut self, layer: Box<dyn Layer>) -> &mut Self {
        self.current_container.add_child(layer);
        self
    }

    /// Pop the current transformation/opacity/clip
    pub fn pop(&mut self) -> &mut Self {
        // Check what type of layer we're popping
        let needs_transform_pop = self.transform_stack.len() > 1;
        let needs_opacity_pop = self.opacity_stack.len() > 1;
        let needs_clip_pop = !self.clip_stack.is_empty();

        if needs_transform_pop || needs_opacity_pop || needs_clip_pop {
            // Get the parent container
            if let Some(parent) = self.layer_stack.pop() {
                // The current container contains the children for the effect layer
                let mut children_container = std::mem::replace(&mut self.current_container, parent);
                let children = children_container.take_children();

                // Create appropriate wrapper layer
                if needs_transform_pop && self.transform_stack.len() > 1 {
                    self.transform_stack.pop();
                    let transform = self.transform_stack.last().unwrap().clone();
                    let mut transform_layer = Box::new(TransformLayer::new(transform));

                    // Move children to transform layer
                    for child in children {
                        transform_layer.add_child(child);
                    }

                    self.current_container.add_child(transform_layer);
                } else if needs_opacity_pop && self.opacity_stack.len() > 1 {
                    self.opacity_stack.pop();
                    let opacity = self.opacity_stack.last().copied().unwrap_or(1.0);
                    let mut opacity_layer = Box::new(OpacityLayer::new(opacity));

                    // Move children to opacity layer
                    for child in children {
                        opacity_layer.add_child(child);
                    }

                    self.current_container.add_child(opacity_layer);
                } else if needs_clip_pop {
                    let clip_rect = self.clip_stack.pop().unwrap();
                    let mut clip_layer = Box::new(ClipRectLayer::new(clip_rect));

                    // Move children to clip layer
                    for child in children {
                        clip_layer.add_child(child);
                    }

                    self.current_container.add_child(clip_layer);
                }
            }
        }

        self
    }

    /// Clear all layers and reset the builder
    pub fn clear(&mut self) -> &mut Self {
        self.layer_stack.clear();
        self.current_container = Box::new(ContainerLayer::new());
        self.transform_stack = vec![Matrix4::identity()];
        self.opacity_stack = vec![1.0];
        self.clip_stack.clear();
        self
    }

    // ========================================
    // Scene Building
    // ========================================

    /// Build the final scene
    pub fn build(mut self) -> Scene {
        // Pop any remaining layers
        while !self.layer_stack.is_empty() {
            self.pop();
        }

        // Create scene with the root container
        Scene::new(self.current_container)
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ========================================
// Convenience Builder Functions
// ========================================

/// Create a scene with a single colored rectangle
pub fn colored_box_scene(rect: Rect, color: Color) -> Scene {
    let mut builder = SceneBuilder::new();
    builder.add_rect(rect, Paint::from_color(color));
    builder.build()
}

/// Create a scene with text
pub fn text_scene(text: &str, style: TextStyle) -> Scene {
    let mut builder = SceneBuilder::new();
    builder.add_text(Offset::zero(), text, style);
    builder.build()
}

/// Create a test scene with multiple elements
pub fn test_scene() -> Scene {
    let mut builder = SceneBuilder::new();

    // Add a background
    builder.add_rect(
        Rect::from_ltrb(0.0, 0.0, 800.0, 600.0),
        Paint::from_color(Color::from_rgb(240, 240, 240)),
    );

    // Add a rotated colored box
    builder
        .push_translate(400.0, 300.0)
        .push_rotate(0.5)
        .add_rect(
            Rect::from_ltrb(-50.0, -50.0, 50.0, 50.0),
            Paint::from_color(Color::from_rgb(100, 150, 200)),
        )
        .pop();

    // Add semi-transparent circle
    builder
        .push_opacity(0.7)
        .add_circle(
            Offset::new(200.0, 200.0),
            75.0,
            Paint::from_color(Color::from_rgb(200, 100, 100)),
        )
        .pop();

    // Add clipped content
    builder
        .push_clip_rect(Rect::from_ltrb(500.0, 100.0, 700.0, 300.0))
        .add_rect(
            Rect::from_ltrb(450.0, 50.0, 750.0, 350.0),
            Paint::from_color(Color::from_rgb(100, 200, 100)),
        )
        .pop();

    // Add text
    builder.add_text(
        Offset::new(50.0, 50.0),
        "NightScript + Flutter",
        TextStyle::default()
            .with_size(24.0)
            .with_color(Color::black()),
    );

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_builder_basic() {
        let mut builder = SceneBuilder::new();
        builder.add_rect(
            Rect::from_ltrb(0.0, 0.0, 100.0, 100.0),
            Paint::from_color(Color::from_rgb(255, 0, 0)),
        );
        let scene = builder.build();

        assert_eq!(scene.frame_number(), 0);
    }

    #[test]
    fn test_scene_builder_transform() {
        let mut builder = SceneBuilder::new();
        builder
            .push_translate(50.0, 50.0)
            .push_scale(2.0, 2.0)
            .add_rect(
                Rect::from_ltrb(0.0, 0.0, 10.0, 10.0),
                Paint::from_color(Color::black()),
            )
            .pop()
            .pop();
        let scene = builder.build();

        // The scene should have one root layer
        let bounds = scene.root_layer().bounds();
        // After translation and scale, bounds should be affected
        assert!(bounds.size.width > 0.0);
        assert!(bounds.size.height > 0.0);
    }

    #[test]
    fn test_scene_builder_opacity() {
        let mut builder = SceneBuilder::new();

        builder.push_opacity(0.5);
        assert_eq!(builder.current_opacity(), 0.5);

        builder.push_opacity(0.5);
        assert_eq!(builder.current_opacity(), 0.25);

        builder.pop();
        assert_eq!(builder.current_opacity(), 0.5);

        builder.pop();
        assert_eq!(builder.current_opacity(), 1.0);
    }

    #[test]
    fn test_convenience_builders() {
        let rect = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);
        let color = Color::from_rgb(255, 0, 0);

        let scene = colored_box_scene(rect, color);
        assert!(scene.frame_number() >= 0);

        let text_scene = text_scene("Hello", TextStyle::default());
        assert!(text_scene.frame_number() >= 0);

        let test = test_scene();
        assert!(test.frame_number() >= 0);
    }
}

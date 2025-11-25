// OpacityLayer - A layer that applies opacity/transparency to its children
// This layer is used to make child layers semi-transparent

use super::container_layer::ContainerLayer;
use super::{Layer, Offset, PaintContext, PrerollContext, Rect};
use std::fmt::Debug;

/// A layer that applies opacity to its children
pub struct OpacityLayer {
    /// Base container functionality
    container: ContainerLayer,
    /// Opacity value (0.0 = transparent, 1.0 = opaque)
    opacity: f64,
    /// Cached alpha value (0-255)
    alpha: u8,
}

impl OpacityLayer {
    /// Create a new opacity layer with the given opacity value
    /// Opacity is clamped to the range [0.0, 1.0]
    pub fn new(opacity: f64) -> Self {
        let clamped_opacity = opacity.max(0.0).min(1.0);
        let alpha = (clamped_opacity * 255.0) as u8;

        Self {
            container: ContainerLayer::new(),
            opacity: clamped_opacity,
            alpha,
        }
    }

    /// Create a fully transparent layer
    pub fn transparent() -> Self {
        Self::new(0.0)
    }

    /// Create a fully opaque layer
    pub fn opaque() -> Self {
        Self::new(1.0)
    }

    /// Set the opacity value
    pub fn set_opacity(&mut self, opacity: f64) {
        let clamped_opacity = opacity.max(0.0).min(1.0);
        if (self.opacity - clamped_opacity).abs() > 1e-10 {
            self.opacity = clamped_opacity;
            self.alpha = (clamped_opacity * 255.0) as u8;
            self.container.mark_needs_repaint();
        }
    }

    /// Get the opacity value
    pub fn opacity(&self) -> f64 {
        self.opacity
    }

    /// Get the alpha value (0-255)
    pub fn alpha(&self) -> u8 {
        self.alpha
    }

    /// Check if this layer is fully transparent
    pub fn is_transparent(&self) -> bool {
        self.opacity <= 0.0
    }

    /// Check if this layer is fully opaque
    pub fn is_opaque(&self) -> bool {
        self.opacity >= 1.0
    }

    /// Add a child layer to have opacity applied
    pub fn add_child(&mut self, child: Box<dyn Layer>) {
        self.container.add_child(child);
    }

    /// Remove a child layer by ID
    pub fn remove_child(&mut self, child_id: u64) -> Option<Box<dyn Layer>> {
        self.container.remove_child(child_id)
    }

    /// Clear all children
    pub fn clear_children(&mut self) {
        self.container.clear_children();
    }

    /// Get the number of children
    pub fn child_count(&self) -> usize {
        self.container.child_count()
    }
}

impl Layer for OpacityLayer {
    fn preroll(&mut self, context: &PrerollContext) {
        // Skip entirely if fully transparent
        if self.is_transparent() {
            self.container.set_bounds(Rect::zero());
            return;
        }

        // If fully opaque, just preroll children normally
        if self.is_opaque() {
            self.container.preroll(context);
            return;
        }

        // Create a new context with combined opacity
        let opacity_context = context.with_opacity(self.opacity);

        // Preroll all children with the opacity context
        for child in self.container.children_mut() {
            child.preroll(&opacity_context);
        }

        // Calculate combined bounds from all children
        if self.container.child_count() == 0 {
            self.container.set_bounds(Rect::zero());
        } else {
            let mut combined_bounds = Rect::zero();
            let mut has_bounds = false;

            for child in self.container.children() {
                let child_bounds = child.bounds();
                if !child_bounds.size.is_empty() {
                    if !has_bounds {
                        combined_bounds = child_bounds;
                        has_bounds = true;
                    } else {
                        combined_bounds = combined_bounds.union(&child_bounds);
                    }
                }
            }

            self.container.set_bounds(combined_bounds);
        }

        self.container.clear_needs_repaint();
    }

    fn paint(&self, context: &PaintContext) {
        // Skip painting if fully transparent
        if self.is_transparent() {
            return;
        }

        // If fully opaque, just paint children normally
        if self.is_opaque() {
            self.container.paint(context);
            return;
        }

        let canvas = context.canvas.clone();

        // For opacity layers, we typically need to render children to an offscreen buffer
        // then composite that buffer with the opacity applied.
        // For simplicity in this implementation, we'll use save/restore layers if available,
        // or pass the opacity down to the paint context.

        // Save layer with opacity
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.save();
            // In a real implementation, this would create an offscreen buffer
            // canvas_guard.save_layer_with_opacity(self.opacity);
        }

        // Create a new context with combined opacity
        let opacity_context = context.with_opacity(self.opacity);

        // Paint all children with the opacity context
        for child in self.container.children() {
            child.paint(&opacity_context);
        }

        // Restore the canvas state
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.restore();
        }
    }

    fn bounds(&self) -> Rect {
        self.container.bounds()
    }

    fn hit_test(&self, position: &Offset) -> bool {
        // Fully transparent layers don't respond to hits
        if self.is_transparent() {
            return false;
        }

        // Test children
        self.container.hit_test(position)
    }

    fn id(&self) -> u64 {
        self.container.id()
    }

    fn needs_repaint(&self) -> bool {
        self.container.needs_repaint()
    }

    fn mark_needs_repaint(&mut self) {
        self.container.mark_needs_repaint();
    }

    fn layer_type(&self) -> &'static str {
        "OpacityLayer"
    }
}

impl Debug for OpacityLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpacityLayer")
            .field("id", &self.container.id())
            .field("opacity", &self.opacity)
            .field("alpha", &self.alpha)
            .field("bounds", &self.container.bounds())
            .field("children_count", &self.container.child_count())
            .field("needs_repaint", &self.container.needs_repaint())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flutter::layers::layer::LayerBase;
    use crate::flutter::layers::Size;

    // Mock layer for testing
    struct MockLayer {
        base: LayerBase,
        test_bounds: Rect,
    }

    impl MockLayer {
        fn new(bounds: Rect) -> Self {
            let mut base = LayerBase::new();
            base.set_bounds(bounds);
            Self {
                base,
                test_bounds: bounds,
            }
        }
    }

    impl Layer for MockLayer {
        fn preroll(&mut self, _context: &PrerollContext) {
            self.base.set_bounds(self.test_bounds);
        }

        fn paint(&self, _context: &PaintContext) {}

        fn bounds(&self) -> Rect {
            self.test_bounds
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
            "MockLayer"
        }
    }

    impl Debug for MockLayer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockLayer")
                .field("id", &self.base.id())
                .field("bounds", &self.test_bounds)
                .finish()
        }
    }

    #[test]
    fn test_opacity_layer_creation() {
        let layer = OpacityLayer::new(0.5);
        assert_eq!(layer.opacity(), 0.5);
        assert_eq!(layer.alpha(), 127);
        assert!(!layer.is_transparent());
        assert!(!layer.is_opaque());
    }

    #[test]
    fn test_opacity_clamping() {
        let layer1 = OpacityLayer::new(-0.5);
        assert_eq!(layer1.opacity(), 0.0);
        assert_eq!(layer1.alpha(), 0);
        assert!(layer1.is_transparent());

        let layer2 = OpacityLayer::new(1.5);
        assert_eq!(layer2.opacity(), 1.0);
        assert_eq!(layer2.alpha(), 255);
        assert!(layer2.is_opaque());
    }

    #[test]
    fn test_opacity_layer_with_children() {
        let mut layer = OpacityLayer::new(0.7);

        // Add a child with known bounds
        let child_bounds = Rect::from_ltrb(10.0, 10.0, 110.0, 110.0);
        let child = Box::new(MockLayer::new(child_bounds));
        layer.add_child(child);

        assert_eq!(layer.child_count(), 1);

        // Preroll to calculate bounds
        let viewport_size = Size::new(800.0, 600.0);
        let mut context = PrerollContext::new(viewport_size, 1.0);
        layer.preroll(&mut context);

        // Bounds should be the same as child bounds
        assert_eq!(layer.bounds(), child_bounds);
    }

    #[test]
    fn test_transparent_layer_behavior() {
        let mut layer = OpacityLayer::transparent();
        assert!(layer.is_transparent());
        assert_eq!(layer.opacity(), 0.0);

        // Add a child
        let child_bounds = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);
        let child = Box::new(MockLayer::new(child_bounds));
        layer.add_child(child);

        // Preroll
        let viewport_size = Size::new(800.0, 600.0);
        let mut context = PrerollContext::new(viewport_size, 1.0);
        layer.preroll(&mut context);

        // Transparent layer should have zero bounds
        assert_eq!(layer.bounds(), Rect::zero());

        // Hit test should fail on transparent layer
        assert!(!layer.hit_test(&Offset::new(50.0, 50.0)));
    }

    #[test]
    fn test_opaque_layer_behavior() {
        let mut layer = OpacityLayer::opaque();
        assert!(layer.is_opaque());
        assert_eq!(layer.opacity(), 1.0);
        assert_eq!(layer.alpha(), 255);

        // Add a child
        let child_bounds = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);
        let child = Box::new(MockLayer::new(child_bounds));
        layer.add_child(child);

        // Preroll
        let viewport_size = Size::new(800.0, 600.0);
        let mut context = PrerollContext::new(viewport_size, 1.0);
        layer.preroll(&mut context);

        // Opaque layer should have normal bounds
        assert_eq!(layer.bounds(), child_bounds);

        // Hit test should work normally
        assert!(layer.hit_test(&Offset::new(50.0, 50.0)));
    }

    #[test]
    fn test_opacity_context_propagation() {
        // Create nested opacity layers to test context propagation
        let mut outer_layer = OpacityLayer::new(0.5);
        let mut inner_layer = OpacityLayer::new(0.5);

        // The combined opacity should be 0.5 * 0.5 = 0.25
        let child_bounds = Rect::from_ltrb(0.0, 0.0, 50.0, 50.0);
        let child = Box::new(MockLayer::new(child_bounds));
        inner_layer.add_child(child);
        outer_layer.add_child(Box::new(inner_layer));

        // Preroll with opacity tracking
        let viewport_size = Size::new(800.0, 600.0);
        let mut context = PrerollContext::new(viewport_size, 1.0);
        outer_layer.preroll(&mut context);

        // Both layers should have the same bounds as the child
        assert_eq!(outer_layer.bounds(), child_bounds);
    }

    #[test]
    fn test_set_opacity() {
        let mut layer = OpacityLayer::new(0.5);
        assert_eq!(layer.opacity(), 0.5);

        layer.set_opacity(0.8);
        assert_eq!(layer.opacity(), 0.8);
        assert_eq!(layer.alpha(), 204); // 0.8 * 255 = 204

        // Test clamping
        layer.set_opacity(-1.0);
        assert_eq!(layer.opacity(), 0.0);

        layer.set_opacity(2.0);
        assert_eq!(layer.opacity(), 1.0);
    }
}

// TransformLayer - A layer that applies transformations to its children
// This layer is used to transform the coordinate system for all child layers

use super::container_layer::ContainerLayer;
use super::{Layer, Matrix4, Offset, PaintContext, PrerollContext, Rect};
use std::fmt::Debug;

/// A layer that applies a transformation matrix to its children
pub struct TransformLayer {
    /// Base container functionality
    container: ContainerLayer,
    /// Transformation matrix
    transform: Matrix4,
}

impl TransformLayer {
    /// Create a new transform layer with the given transformation matrix
    pub fn new(transform: Matrix4) -> Self {
        Self {
            container: ContainerLayer::new(),
            transform,
        }
    }

    /// Create a translation transform layer
    pub fn translation(dx: f64, dy: f64) -> Self {
        Self::new(Matrix4::translation(dx, dy))
    }

    /// Create a scale transform layer
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self::new(Matrix4::scale(sx, sy))
    }

    /// Create a rotation transform layer (in radians)
    pub fn rotation(radians: f64) -> Self {
        Self::new(Matrix4::rotation_z(radians))
    }

    /// Set the transformation matrix
    pub fn set_transform(&mut self, transform: Matrix4) {
        if self.transform != transform {
            self.transform = transform;
            self.container.mark_needs_repaint();
        }
    }

    /// Get the transformation matrix
    pub fn transform(&self) -> &Matrix4 {
        &self.transform
    }

    /// Add a child layer to be transformed
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

    /// Calculate the inverse of the transformation matrix
    fn calculate_inverse(&self) -> Option<Matrix4> {
        // Simplified inverse calculation for 2D transformations
        // For a full implementation, would need proper matrix inversion
        let m = &self.transform.data;

        // Calculate determinant for 2x2 portion
        let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];

        if det.abs() < 1e-10 {
            // Matrix is not invertible
            return None;
        }

        // Calculate inverse for 2D transformation
        let inv_det = 1.0 / det;

        let mut inverse = Matrix4::identity();
        inverse.data[0][0] = m[1][1] * inv_det;
        inverse.data[0][1] = -m[0][1] * inv_det;
        inverse.data[1][0] = -m[1][0] * inv_det;
        inverse.data[1][1] = m[0][0] * inv_det;

        // Handle translation
        inverse.data[0][3] = -(m[0][3] * inverse.data[0][0] + m[1][3] * inverse.data[0][1]);
        inverse.data[1][3] = -(m[0][3] * inverse.data[1][0] + m[1][3] * inverse.data[1][1]);

        Some(inverse)
    }
}

impl Layer for TransformLayer {
    fn preroll(&mut self, context: &PrerollContext) {
        // Create a new context with the combined transformation
        let transformed_context = context.with_transform(&self.transform);

        // Preroll all children with the transformed context
        for child in self.container.children_mut() {
            child.preroll(&transformed_context);
        }

        // Transform the combined bounds of all children
        if self.container.child_count() == 0 {
            self.container.set_bounds(Rect::zero());
        } else {
            // Get the untransformed bounds from children
            let child_bounds = self.container.bounds();

            // Transform the bounds
            let transformed_bounds = self.transform.transform_rect(&child_bounds);
            self.container.set_bounds(transformed_bounds);
        }

        self.container.clear_needs_repaint();
    }

    fn paint(&self, context: &PaintContext) {
        if self.container.child_count() == 0 {
            return;
        }

        let canvas = context.canvas.clone();

        // Save the canvas state
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.save();
            canvas_guard.transform(self.transform.clone());
        }

        // Create a new context with the transformation
        let transformed_context = context.with_transform(&self.transform);

        // Paint all children with the transformed context
        for child in self.container.children() {
            child.paint(&transformed_context);
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
        // Transform the position to the local coordinate system
        let local_position = if let Some(inverse) = self.calculate_inverse() {
            inverse.transform_offset(position)
        } else {
            // If transform is not invertible, can't hit test
            return false;
        };

        // Test against children in local coordinates
        for child in self.container.children().iter().rev() {
            if child.hit_test(&local_position) {
                return true;
            }
        }

        false
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
        "TransformLayer"
    }
}

impl Debug for TransformLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformLayer")
            .field("id", &self.container.id())
            .field("transform", &self.transform)
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
    use crate::flutter::layers::{Color, Paint, Size};

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
        fn preroll(&mut self, _context: &PrerollContext) {}

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
    fn test_transform_layer_creation() {
        let transform = Matrix4::translation(10.0, 20.0);
        let layer = TransformLayer::new(transform.clone());

        assert_eq!(layer.transform(), &transform);
        assert_eq!(layer.child_count(), 0);
        assert_eq!(layer.bounds(), Rect::zero());
    }

    #[test]
    fn test_transform_layer_translation() {
        let mut layer = TransformLayer::translation(50.0, 100.0);

        // Add a child with known bounds
        let child_bounds = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);
        let child = Box::new(MockLayer::new(child_bounds));
        layer.add_child(child);

        // Preroll to calculate transformed bounds
        let viewport_size = Size::new(800.0, 600.0);
        let mut context = PrerollContext::new(viewport_size, 1.0);
        layer.preroll(&mut context);

        // Bounds should be translated
        let expected_bounds = Rect::from_ltrb(50.0, 100.0, 150.0, 200.0);
        assert_eq!(layer.bounds(), expected_bounds);
    }

    #[test]
    fn test_transform_layer_scale() {
        let mut layer = TransformLayer::scale(2.0, 2.0);

        // Add a child with known bounds
        let child_bounds = Rect::from_ltrb(10.0, 10.0, 60.0, 60.0);
        let child = Box::new(MockLayer::new(child_bounds));
        layer.add_child(child);

        // Preroll to calculate transformed bounds
        let viewport_size = Size::new(800.0, 600.0);
        let mut context = PrerollContext::new(viewport_size, 1.0);
        layer.preroll(&mut context);

        // Bounds should be scaled
        // Scale happens from origin, so (10,10)-(60,60) becomes (20,20)-(120,120)
        let expected_bounds = Rect::from_ltrb(20.0, 20.0, 120.0, 120.0);
        assert_eq!(layer.bounds(), expected_bounds);
    }

    #[test]
    fn test_transform_layer_hit_test() {
        let mut layer = TransformLayer::translation(100.0, 100.0);

        // Add a child with known bounds
        let child_bounds = Rect::from_ltrb(0.0, 0.0, 50.0, 50.0);
        let child = Box::new(MockLayer::new(child_bounds));
        layer.add_child(child);

        // Test hit in transformed coordinates
        assert!(layer.hit_test(&Offset::new(125.0, 125.0))); // Inside transformed bounds
        assert!(!layer.hit_test(&Offset::new(25.0, 25.0))); // Outside transformed bounds
    }

    #[test]
    fn test_transform_layer_rotation() {
        let radians = std::f64::consts::PI / 4.0; // 45 degrees
        let layer = TransformLayer::rotation(radians);

        let transform = layer.transform();
        let cos = radians.cos();
        let sin = radians.sin();

        // Check rotation matrix values
        assert!((transform.data[0][0] - cos).abs() < 1e-10);
        assert!((transform.data[0][1] - (-sin)).abs() < 1e-10);
        assert!((transform.data[1][0] - sin).abs() < 1e-10);
        assert!((transform.data[1][1] - cos).abs() < 1e-10);
    }

    #[test]
    fn test_transform_layer_inverse() {
        // Test translation inverse
        let transform = TransformLayer::translation(10.0, 20.0);
        let inverse = transform.calculate_inverse().unwrap();

        let point = Offset::new(100.0, 200.0);
        let transformed = transform.transform.transform_offset(&point);
        let back = inverse.transform_offset(&transformed);

        assert!((back.x - point.x).abs() < 1e-10);
        assert!((back.y - point.y).abs() < 1e-10);
    }
}

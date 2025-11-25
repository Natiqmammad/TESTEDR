// ClipLayer implementations for various clipping shapes
// These layers clip their children to specific shapes like rectangles, rounded rectangles, and paths

use super::container_layer::ContainerLayer;
use super::{Layer, Offset, PaintContext, Path, PrerollContext, Rect};
use std::fmt::Debug;

/// Base trait for all clip layers
pub trait ClipLayer: Layer {
    /// Check if a point is within the clip region
    fn contains_point(&self, point: &Offset) -> bool;

    /// Get the clip bounds
    fn clip_bounds(&self) -> Rect;
}

/// A layer that clips its children to a rectangular shape
pub struct ClipRectLayer {
    /// Base container functionality
    container: ContainerLayer,
    /// Clipping rectangle
    clip_rect: Rect,
    /// Clip behavior when content overflows
    clip_behavior: ClipBehavior,
}

/// Clip behavior options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipBehavior {
    /// No clipping
    None,
    /// Clip to bounds, but allow anti-aliasing
    AntiAlias,
    /// Clip to bounds with anti-aliasing and save layer
    AntiAliasWithSaveLayer,
    /// Hard clip without anti-aliasing
    HardEdge,
}

impl Default for ClipBehavior {
    fn default() -> Self {
        Self::AntiAlias
    }
}

impl ClipRectLayer {
    /// Create a new clip rect layer
    pub fn new(clip_rect: Rect) -> Self {
        Self::with_behavior(clip_rect, ClipBehavior::default())
    }

    /// Create a new clip rect layer with specific clip behavior
    pub fn with_behavior(clip_rect: Rect, clip_behavior: ClipBehavior) -> Self {
        Self {
            container: ContainerLayer::new(),
            clip_rect,
            clip_behavior,
        }
    }

    /// Set the clipping rectangle
    pub fn set_clip_rect(&mut self, clip_rect: Rect) {
        if self.clip_rect != clip_rect {
            self.clip_rect = clip_rect;
            self.container.mark_needs_repaint();
        }
    }

    /// Get the clipping rectangle
    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    /// Set the clip behavior
    pub fn set_clip_behavior(&mut self, clip_behavior: ClipBehavior) {
        if self.clip_behavior != clip_behavior {
            self.clip_behavior = clip_behavior;
            self.container.mark_needs_repaint();
        }
    }

    /// Get the clip behavior
    pub fn clip_behavior(&self) -> ClipBehavior {
        self.clip_behavior
    }

    /// Add a child to be clipped
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

impl Layer for ClipRectLayer {
    fn preroll(&mut self, context: &PrerollContext) {
        // Skip if clip behavior is None
        if self.clip_behavior == ClipBehavior::None {
            self.container.preroll(context);
            return;
        }

        // Preroll children first
        self.container.preroll(context);

        // Intersect child bounds with clip rect
        let child_bounds = self.container.bounds();
        if let Some(clipped_bounds) = child_bounds.intersection(&self.clip_rect) {
            self.container.set_bounds(clipped_bounds);
        } else {
            // No intersection means nothing will be visible
            self.container.set_bounds(Rect::zero());
        }
    }

    fn paint(&self, context: &PaintContext) {
        // Skip if clip behavior is None or no children
        if self.clip_behavior == ClipBehavior::None || self.container.child_count() == 0 {
            self.container.paint(context);
            return;
        }

        let canvas = context.canvas.clone();

        // Save the canvas state and apply clipping
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.save();
            canvas_guard.clip_rect(self.clip_rect);
        }

        // Paint children within the clip
        self.container.paint(context);

        // Restore canvas state
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.restore();
        }
    }

    fn bounds(&self) -> Rect {
        self.container.bounds()
    }

    fn hit_test(&self, position: &Offset) -> bool {
        // First check if position is within clip rect
        if self.clip_behavior != ClipBehavior::None && !self.clip_rect.contains(position) {
            return false;
        }
        // Then check children
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
        "ClipRectLayer"
    }
}

impl ClipLayer for ClipRectLayer {
    fn contains_point(&self, point: &Offset) -> bool {
        self.clip_rect.contains(point)
    }

    fn clip_bounds(&self) -> Rect {
        self.clip_rect
    }
}

impl Debug for ClipRectLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipRectLayer")
            .field("id", &self.container.id())
            .field("clip_rect", &self.clip_rect)
            .field("clip_behavior", &self.clip_behavior)
            .field("bounds", &self.container.bounds())
            .field("children_count", &self.container.child_count())
            .finish()
    }
}

/// A rounded rectangle for clipping
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RRect {
    /// The bounding rectangle
    pub rect: Rect,
    /// The corner radius (uniform for all corners)
    pub radius: f64,
}

impl RRect {
    /// Create a new rounded rectangle
    pub fn new(rect: Rect, radius: f64) -> Self {
        Self { rect, radius }
    }

    /// Check if a point is inside the rounded rectangle
    pub fn contains(&self, point: &Offset) -> bool {
        // First check if point is outside the bounding rect
        if !self.rect.contains(point) {
            return false;
        }

        // If radius is zero, it's just a regular rectangle
        if self.radius <= 0.0 {
            return true;
        }

        // Check corners
        let dx = point.x - self.rect.offset.x;
        let dy = point.y - self.rect.offset.y;
        let right = self.rect.size.width;
        let bottom = self.rect.size.height;

        // Top-left corner
        if dx < self.radius && dy < self.radius {
            let corner_dx = dx - self.radius;
            let corner_dy = dy - self.radius;
            if corner_dx * corner_dx + corner_dy * corner_dy > self.radius * self.radius {
                return false;
            }
        }

        // Top-right corner
        if dx > right - self.radius && dy < self.radius {
            let corner_dx = dx - (right - self.radius);
            let corner_dy = dy - self.radius;
            if corner_dx * corner_dx + corner_dy * corner_dy > self.radius * self.radius {
                return false;
            }
        }

        // Bottom-left corner
        if dx < self.radius && dy > bottom - self.radius {
            let corner_dx = dx - self.radius;
            let corner_dy = dy - (bottom - self.radius);
            if corner_dx * corner_dx + corner_dy * corner_dy > self.radius * self.radius {
                return false;
            }
        }

        // Bottom-right corner
        if dx > right - self.radius && dy > bottom - self.radius {
            let corner_dx = dx - (right - self.radius);
            let corner_dy = dy - (bottom - self.radius);
            if corner_dx * corner_dx + corner_dy * corner_dy > self.radius * self.radius {
                return false;
            }
        }

        true
    }
}

/// A layer that clips its children to a rounded rectangle
pub struct ClipRRectLayer {
    /// Base container functionality
    container: ContainerLayer,
    /// Clipping rounded rectangle
    clip_rrect: RRect,
    /// Clip behavior when content overflows
    clip_behavior: ClipBehavior,
}

impl ClipRRectLayer {
    /// Create a new clip rounded rect layer
    pub fn new(clip_rrect: RRect) -> Self {
        Self::with_behavior(clip_rrect, ClipBehavior::default())
    }

    /// Create a new clip rounded rect layer with specific clip behavior
    pub fn with_behavior(clip_rrect: RRect, clip_behavior: ClipBehavior) -> Self {
        Self {
            container: ContainerLayer::new(),
            clip_rrect,
            clip_behavior,
        }
    }

    /// Set the clipping rounded rectangle
    pub fn set_clip_rrect(&mut self, clip_rrect: RRect) {
        if self.clip_rrect != clip_rrect {
            self.clip_rrect = clip_rrect;
            self.container.mark_needs_repaint();
        }
    }

    /// Get the clipping rounded rectangle
    pub fn clip_rrect(&self) -> RRect {
        self.clip_rrect
    }

    /// Add a child to be clipped
    pub fn add_child(&mut self, child: Box<dyn Layer>) {
        self.container.add_child(child);
    }
}

impl Layer for ClipRRectLayer {
    fn preroll(&mut self, context: &PrerollContext) {
        // Skip if clip behavior is None
        if self.clip_behavior == ClipBehavior::None {
            self.container.preroll(context);
            return;
        }

        // Preroll children first
        self.container.preroll(context);

        // Use the bounding rect of the rounded rect for bounds calculation
        let child_bounds = self.container.bounds();
        if let Some(clipped_bounds) = child_bounds.intersection(&self.clip_rrect.rect) {
            self.container.set_bounds(clipped_bounds);
        } else {
            self.container.set_bounds(Rect::zero());
        }
    }

    fn paint(&self, context: &PaintContext) {
        // Skip if clip behavior is None or no children
        if self.clip_behavior == ClipBehavior::None || self.container.child_count() == 0 {
            self.container.paint(context);
            return;
        }

        let canvas = context.canvas.clone();

        // Save the canvas state and apply clipping
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.save();
            // In a real implementation, this would clip to a rounded rectangle
            // For now, we clip to the bounding rect
            canvas_guard.clip_rect(self.clip_rrect.rect);
        }

        // Paint children within the clip
        self.container.paint(context);

        // Restore canvas state
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.restore();
        }
    }

    fn bounds(&self) -> Rect {
        self.container.bounds()
    }

    fn hit_test(&self, position: &Offset) -> bool {
        // First check if position is within rounded rect
        if self.clip_behavior != ClipBehavior::None && !self.clip_rrect.contains(position) {
            return false;
        }
        // Then check children
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
        "ClipRRectLayer"
    }
}

impl ClipLayer for ClipRRectLayer {
    fn contains_point(&self, point: &Offset) -> bool {
        self.clip_rrect.contains(point)
    }

    fn clip_bounds(&self) -> Rect {
        self.clip_rrect.rect
    }
}

impl Debug for ClipRRectLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipRRectLayer")
            .field("id", &self.container.id())
            .field("clip_rrect", &self.clip_rrect)
            .field("clip_behavior", &self.clip_behavior)
            .field("bounds", &self.container.bounds())
            .field("children_count", &self.container.child_count())
            .finish()
    }
}

/// A layer that clips its children to a path
pub struct ClipPathLayer {
    /// Base container functionality
    container: ContainerLayer,
    /// Clipping path
    clip_path: Path,
    /// Clip behavior when content overflows
    clip_behavior: ClipBehavior,
}

impl ClipPathLayer {
    /// Create a new clip path layer
    pub fn new(clip_path: Path) -> Self {
        Self::with_behavior(clip_path, ClipBehavior::default())
    }

    /// Create a new clip path layer with specific clip behavior
    pub fn with_behavior(clip_path: Path, clip_behavior: ClipBehavior) -> Self {
        Self {
            container: ContainerLayer::new(),
            clip_path,
            clip_behavior,
        }
    }

    /// Set the clipping path
    pub fn set_clip_path(&mut self, clip_path: Path) {
        self.clip_path = clip_path;
        self.container.mark_needs_repaint();
    }

    /// Get the clipping path
    pub fn clip_path(&self) -> &Path {
        &self.clip_path
    }

    /// Add a child to be clipped
    pub fn add_child(&mut self, child: Box<dyn Layer>) {
        self.container.add_child(child);
    }
}

impl Layer for ClipPathLayer {
    fn preroll(&mut self, context: &PrerollContext) {
        // Skip if clip behavior is None
        if self.clip_behavior == ClipBehavior::None {
            self.container.preroll(context);
            return;
        }

        // Preroll children first
        self.container.preroll(context);

        // Use the path bounds for clipping
        if let Some(path_bounds) = self.clip_path.bounds() {
            let child_bounds = self.container.bounds();
            if let Some(clipped_bounds) = child_bounds.intersection(&path_bounds) {
                self.container.set_bounds(clipped_bounds);
            } else {
                self.container.set_bounds(Rect::zero());
            }
        }
    }

    fn paint(&self, context: &PaintContext) {
        // Skip if clip behavior is None or no children
        if self.clip_behavior == ClipBehavior::None || self.container.child_count() == 0 {
            self.container.paint(context);
            return;
        }

        let canvas = context.canvas.clone();

        // Save the canvas state and apply clipping
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.save();
            // In a real implementation, this would clip to the path
            // For now, we use the path bounds
            if let Some(bounds) = self.clip_path.bounds() {
                canvas_guard.clip_rect(bounds);
            }
        }

        // Paint children within the clip
        self.container.paint(context);

        // Restore canvas state
        {
            let mut canvas_guard = canvas.write().unwrap();
            canvas_guard.restore();
        }
    }

    fn bounds(&self) -> Rect {
        self.container.bounds()
    }

    fn hit_test(&self, position: &Offset) -> bool {
        // For simplicity, use path bounds for hit testing
        if self.clip_behavior != ClipBehavior::None {
            if let Some(bounds) = self.clip_path.bounds() {
                if !bounds.contains(position) {
                    return false;
                }
            }
        }
        // Then check children
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
        "ClipPathLayer"
    }
}

impl Debug for ClipPathLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipPathLayer")
            .field("id", &self.container.id())
            .field("clip_behavior", &self.clip_behavior)
            .field("bounds", &self.container.bounds())
            .field("children_count", &self.container.child_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flutter::layers::Size;

    #[test]
    fn test_clip_rect_layer() {
        let clip_rect = Rect::from_ltrb(10.0, 10.0, 90.0, 90.0);
        let mut layer = ClipRectLayer::new(clip_rect);

        assert_eq!(layer.clip_rect(), clip_rect);
        assert_eq!(layer.clip_behavior(), ClipBehavior::AntiAlias);

        // Test hit testing
        assert!(layer.contains_point(&Offset::new(50.0, 50.0)));
        assert!(!layer.contains_point(&Offset::new(5.0, 5.0)));
        assert!(!layer.contains_point(&Offset::new(95.0, 95.0)));
    }

    #[test]
    fn test_rounded_rect_contains() {
        let rect = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);
        let rrect = RRect::new(rect, 10.0);

        // Center point should be inside
        assert!(rrect.contains(&Offset::new(50.0, 50.0)));

        // Points near corners but outside radius should be outside
        assert!(!rrect.contains(&Offset::new(2.0, 2.0)));
        assert!(!rrect.contains(&Offset::new(98.0, 2.0)));
        assert!(!rrect.contains(&Offset::new(2.0, 98.0)));
        assert!(!rrect.contains(&Offset::new(98.0, 98.0)));

        // Points just inside the rounded corners
        assert!(rrect.contains(&Offset::new(10.0, 10.0)));
        assert!(rrect.contains(&Offset::new(90.0, 10.0)));
        assert!(rrect.contains(&Offset::new(10.0, 90.0)));
        assert!(rrect.contains(&Offset::new(90.0, 90.0)));

        // Points outside the rect
        assert!(!rrect.contains(&Offset::new(-5.0, 50.0)));
        assert!(!rrect.contains(&Offset::new(105.0, 50.0)));
        assert!(!rrect.contains(&Offset::new(50.0, -5.0)));
        assert!(!rrect.contains(&Offset::new(50.0, 105.0)));
    }

    #[test]
    fn test_clip_behavior() {
        let clip_rect = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);
        let mut layer = ClipRectLayer::with_behavior(clip_rect, ClipBehavior::None);

        assert_eq!(layer.clip_behavior(), ClipBehavior::None);

        layer.set_clip_behavior(ClipBehavior::HardEdge);
        assert_eq!(layer.clip_behavior(), ClipBehavior::HardEdge);
    }
}

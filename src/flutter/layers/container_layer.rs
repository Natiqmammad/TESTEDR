// Container layer that can hold multiple child layers
// This is the fundamental compositing layer for building complex visual hierarchies

use super::layer::*;
use std::fmt::Debug;

/// A layer that contains multiple child layers
/// This is used to group related visual elements together
pub struct ContainerLayer {
    /// Base layer properties
    base: LayerBase,
    /// Child layers
    children: Vec<Box<dyn Layer>>,
}

impl ContainerLayer {
    /// Create a new empty container layer
    pub fn new() -> Self {
        Self {
            base: LayerBase::new(),
            children: Vec::new(),
        }
    }

    /// Add a child layer to this container
    pub fn add_child(&mut self, child: Box<dyn Layer>) {
        self.children.push(child);
        self.base.mark_needs_repaint();
    }

    /// Remove a child layer by its ID
    pub fn remove_child(&mut self, child_id: u64) -> Option<Box<dyn Layer>> {
        if let Some(index) = self.children.iter().position(|c| c.id() == child_id) {
            self.base.mark_needs_repaint();
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    /// Clear all children
    pub fn clear_children(&mut self) {
        self.children.clear();
        self.base.mark_needs_repaint();
    }

    /// Get the number of children
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Get a reference to the children
    pub fn children(&self) -> &[Box<dyn Layer>] {
        &self.children
    }

    /// Get a mutable reference to the children
    pub fn children_mut(&mut self) -> &mut Vec<Box<dyn Layer>> {
        &mut self.children
    }

    /// Find a child by ID
    pub fn find_child(&self, child_id: u64) -> Option<&dyn Layer> {
        self.children
            .iter()
            .find(|c| c.id() == child_id)
            .map(|c| c.as_ref())
    }

    /// Find a child mutably by ID
    pub fn find_child_mut(&mut self, child_id: u64) -> Option<&mut Box<dyn Layer>> {
        self.children.iter_mut().find(|c| c.id() == child_id)
    }

    /// Update the cached bounds for this layer
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.base.set_bounds(bounds);
    }

    /// Clear the repaint flag manually
    pub fn clear_needs_repaint(&mut self) {
        self.base.clear_needs_repaint();
    }

    /// Take ownership of all children, leaving the container empty
    pub fn take_children(&mut self) -> Vec<Box<dyn Layer>> {
        std::mem::take(&mut self.children)
    }
}

impl Layer for ContainerLayer {
    fn preroll(&mut self, context: &PrerollContext) {
        // First preroll all children
        for child in &mut self.children {
            child.preroll(context);
        }

        // Calculate combined bounds from all children
        if self.children.is_empty() {
            self.base.set_bounds(Rect::zero());
        } else {
            let mut combined_bounds = self.children[0].bounds();
            for child in &self.children[1..] {
                let child_bounds = child.bounds();
                if !child_bounds.size.is_empty() {
                    if combined_bounds.size.is_empty() {
                        combined_bounds = child_bounds;
                    } else {
                        combined_bounds = combined_bounds.union(&child_bounds);
                    }
                }
            }
            self.base.set_bounds(combined_bounds);
        }

        // Clear repaint flag after preroll
        self.base.clear_needs_repaint();
    }

    fn paint(&self, context: &PaintContext) {
        // Paint all children in order
        for child in &self.children {
            child.paint(context);
        }
    }

    fn bounds(&self) -> Rect {
        self.base.bounds()
    }

    fn hit_test(&self, position: &Offset) -> bool {
        // Check if any child is hit
        for child in self.children.iter().rev() {
            if child.hit_test(position) {
                return true;
            }
        }
        false
    }

    fn id(&self) -> u64 {
        self.base.id()
    }

    fn needs_repaint(&self) -> bool {
        // Container needs repaint if itself or any child needs repaint
        if self.base.needs_repaint() {
            return true;
        }
        self.children.iter().any(|child| child.needs_repaint())
    }

    fn mark_needs_repaint(&mut self) {
        self.base.mark_needs_repaint();
    }

    fn layer_type(&self) -> &'static str {
        "ContainerLayer"
    }
}

impl Default for ContainerLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ContainerLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerLayer")
            .field("id", &self.base.id())
            .field("bounds", &self.base.bounds())
            .field("children_count", &self.children.len())
            .field("needs_repaint", &self.base.needs_repaint())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_layer_creation() {
        let container = ContainerLayer::new();
        assert_eq!(container.child_count(), 0);
        assert_eq!(container.bounds(), Rect::zero());
    }

    #[test]
    fn test_add_remove_children() {
        let mut container = ContainerLayer::new();

        // Create mock child layer
        struct MockLayer {
            base: LayerBase,
        }

        impl Layer for MockLayer {
            fn preroll(&mut self, _context: &PrerollContext) {}
            fn paint(&self, _context: &PaintContext) {}
            fn bounds(&self) -> Rect {
                Rect::from_ltrb(0.0, 0.0, 100.0, 100.0)
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
                f.debug_struct("MockLayer").finish()
            }
        }

        let child = Box::new(MockLayer {
            base: LayerBase::new(),
        });
        let child_id = child.id();

        container.add_child(child);
        assert_eq!(container.child_count(), 1);

        let removed = container.remove_child(child_id);
        assert!(removed.is_some());
        assert_eq!(container.child_count(), 0);
    }
}

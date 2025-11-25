// Core Layer trait and base types for Flutter rendering system
// This file defines the fundamental abstraction for all visual layers in the Flutter engine

use super::canvas::Canvas;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

/// 2D offset in logical pixels
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

impl Offset {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn translate(&self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn distance(&self, other: &Offset) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// 2D size in logical pixels
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    pub fn contains(&self, offset: &Offset) -> bool {
        offset.x >= 0.0 && offset.x < self.width && offset.y >= 0.0 && offset.y < self.height
    }
}

/// Rectangle defined by offset and size
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub offset: Offset,
    pub size: Size,
}

impl Rect {
    pub fn new(offset: Offset, size: Size) -> Self {
        Self { offset, size }
    }

    pub fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            offset: Offset::new(left, top),
            size: Size::new(right - left, bottom - top),
        }
    }

    pub fn zero() -> Self {
        Self {
            offset: Offset::zero(),
            size: Size::zero(),
        }
    }

    pub fn left(&self) -> f64 {
        self.offset.x
    }

    pub fn top(&self) -> f64 {
        self.offset.y
    }

    pub fn right(&self) -> f64 {
        self.offset.x + self.size.width
    }

    pub fn bottom(&self) -> f64 {
        self.offset.y + self.size.height
    }

    pub fn contains(&self, point: &Offset) -> bool {
        point.x >= self.left()
            && point.x < self.right()
            && point.y >= self.top()
            && point.y < self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.left() < other.right()
            && other.left() < self.right()
            && self.top() < other.bottom()
            && other.top() < self.bottom()
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let left = self.left().min(other.left());
        let top = self.top().min(other.top());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::from_ltrb(left, top, right, bottom)
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let left = self.left().max(other.left());
        let top = self.top().max(other.top());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Some(Rect::from_ltrb(left, top, right, bottom))
    }

    pub fn expand(&self, delta: f64) -> Rect {
        Rect::from_ltrb(
            self.left() - delta,
            self.top() - delta,
            self.right() + delta,
            self.bottom() + delta,
        )
    }

    pub fn translate(&self, offset: &Offset) -> Rect {
        Rect::new(self.offset.translate(offset.x, offset.y), self.size)
    }
}

/// 4x4 transformation matrix for 2D and 3D transformations
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix4 {
    pub data: [[f64; 4]; 4],
}

impl Matrix4 {
    pub fn identity() -> Self {
        Self {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn translation(dx: f64, dy: f64) -> Self {
        Self {
            data: [
                [1.0, 0.0, 0.0, dx],
                [0.0, 1.0, 0.0, dy],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            data: [
                [sx, 0.0, 0.0, 0.0],
                [0.0, sy, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_z(radians: f64) -> Self {
        let cos = radians.cos();
        let sin = radians.sin();
        Self {
            data: [
                [cos, -sin, 0.0, 0.0],
                [sin, cos, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn multiply(&self, other: &Matrix4) -> Matrix4 {
        let mut result = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += self.data[i][k] * other.data[k][j];
                }
            }
        }
        Matrix4 { data: result }
    }

    pub fn transform_offset(&self, offset: &Offset) -> Offset {
        let x = self.data[0][0] * offset.x + self.data[0][1] * offset.y + self.data[0][3];
        let y = self.data[1][0] * offset.x + self.data[1][1] * offset.y + self.data[1][3];
        Offset::new(x, y)
    }

    pub fn transform_rect(&self, rect: &Rect) -> Rect {
        let top_left = self.transform_offset(&rect.offset);
        let bottom_right = self.transform_offset(&Offset::new(rect.right(), rect.bottom()));
        Rect::from_ltrb(
            top_left.x.min(bottom_right.x),
            top_left.y.min(bottom_right.y),
            top_left.x.max(bottom_right.x),
            top_left.y.max(bottom_right.y),
        )
    }
}

impl Default for Matrix4 {
    fn default() -> Self {
        Self::identity()
    }
}

/// Context for the preroll phase where layers calculate their bounds
pub struct PrerollContext {
    /// Size of the viewport in logical pixels
    pub viewport_size: Size,
    /// Device pixel ratio (physical pixels per logical pixel)
    pub device_pixel_ratio: f64,
    /// Current transformation matrix
    pub transform: Matrix4,
    /// Current opacity (0.0 to 1.0)
    pub opacity: f64,
    /// Whether this subtree is being cached
    pub is_cached: bool,
}

impl PrerollContext {
    pub fn new(viewport_size: Size, device_pixel_ratio: f64) -> Self {
        Self {
            viewport_size,
            device_pixel_ratio,
            transform: Matrix4::identity(),
            opacity: 1.0,
            is_cached: false,
        }
    }

    pub fn with_transform(&self, transform: &Matrix4) -> Self {
        Self {
            viewport_size: self.viewport_size,
            device_pixel_ratio: self.device_pixel_ratio,
            transform: self.transform.multiply(transform),
            opacity: self.opacity,
            is_cached: self.is_cached,
        }
    }

    pub fn with_opacity(&self, opacity: f64) -> Self {
        Self {
            viewport_size: self.viewport_size,
            device_pixel_ratio: self.device_pixel_ratio,
            transform: self.transform.clone(),
            opacity: self.opacity * opacity,
            is_cached: self.is_cached,
        }
    }
}

/// Context for the paint phase where layers draw their content
pub struct PaintContext {
    /// Canvas to paint on
    pub canvas: Arc<RwLock<Canvas>>,
    /// Current transformation matrix
    pub transform: Matrix4,
    /// Current opacity (0.0 to 1.0)
    pub opacity: f64,
}

impl PaintContext {
    pub fn new(canvas: Arc<RwLock<Canvas>>) -> Self {
        Self {
            canvas,
            transform: Matrix4::identity(),
            opacity: 1.0,
        }
    }

    pub fn with_transform(&self, transform: &Matrix4) -> Self {
        Self {
            canvas: self.canvas.clone(),
            transform: self.transform.multiply(transform),
            opacity: self.opacity,
        }
    }

    pub fn with_opacity(&self, opacity: f64) -> Self {
        Self {
            canvas: self.canvas.clone(),
            transform: self.transform.clone(),
            opacity: self.opacity * opacity,
        }
    }
}

/// Core trait for all layer types in the rendering pipeline
pub trait Layer: Send + Sync + Debug {
    /// Called before painting to calculate bounds and prepare resources
    fn preroll(&mut self, context: &PrerollContext);

    /// Paint the layer's content
    fn paint(&self, context: &PaintContext);

    /// Get the bounds of this layer in logical pixels
    fn bounds(&self) -> Rect;

    /// Check if a point hits this layer
    fn hit_test(&self, position: &Offset) -> bool {
        self.bounds().contains(position)
    }

    /// Get a unique identifier for this layer
    fn id(&self) -> u64;

    /// Check if this layer needs repainting
    fn needs_repaint(&self) -> bool;

    /// Mark this layer as needing repaint
    fn mark_needs_repaint(&mut self);

    /// Get the layer's type name for debugging
    fn layer_type(&self) -> &'static str;
}

/// Base implementation for common layer functionality
pub struct LayerBase {
    /// Unique identifier for this layer
    id: u64,
    /// Bounds of this layer
    bounds: Rect,
    /// Whether this layer needs repainting
    needs_repaint: bool,
}

impl LayerBase {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            bounds: Rect::zero(),
            needs_repaint: true,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.needs_repaint = true;
        }
    }

    pub fn needs_repaint(&self) -> bool {
        self.needs_repaint
    }

    pub fn mark_needs_repaint(&mut self) {
        self.needs_repaint = true;
    }

    pub fn clear_needs_repaint(&mut self) {
        self.needs_repaint = false;
    }
}

impl Default for LayerBase {
    fn default() -> Self {
        Self::new()
    }
}

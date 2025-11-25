// Flutter Layer System for NightScript
// This module implements the layer tree structure used by Flutter Engine
// to represent the visual hierarchy of the UI.

pub mod canvas;
pub mod clip_layer;
pub mod container_layer;
pub mod layer;
pub mod opacity_layer;
pub mod picture_layer;
pub mod transform_layer;

// Re-export all layer types
pub use canvas::*;
pub use clip_layer::*;
pub use container_layer::*;
pub use layer::*;
pub use opacity_layer::*;
pub use picture_layer::*;
pub use transform_layer::*;

// The layer system is the core abstraction for Flutter's rendering pipeline.
// Each layer represents a visual element that can be composited.
//
// Architecture:
// - Layer: Base trait for all layer types
// - ContainerLayer: Can contain child layers
// - PictureLayer: Contains drawing commands
// - TransformLayer: Applies transformations
// - OpacityLayer: Applies transparency
// - ClipLayer: Clips children to a shape

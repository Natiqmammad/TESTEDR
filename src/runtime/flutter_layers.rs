//! Layer tree primitives for the Flutter bridge.
//! Provides base layer structures (container, picture, transform) and helper APIs
//! to compose them. This acts as the target representation for widgets produced
//! by the AFNS VM before they are rendered or passed to the real Flutter engine.

use std::fmt;

/// Result type for engine-specific operations.
pub type EngineResult<T> = Result<T, EngineError>;

/// Errors reported while building the layer tree.
#[derive(Debug)]
pub enum EngineError {
    InvalidWidget(String),
    EmptyScene,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::InvalidWidget(msg) => write!(f, "invalid widget: {msg}"),
            EngineError::EmptyScene => write!(f, "scene builder produced no layers"),
        }
    }
}

/// Root layer type that the renderer consumes.
#[derive(Clone, Debug)]
pub enum Layer {
    Container(ContainerLayer),
    Picture(PictureLayer),
    Transform(TransformLayer),
}

impl Layer {
    pub fn id(&self) -> &str {
        match self {
            Layer::Container(layer) => &layer.id,
            Layer::Picture(layer) => &layer.id,
            Layer::Transform(layer) => &layer.id,
        }
    }
}

/// A layer that contains other layers.
#[derive(Clone, Debug)]
pub struct ContainerLayer {
    pub id: String,
    pub children: Vec<Layer>,
}

/// A layer that represents a drawing command.
#[derive(Clone, Debug)]
pub struct PictureLayer {
    pub id: String,
    pub description: String,
    pub opacity: f32,
}

/// A layer that applies a transform to its children.
#[derive(Clone, Debug)]
pub struct TransformLayer {
    pub id: String,
    pub matrix: [f32; 16],
    pub child: Box<Layer>,
}

/// A built scene with a root layer.
#[derive(Clone, Debug)]
pub struct Scene {
    pub root: Layer,
}

impl Scene {
    pub fn new(root: Layer) -> Self {
        Self { root }
    }
}

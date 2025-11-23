//! Scene builder API that mimics Flutter's `SceneBuilder`.

use crate::runtime::flutter_layers::{
    ContainerLayer, EngineError, EngineResult, Layer, PictureLayer, Scene, TransformLayer,
};

/// Represents a node on the build stack.
enum BuilderEntry {
    Container(ContainerLayer),
    Transform(TransformLayer),
}

/// High level scene builder.
pub struct SceneBuilder {
    stack: Vec<BuilderEntry>,
    opacity_stack: Vec<f32>,
}

impl SceneBuilder {
    pub fn new(root_id: &str) -> Self {
        let root = ContainerLayer {
            id: root_id.to_string(),
            children: Vec::new(),
        };
        Self {
            stack: vec![BuilderEntry::Container(root)],
            opacity_stack: vec![1.0],
        }
    }

    /// Pushes a transform layer.
    pub fn push_transform(&mut self, id: &str, matrix: [f32; 16]) {
        let transform = TransformLayer {
            id: id.to_string(),
            matrix,
            child: Box::new(Layer::Container(ContainerLayer {
                id: format!("{id}_container"),
                children: Vec::new(),
            })),
        };
        self.stack.push(BuilderEntry::Transform(transform));
    }

    /// Pushes an opacity layer.
    pub fn push_opacity(&mut self, opacity: f32) {
        let current = *self.opacity_stack.last().unwrap_or(&1.0);
        self.opacity_stack.push(current * opacity);
    }

    /// Pops the most recent entry (opacity or layer).
    pub fn pop(&mut self) {
        if self.opacity_stack.len() > 1 {
            self.opacity_stack.pop();
        } else if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    /// Adds a picture layer to the current container.
    pub fn add_picture(&mut self, id: &str, description: &str) {
        let opacity = *self.opacity_stack.last().unwrap_or(&1.0);
        let layer = Layer::Picture(PictureLayer {
            id: id.to_string(),
            description: description.to_string(),
            opacity,
        });
        self.append_layer(layer);
    }

    fn append_layer(&mut self, layer: Layer) {
        if let Some(entry) = self.stack.last_mut() {
            match entry {
                BuilderEntry::Container(container) => container.children.push(layer),
                BuilderEntry::Transform(transform) => {
                    let container = match *transform.child {
                        Layer::Container(ref mut container) => container,
                        _ => unreachable!(),
                    };
                    container.children.push(layer);
                }
            }
        }
    }

    /// Finalizes the scene.
    pub fn build(mut self) -> EngineResult<Scene> {
        while self.stack.len() > 1 {
            if let Some(entry) = self.stack.pop() {
                match entry {
                    BuilderEntry::Transform(transform) => {
                        self.append_layer(Layer::Transform(transform));
                    }
                    BuilderEntry::Container(container) => {
                        self.append_layer(Layer::Container(container));
                    }
                }
            }
        }

        if let Some(BuilderEntry::Container(root)) = self.stack.pop() {
            Ok(Scene::new(Layer::Container(root)))
        } else {
            Err(EngineError::EmptyScene)
        }
    }
}

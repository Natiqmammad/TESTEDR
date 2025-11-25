// Core Flutter integration modules for NightScript.
// This namespace exposes the layer system, SceneBuilder, embedder bindings,
// and the renderer loop that will eventually drive the real Flutter engine.

pub mod embedder;
pub mod layers;
pub mod renderer;
pub mod scene_builder;

pub use embedder::*;
pub use layers::*;
pub use renderer::*;
pub use scene_builder::*;

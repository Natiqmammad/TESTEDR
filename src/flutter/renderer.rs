//! Render loop that bridges NightScript scenes with the Flutter embedder.

use super::embedder::{EmbedderError, FlutterEmbedder};
use super::scene_builder::Scene;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Drives the Flutter engine by consuming Scenes produced by the VM.
pub struct Renderer {
    embedder: Arc<FlutterEmbedder>,
    scene_source: Arc<Mutex<Option<Scene>>>,
    target_fps: u32,
}

impl Renderer {
    pub fn new(embedder: Arc<FlutterEmbedder>, scene_source: Arc<Mutex<Option<Scene>>>) -> Self {
        Self {
            embedder,
            scene_source,
            target_fps: 60,
        }
    }

    /// Start the render loop on a dedicated thread.
    pub fn start(&self) -> RenderLoopHandle {
        let embedder = self.embedder.clone();
        let scenes = self.scene_source.clone();
        let fps = self.target_fps;
        let running = Arc::new(AtomicBool::new(true));
        let running_thread = running.clone();

        let handle = thread::spawn(move || {
            let frame_interval = Duration::from_micros(1_000_000 / fps as u64);
            let mut last_frame = Instant::now();

            while running_thread.load(Ordering::Acquire) {
                let now = Instant::now();
                if now.duration_since(last_frame) >= frame_interval {
                    if let Err(err) = render_once(&embedder, &scenes) {
                        eprintln!("[Renderer] Failed to present scene: {err}");
                    }
                    last_frame = now;
                }

                let elapsed = now.duration_since(last_frame);
                if elapsed < frame_interval {
                    thread::sleep(frame_interval - elapsed);
                }
            }
        });

        RenderLoopHandle {
            running,
            handle: Some(handle),
        }
    }
}

fn render_once(
    embedder: &FlutterEmbedder,
    scenes: &Arc<Mutex<Option<Scene>>>,
) -> Result<(), EmbedderError> {
    let maybe_scene = scenes.lock().unwrap();
    if let Some(scene) = maybe_scene.as_ref() {
        embedder.present_scene(scene)
    } else {
        Ok(())
    }
}

/// Allows callers to stop the loop gracefully.
pub struct RenderLoopHandle {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RenderLoopHandle {
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for RenderLoopHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

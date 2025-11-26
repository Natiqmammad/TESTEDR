use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::ui::input::{PointerPhase, PointerState};
use crate::ui::tree::{WidgetId, WidgetNode, WidgetTree};
use crate::ui::widget::WidgetKind;

#[derive(Clone, Debug, Default)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutCache {
    pub rects: HashMap<WidgetId, LayoutRect>,
}

impl LayoutCache {
    pub fn get(&self, id: &WidgetId) -> Option<&LayoutRect> {
        self.rects.get(id)
    }
}

#[derive(Clone, Debug)]
pub struct UiSnapshot {
    pub tree: WidgetTree,
    pub layout: HashMap<WidgetId, LayoutRect>,
    pub pressed_buttons: HashSet<WidgetId>,
}

#[derive(Default)]
struct UiBridgeState {
    tree: Option<WidgetTree>,
    dirty: bool,
    window_w: f32,
    window_h: f32,
    pointer: PointerState,
    pressed_buttons: HashSet<WidgetId>,
    layout_cache: HashMap<WidgetId, LayoutRect>,
}

static UI_BRIDGE: OnceLock<Mutex<UiBridgeState>> = OnceLock::new();

fn bridge() -> &'static Mutex<UiBridgeState> {
    UI_BRIDGE.get_or_init(|| Mutex::new(UiBridgeState::default()))
}

pub fn ui_set_root_tree(tree: WidgetTree) {
    let mut state = bridge().lock().expect("ui bridge poisoned");
    state.tree = Some(tree);
    state.dirty = true;
    println!("[ui-bridge] tree set");
}

pub fn ui_mark_dirty() {
    let mut state = bridge().lock().expect("ui bridge poisoned");
    state.dirty = true;
}

pub fn ui_set_window_size(w: f32, h: f32) {
    let mut state = bridge().lock().expect("ui bridge poisoned");
    state.window_w = w;
    state.window_h = h;
    state.dirty = true;
}

pub fn ui_update_pointer(state_in: PointerState) {
    let mut state = bridge().lock().expect("ui bridge poisoned");
    state.pointer = state_in;
}

pub fn ui_pointer_event(phase: PointerPhase, x: f64, y: f64) {
    let mut state = bridge().lock().expect("ui bridge poisoned");
    state.pointer.mouse_x = x;
    state.pointer.mouse_y = y;
    match phase {
        PointerPhase::Down => {
            state.pointer.mouse_down = true;
            // hit test buttons
            if let Some(tree) = &state.tree {
                if let Some(hit) = hit_test_buttons(tree, &state.layout_cache, x as f32, y as f32) {
                    state.pressed_buttons.insert(hit);
                }
            }
        }
        PointerPhase::Up => {
            state.pointer.mouse_down = false;
            state.pressed_buttons.clear();
        }
        PointerPhase::Hover => {}
    }
    state.dirty = true;
}

fn hit_test_buttons(
    tree: &WidgetTree,
    layout: &HashMap<WidgetId, LayoutRect>,
    x: f32,
    y: f32,
) -> Option<WidgetId> {
    for (id, node) in tree_iter(tree) {
        if matches!(node.kind, WidgetKind::Button { .. }) {
            if let Some(rect) = layout.get(&id) {
                if x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h {
                    return Some(id);
                }
            }
        }
    }
    None
}

fn tree_iter<'a>(tree: &'a WidgetTree) -> Vec<(WidgetId, WidgetNode)> {
    let mut v = Vec::new();
    for (id, node) in tree.nodes.iter() {
        v.push((*id, node.clone()));
    }
    v
}

pub fn compute_layout(tree: &WidgetTree, root: WidgetId, width: f32, height: f32) -> LayoutCache {
    let mut cache = LayoutCache::default();
    layout_node(tree, root, 0.0, 0.0, width, height, &mut cache);
    cache
}

fn layout_node(
    tree: &WidgetTree,
    id: WidgetId,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    cache: &mut LayoutCache,
) {
    cache.rects.insert(id, LayoutRect { x, y, w, h });
    let Some(node) = tree.get(id) else { return; };
    match &node.kind {
        WidgetKind::Column => {
            let mut cy = y;
            for child_id in tree.iter_children(id) {
                let ch = tree.get(child_id).unwrap();
                let ch_size = child_size(&ch.kind);
                let ch_h = match ch.kind {
                    WidgetKind::Spacer { size } => size,
                    _ => ch_size.1,
                };
                let cw = w;
                layout_node(tree, child_id, x, cy, cw, ch_h, cache);
                cy += ch_h;
            }
        }
        WidgetKind::Row => {
            let mut cx = x;
            for child_id in tree.iter_children(id) {
                let ch = tree.get(child_id).unwrap();
                let (cw, _) = child_size(&ch.kind);
                let cw = match ch.kind {
                    WidgetKind::Spacer { size } => size,
                    WidgetKind::Container { width, .. } => width.unwrap_or(cw),
                    _ => cw,
                };
                let ch_h = h;
                layout_node(tree, child_id, cx, y, cw, ch_h, cache);
                cx += cw;
            }
        }
        WidgetKind::Container { width, height, .. } => {
            let cw = width.unwrap_or(w);
            let ch = height.unwrap_or(h);
            if let Some(child_id) = node.children.first() {
                layout_node(tree, *child_id, x, y, cw, ch, cache);
            }
        }
        WidgetKind::Button { .. } => {}
        WidgetKind::Text { .. } => {}
        WidgetKind::Spacer { .. } => {}
        WidgetKind::Root => {
            for child_id in tree.iter_children(id) {
                layout_node(tree, child_id, x, y, w, h, cache);
            }
        }
    }
}

pub fn ui_get_snapshot_for_render() -> Option<UiSnapshot> {
    let mut state = bridge().lock().expect("ui bridge poisoned");
    if state.tree.is_none() {
        return None;
    }
    if state.dirty {
        let tree = state.tree.as_ref().unwrap();
        let layout = compute_layout(tree, tree.root(), state.window_w.max(1.0), state.window_h.max(1.0));
        state.layout_cache = layout.rects.clone();
        state.dirty = false;
    }
    println!(
        "[host] ui_get_snapshot_for_render -> tree: {:?}, rects: {}",
        state.tree.as_ref().map(|t| t.root()),
        state.layout_cache.len()
    );
    Some(UiSnapshot {
        tree: state.tree.clone().unwrap(),
        layout: state.layout_cache.clone(),
        pressed_buttons: state.pressed_buttons.clone(),
    })
}

#[derive(Clone, Debug)]
pub struct DrawRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
}

pub fn build_draw_list(snapshot: &UiSnapshot) -> Vec<DrawRect> {
    let mut out = Vec::new();
    for (id, node) in tree_iter(&snapshot.tree) {
        if let Some(rect) = snapshot.layout.get(&id) {
            match node.kind {
                WidgetKind::Button { .. } => {
                    let pressed = snapshot.pressed_buttons.contains(&id);
                    let color = if pressed {
                        [0.9, 0.3, 0.3, 1.0]
                    } else {
                        [0.2, 0.7, 0.2, 1.0]
                    };
                    out.push(DrawRect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                        color,
                    });
                }
                WidgetKind::Text { .. } => {
                    out.push(DrawRect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                        color: [0.7, 0.7, 0.7, 1.0],
                    });
                }
                WidgetKind::Container { background, .. } => {
                    let col = background
                        .map(|c| [c.r, c.g, c.b, c.a])
                        .unwrap_or([0.9, 0.9, 0.9, 0.2]);
                    out.push(DrawRect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                        color: col,
                    });
                }
                WidgetKind::Row | WidgetKind::Column => {
                    out.push(DrawRect {
                        x: rect.x,
                        y: rect.y,
                        w: rect.w,
                        h: rect.h,
                        color: [0.95, 0.95, 0.95, 0.1],
                    });
                }
                WidgetKind::Root | WidgetKind::Spacer { .. } => {}
            }
        }
    }
    out
}

fn child_size(kind: &WidgetKind) -> (f32, f32) {
    match kind {
        WidgetKind::Button { .. } => (120.0, 40.0),
        WidgetKind::Text { text } => ((text.len() as f32 * 8.0).max(80.0), 20.0),
        WidgetKind::Spacer { size } => (*size, *size),
        WidgetKind::Container { width, height, .. } => (width.unwrap_or(200.0), height.unwrap_or(200.0)),
        WidgetKind::Row | WidgetKind::Column | WidgetKind::Root => (120.0, 40.0),
    }
}

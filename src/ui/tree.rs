use std::collections::HashMap;

use crate::ui::widget::{ButtonState, WidgetKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct WidgetId(pub u64);

#[derive(Clone, Debug)]
pub struct WidgetNode {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub parent: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub layout_x: f32,
    pub layout_y: f32,
    pub layout_w: f32,
    pub layout_h: f32,
    pub dirty: bool,
}

impl WidgetNode {
    pub fn new(id: WidgetId, kind: WidgetKind) -> Self {
        Self {
            id,
            kind,
            parent: None,
            children: Vec::new(),
            layout_x: 0.0,
            layout_y: 0.0,
            layout_w: 0.0,
            layout_h: 0.0,
            dirty: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WidgetTree {
    next_id: u64,
    pub(crate) nodes: HashMap<WidgetId, WidgetNode>,
    root: WidgetId,
}

impl WidgetTree {
    /// Create a new tree with a Root widget.
    pub fn new() -> Self {
        let root_id = WidgetId(0);
        let mut nodes = HashMap::new();
        nodes.insert(root_id, WidgetNode::new(root_id, WidgetKind::Root));
        Self {
            next_id: 1,
            nodes,
            root: root_id,
        }
    }

    pub fn root(&self) -> WidgetId {
        self.root
    }

    pub fn new_widget(&mut self, kind: WidgetKind) -> WidgetId {
        let id = WidgetId(self.next_id);
        self.next_id += 1;
        let node = WidgetNode::new(id, kind);
        self.nodes.insert(id, node);
        id
    }

    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        if let Some(child_node) = self.nodes.get_mut(&child) {
            child_node.parent = Some(parent);
        }
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.children.push(child);
            parent_node.dirty = true;
        }
    }

    pub fn get(&self, id: WidgetId) -> Option<&WidgetNode> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut WidgetNode> {
        self.nodes.get_mut(&id)
    }

    pub fn mark_dirty(&mut self, id: WidgetId) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.dirty = true;
        }
    }

    pub fn iter_children<'a>(&'a self, parent: WidgetId) -> impl Iterator<Item = WidgetId> + 'a {
        self.nodes
            .get(&parent)
            .into_iter()
            .flat_map(|n| n.children.iter().copied())
    }

    pub fn remove_subtree(&mut self, id: WidgetId) {
        let children: Vec<WidgetId> = self
            .nodes
            .get(&id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        for child in children {
            self.remove_subtree(child);
        }
        self.nodes.remove(&id);
    }

    pub fn single_button_tree(button: ButtonState) -> Self {
        let mut tree = WidgetTree::new();
        let btn_id = tree.new_widget(WidgetKind::Button {
            text: "Button".to_string(),
        });
        if let Some(node) = tree.get_mut(btn_id) {
            node.layout_x = button.x;
            node.layout_y = button.y;
            node.layout_w = button.w;
            node.layout_h = button.h;
            node.dirty = true;
        }
        tree.add_child(tree.root(), btn_id);
        tree
    }
}

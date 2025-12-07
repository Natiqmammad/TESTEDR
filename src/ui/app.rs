use std::sync::{Mutex, OnceLock};

use super::runtime_bridge::{ui_mark_dirty, ui_set_root_tree};
use super::tree::{WidgetId, WidgetTree};
use super::widget::{Color, EdgeInsets, Widget, WidgetKind};

pub trait App {
    /// Build the root UI for this app.
    fn build(&self, ctx: &mut Context) -> Widget;
}

pub struct Context<'a> {
    tree: &'a mut WidgetTree,
}

impl<'a> Context<'a> {
    pub fn new(tree: &'a mut WidgetTree) -> Self {
        Self { tree }
    }

    pub fn column(&mut self, children: &[Widget]) -> Widget {
        let id = self.tree.new_widget(WidgetKind::Column);
        for child in children {
            self.tree.add_child(id, child.0);
        }
        Widget(id)
    }

    pub fn row(&mut self, children: &[Widget]) -> Widget {
        let id = self.tree.new_widget(WidgetKind::Row);
        for child in children {
            self.tree.add_child(id, child.0);
        }
        Widget(id)
    }

    pub fn text(&mut self, text: &str) -> Widget {
        let id = self.tree.new_widget(WidgetKind::Text {
            text: text.to_string(),
        });
        Widget(id)
    }

    pub fn button(&mut self, text: &str) -> Widget {
        let id = self.tree.new_widget(WidgetKind::Button {
            text: text.to_string(),
        });
        Widget(id)
    }

    pub fn container(
        &mut self,
        child: Option<Widget>,
        width: Option<f32>,
        height: Option<f32>,
        background: Option<Color>,
        padding: Option<EdgeInsets>,
    ) -> Widget {
        let id = self.tree.new_widget(WidgetKind::Container {
            width,
            height,
            background,
            padding,
        });
        if let Some(child) = child {
            self.tree.add_child(id, child.0);
        }
        Widget(id)
    }

    pub fn spacer(&mut self, size: f32) -> Widget {
        let id = self.tree.new_widget(WidgetKind::Spacer { size });
        Widget(id)
    }
}

pub struct UiRuntime {
    pub tree: WidgetTree,
    pub root: WidgetId,
}

impl UiRuntime {
    pub fn new() -> Self {
        let tree = WidgetTree::new();
        let root = tree.root();
        Self { tree, root }
    }
}

static UI_RUNTIME: OnceLock<Mutex<UiRuntime>> = OnceLock::new();

pub fn run_app<A: App>(app: A) {
    let runtime = UI_RUNTIME.get_or_init(|| Mutex::new(UiRuntime::new()));
    let mut runtime = runtime.lock().expect("UI runtime poisoned");

    {
        let mut ctx = Context {
            tree: &mut runtime.tree,
        };

        let root_widget = app.build(&mut ctx);
        let root_id = runtime.root;
        runtime.tree.add_child(root_id, root_widget.0);

        // hand off the tree to the global UI bridge (clone for now)
        ui_set_root_tree(runtime.tree.clone());
        ui_mark_dirty();
    }

    println!("[ui] Built widget tree with root {:?}", runtime.root);
}

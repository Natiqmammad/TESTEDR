use crate::ui::tree::WidgetId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Widget(pub WidgetId);

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonState {
    pub pressed: bool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ButtonState {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let bx = self.x as f64;
        let by = self.y as f64;
        let bw = self.w as f64;
        let bh = self.h as f64;

        x >= bx && x <= bx + bw && y >= by && y <= by + bh
    }
}

#[derive(Clone, Debug)]
pub enum WidgetKind {
    Root,
    Column,
    Row,
    Container {
        width: Option<f32>,
        height: Option<f32>,
        background: Option<Color>,
        padding: Option<EdgeInsets>,
    },
    Text {
        text: String,
    },
    Button {
        text: String,
    },
    Spacer {
        size: f32,
    },
}

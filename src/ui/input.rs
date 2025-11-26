#[derive(Clone, Copy, Debug, Default)]
pub struct PointerState {
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub mouse_down: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Hover,
    Down,
    Up,
}

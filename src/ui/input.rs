#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiInputEvent {
    Left,
    Right,
    Up,
    Down,
    Open,
    Back,
    PagePrev,
    PageNext,
    Quit,
}

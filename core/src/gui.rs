#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotState {
    Idle,
    Listening,
    Thinking,
    Speaking,
}

pub trait MascotRenderer {
    fn run(&mut self) -> Result<(), String>;
    fn set_state(&mut self, state: MascotState);
}

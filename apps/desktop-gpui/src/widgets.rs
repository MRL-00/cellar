use gpui::{prelude::*, Entity};
use gpui_component::input::{Input, InputState};
use gpui_component::Sizable;

pub fn compact_input(state: &Entity<InputState>) -> Input {
    Input::new(state).xsmall().appearance(false).min_w_0()
}

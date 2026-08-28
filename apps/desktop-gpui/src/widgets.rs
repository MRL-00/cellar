use gpui::{prelude::*, px, Entity};
use gpui_component::input::{Input, InputState};

pub fn compact_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
        .appearance(false)
        .h_full()
        .min_w_0()
        .px(px(6.))
        .py(px(0.))
}

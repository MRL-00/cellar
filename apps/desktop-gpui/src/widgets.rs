use gpui::{px, Entity, prelude::*};
use gpui_component::input::{Input, InputState};
use gpui_component::Sizable;

use crate::theme::FG;

pub fn compact_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
        .xsmall()
        .appearance(false)
        .min_w_0()
        .overflow_hidden()
        .text_size(px(12.))
        .line_height(px(16.))
        .text_color(FG)
}

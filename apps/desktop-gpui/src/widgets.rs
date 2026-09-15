use gpui::{Entity, prelude::*};
use gpui_component::input::{Input, InputState};
use gpui_component::Sizable;

use crate::theme::{ui_px, FG};

pub fn compact_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
        .xsmall()
        .appearance(false)
        .min_w_0()
        .overflow_hidden()
        .text_size(ui_px(13.))
        .line_height(ui_px(18.))
        .text_color(FG)
}

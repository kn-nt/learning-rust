mod constants;
mod misc;
mod websocket;
mod input;
mod app_state;
mod triangle_drawing;

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast};

#[wasm_bindgen(start)]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut app_state = app_state::ApplicationState::new();

    app_state.init();

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    

    *g.borrow_mut() = Some(Closure::new(move || {
        app_state.set_debug_stats();
        app_state.reset_canvas();
        app_state.handle_input();

        match app_state.draw_setting {
            0 => app_state.triangle_drawing.draw_triangle(),
            1 => app_state.triangle_drawing.draw_tri_vao(),
            2 => app_state.triangle_drawing.draw_sierpinski_tri_simple(),
            3 => app_state.triangle_drawing.draw_tri_random(),
            _ => {},
        };

        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    misc::window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}
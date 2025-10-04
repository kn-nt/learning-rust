use web_sys::WebGl2RenderingContext;
use crate::{create_shader, link_program, app_state};

pub struct TriangleDrawing<'a> {
    pub init_setup: bool,
    pub app_state: &'a app_state::ApplicationState,
    pub gl : &'a WebGl2RenderingContext,
}

impl<'a> TriangleDrawing<'a> {
    pub fn new(app_state: &app_state::ApplicationState) -> TriangleDrawing {
        TriangleDrawing {
            init_setup: false,
            app_state,
            gl: &app_state.gl
        }
    }
}
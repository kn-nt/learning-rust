use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{js_sys, JsFuture};
use web_sys::{
    console, HtmlCanvasElement, Response, WebGl2RenderingContext,
    WebGlBuffer, WebGlProgram, WebGlShader, WebGlUniformLocation, WebSocket,
};

#[wasm_bindgen(start)]
pub async unsafe fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let document = window().document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    let gl = canvas
        .get_context("webgl2")
        .unwrap()
        .expect("Browser doesn't support webgl2")
        .dyn_into::<WebGl2RenderingContext>()
        .unwrap();

    canvas.set_width(canvas.client_width() as u32);
    canvas.set_height(canvas.client_height() as u32);
    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);

    let glsl_v = r##"#version 300 es

    // an attribute is an input (in) to a vertex shader.
    // It will receive data from a buffer
    in vec4 a_position;

    // all shaders have a main function
    void main() {

      // gl_Position is a special variable a vertex shader
      // is responsible for setting
      gl_Position = a_position;
    }"##;

    let glsl_f = r##"    #version 300 es

    // fragment shaders don't have a default precision so we need
    // to pick one. highp is a good default. It means "high precision"
    precision highp float;

    // we need to declare an output for the fragment shader
    out vec4 outColor;

    void main() {
      // Just set the output to a constant reddish-purple
      outColor = vec4(1, 0, 0.5, 1);
    }
    "##;
    let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
    let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

    let program = link_program(&gl, &shader_v, &shader_f).unwrap();

    let att_a_position: u32 = gl.get_attrib_location(&program, "a_position") as u32;

    let vertices: Vec<f32> = vec![
        0.0, 0.5,
        -0.5, -0.5,
        0.5, -0.5
    ];

    gl.clear_color(0.08, 0.08, 0.08, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

    let buf_vert = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_vert));
    gl.buffer_data_with_array_buffer_view(WebGl2RenderingContext::ARRAY_BUFFER, &js_sys::Float32Array::view(&vertices), WebGl2RenderingContext::STATIC_DRAW);

    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(&vao));


    gl.enable_vertex_attrib_array(att_a_position);

    gl.use_program(Some(&program));

    gl.vertex_attrib_pointer_with_i32(
        att_a_position,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0
    );

    gl.draw_arrays(
        WebGl2RenderingContext::TRIANGLES,
        0,
        3
    );


}

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn create_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    glsl: &str,
) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(shader_type).unwrap();
    gl.shader_source(&shader, glsl);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

fn link_program(
    gl: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = gl
        .create_program()
        .ok_or_else(|| String::from("Unable to create program object"))
        .unwrap();

    gl.attach_shader(&program, &vert_shader);
    gl.attach_shader(&program, &frag_shader);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
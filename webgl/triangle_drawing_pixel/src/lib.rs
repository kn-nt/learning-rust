mod constants;

use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{js_sys, JsFuture};
use web_sys::{
    console, HtmlCanvasElement, Response, WebGl2RenderingContext, WebGlBuffer, WebGlProgram,
    WebGlShader, WebGlUniformLocation, WebSocket,
};

#[wasm_bindgen(start)]
pub unsafe fn main() {
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
    print(&format!(
        "H {} W: {}",
        canvas.client_height(),
        canvas.client_width()
    ));
    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
    let glsl_v = r##"#version 300 es

    // an attribute is an input (in) to a vertex shader.
    // It will receive data from a buffer
    in vec2 a_position;
    in vec2 a_texCoord;
    uniform vec2 u_canvas_size;
    
    out vec2 v_texCoord;

    // all shaders have a main function
    void main() {
        vec2 pixel_position = a_position / u_canvas_size;
        vec2 clip_space = (pixel_position * 2.0) - 1.0;
        gl_Position = vec4(clip_space * vec2(1.0, -1.0), 0.0, 1.0);
        v_texCoord = a_texCoord * vec2(1.0, -1.0);
    }"##;

    let glsl_f = r##"#version 300 es

    // fragment shaders don't have a default precision so we need
    // to pick one. highp is a good default. It means "high precision"
    precision highp float;

    in vec2 v_texCoord;
    
    uniform sampler2D u_image;
    
    out vec4 outColor;

    void main() {
        outColor = texture(u_image, v_texCoord);
    }
    "##;
    let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
    let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

    let program = link_program(&gl, &shader_v, &shader_f).unwrap();

    gl.clear_color(0.08, 0.08, 0.08, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

    draw_triangle(&gl, &program);
}

pub fn time() -> f64 {
    return window().performance().unwrap().now();
}

pub unsafe fn draw_triangle(gl: &WebGl2RenderingContext, program: &WebGlProgram) {
    let canvas = gl
        .canvas()
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    let canvas_h: f32 = canvas.client_height() as f32;
    let canvas_w: f32 = canvas.client_width() as f32;

    let mut vertices: Vec<f32> = vec![0.0, 0.0, 0.0, 200.0, 200.0, 200.0,
                                      0.0, 200.0, 0.0, 400.0, 200.0, 400.0];
    let mut vertices: Vec<f32> = vec![0.0, 0.0, 0.0, 200.0, 200.0, 200.0,
                                      0.0, 200.0, 0.0, 400.0, 200.0, 400.0];
    let mut vertice_color: Vec<f32> = vec![1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5,1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5];

    let texture_coords: [f32; 12] = [
        1.0, 1.0, // top
        0.0, 0.0, // bottom left
        1.0, 0.0, // bottom right
        1.0, 1.0, // top
        0.0, 0.0, // bottom left
        0.0, 1.0, // bottom right
    ];

    let att_a_position: u32 = gl.get_attrib_location(program, "a_position") as u32;
    let tex_coord_attr_loc = gl.get_attrib_location(&program, "a_texCoord");
    let u_canvas_size = gl.get_uniform_location(&program, "u_canvas_size").unwrap();

    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(&vao));

    let buf_vert = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_vert));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &js_sys::Float32Array::view(&vertices),
        WebGl2RenderingContext::STATIC_DRAW,
    );

    gl.enable_vertex_attrib_array(att_a_position);
    gl.vertex_attrib_pointer_with_i32(
        att_a_position,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );

    // let buf_vert_color = gl.create_buffer().unwrap();
    // gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_vert_color));
    // gl.buffer_data_with_array_buffer_view(
    //     WebGl2RenderingContext::ARRAY_BUFFER,
    //     &js_sys::Float32Array::view(&vertice_color),
    //     WebGl2RenderingContext::STATIC_DRAW,
    // );
    // 
    // gl.enable_vertex_attrib_array(att_v_color);
    // gl.vertex_attrib_pointer_with_i32(att_v_color, 3, WebGl2RenderingContext::FLOAT, false, 0, 0);
    
    let tex_vert = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&tex_vert));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &js_sys::Float32Array::view(&texture_coords),
        WebGl2RenderingContext::STATIC_DRAW,
    );
    gl.enable_vertex_attrib_array(tex_coord_attr_loc as u32);
    gl.vertex_attrib_pointer_with_i32(
        tex_coord_attr_loc as u32,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );
    
    gl.use_program(Some(&program));
    gl.uniform2f(Some(&u_canvas_size), canvas_w, canvas_h);

    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
}

pub fn print(s: &str) {
    console::log_1(&s.into());
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

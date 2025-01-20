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
    in vec3 a_triangle_color;
    uniform float u_triangle_size;
    uniform vec2 u_triangle_center;
    uniform vec2 u_canvas_size;
    
    out vec3 f_triangle_color;

    // all shaders have a main function
    void main() {
      vec2 pixel_position = a_position * u_triangle_size + u_triangle_center;
      vec2 clip_space = (pixel_position/ u_canvas_size) * 2.0 - 1.0;
      // gl_Position is a special variable a vertex shader
      // is responsible for setting
      // gl_Position = a_position;
      gl_Position = vec4(clip_space, 0.0, 1.0);
      f_triangle_color = a_triangle_color;
    }"##;

    let glsl_f = r##"#version 300 es

    // fragment shaders don't have a default precision so we need
    // to pick one. highp is a good default. It means "high precision"
    precision highp float;

    in vec3 f_triangle_color;
    // we need to declare an output for the fragment shader
    out vec4 outColor;

    void main() {
      // Just set the output to a constant reddish-purple
      outColor = vec4(f_triangle_color, 1);
    }
    "##;
    let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
    let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

    let program = link_program(&gl, &shader_v, &shader_f).unwrap();

    gl.clear_color(0.08, 0.08, 0.08, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);

    // draw_triangle(
    //     &gl,
    //     &program,
    //     (500, 375),
    //     250,
    //     false
    // );
    // draw_triangle(
    //     &gl,
    //     &program,
    //     (375, 310),
    //     125,
    //     false
    // );

    draw_sierpinski_triangle(
        &gl,
        &program,
        (canvas.client_height() as i16/ 2, canvas.client_width() as i16/ 2),
        canvas.client_height() as u16
    );
    
}

pub fn sleep(ms: f64) {
    let now = time();
    while now + ms > time() {}
}

pub fn time() -> f64 {
    return window().performance().unwrap().now()
}

pub unsafe fn draw_sierpinski_triangle(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    position: (i16, i16),
    size: u16,
) {
    draw_triangle(gl, program, position, size, true)
}

pub unsafe fn draw_triangle(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    position: (i16, i16),
    size: u16,
    top: bool
) {
    if size <= 2 { 
        return; 
    }
    
    let canvas_h: f32 = gl.canvas().unwrap().dyn_into::<HtmlCanvasElement>().unwrap().client_height() as f32;
    let canvas_w: f32 = gl.canvas().unwrap().dyn_into::<HtmlCanvasElement>().unwrap().client_width() as f32;
    let x: f32 = position.0 as f32;
    let y: f32 = position.1 as f32;

    let mut vertices: Vec<f32> = vec![0.0, 0.5, -0.5, -0.5, 0.5, -0.5];
    let mut vertice_color: Vec<f32> = vec![1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5];
    if !top {
        vertices = vec![-0.5, 0.5, 0.0, -0.5, 0.5, 0.5];
        vertice_color = vec![0.0, 0.5, 0.5, 0.0, 0.5, 0.5, 0.0, 0.5, 0.5];
    }
    
    let att_a_position: u32 = gl.get_attrib_location(program, "a_position") as u32;
    let att_v_color: u32 = gl.get_attrib_location(program, "a_triangle_color") as u32;
    let u_triangle_size = gl
        .get_uniform_location(&program, "u_triangle_size")
        .unwrap();
    let u_triangle_center = gl
        .get_uniform_location(&program, "u_triangle_center")
        .unwrap();
    let u_canvas_size = gl
        .get_uniform_location(&program, "u_canvas_size")
        .unwrap();

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


    let buf_vert_color = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_vert_color));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &js_sys::Float32Array::view(&vertice_color),
        WebGl2RenderingContext::STATIC_DRAW,
    );

    gl.enable_vertex_attrib_array(att_v_color);
    gl.vertex_attrib_pointer_with_i32(
        att_v_color,
        3,
        WebGl2RenderingContext::FLOAT,
        false,
        0,
        0,
    );

    
    gl.use_program(Some(&program));
    gl.uniform1f(Some(&u_triangle_size), size as f32);
    gl.uniform2f(Some(&u_triangle_center), x, y);
    gl.uniform2f(Some(&u_canvas_size), canvas_h as f32, canvas_w as f32);

    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
    
    let new_size = ((size as f32/ 2f32) * 1.003) as u16;
    // sleep(5.0);
    if top {
        let new_position = (position.0, position.1 - (size as i16/4) );
        draw_triangle(gl, program, new_position, new_size, false);
        // let position = new_position;
        // let position_1 = (position.0 - (new_size as i16/2), position.1 - (size as i16/8) );
        // let position_2 = (position.0, position.1 + (size as i16 / 8 * 3) );
        // let position_3 = (position.0 + (new_size as i16/2), position.1 - (size as i16/8) );
        // let new_size = new_size/ 2;
        // 
        // draw_triangle(gl, program, position_1, new_size, false);
        // draw_triangle(gl, program, position_2, new_size, false);
        // draw_triangle(gl, program, position_3, new_size, false);
    } else {
        let position_1 = (position.0 - (size as i16/2), position.1 - (size as i16/4) );
        let position_2 = (position.0, position.1 + (size as i16/ 4 * 3) );
        let position_3 = (position.0 + (size as i16/2), position.1 - (size as i16/4) );
        
        draw_triangle(gl, program, position_1, new_size, false);
        draw_triangle(gl, program, position_2, new_size, false);
        draw_triangle(gl, program, position_3, new_size, false);
    }
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

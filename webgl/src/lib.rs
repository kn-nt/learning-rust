mod constants;
mod misc;
mod websocket;
mod triangle_drawing;
mod input;
mod app_state;

use getrandom::getrandom;
use nx::{NodeDataPopulated, NodeSH};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{js_sys, JsFuture};
use web_sys::{console, HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture};
use websocket::WSResponse;

#[wasm_bindgen(start)]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut app_state = app_state::ApplicationState::new();

    app_state.init();

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    // Lessons learned so far
    // Make sure to not constantly upload textures to buffer otherwise you run into memory issues
    //      VRAM was full -> spilled into RAM
    // Make sure to set uniform calls before draw every time or at least once per draw time
    //      It is set up per program and doesn't change by default
    *g.borrow_mut() = Some(Closure::new(move || {
        app_state.set_debug_stats();
        app_state.reset_canvas();
        app_state.handle_input();

        match app_state.draw_setting {
            0 => app_state.draw_triangle(),
            1 => app_state.draw_triangle_vao(),
            2 => app_state.draw_sierpinski_simple_tri(),
            _ => {},
        };

        // draw_triangle_at_coords_instanced(&gl, &program_i, &[0.0, 0.0, 400.0, 400.0, 500.0, 200.0]);
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}
pub fn time() -> f64 {
    window().performance().unwrap().now()
}

pub fn setup_instanced_program(gl: &WebGl2RenderingContext) -> WebGlProgram {
    let glsl_v = r##"#version 300 es

    // an attribute is an input (in) to a vertex shader.
    // It will receive data from a buffer
    in vec2 a_position;
    in vec2 a_instancePosition;
    in vec2 a_texCoord;
    uniform vec2 u_canvas_size;

    out vec2 v_texCoord;

    // all shaders have a main function
    void main() {
        // the below converts the incoming a_postion values (pixel coordinates)
        // to WebGL's clip space coordinates
        vec2 pixel_position = (a_position + a_instancePosition) / u_canvas_size;
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
        //outColor = vec4(0.5, 0.0, 0.5, 1.0);
        outColor = texture(u_image, v_texCoord);
    }
    "##;
    let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
    let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

    link_program(&gl, &shader_v, &shader_f).unwrap()
}

/// This is test code to see if it causes lag- and it does!
/// Doesn't cause memory leak though- not sure exactly how to recreate that
pub fn upload_tex(gl: &WebGl2RenderingContext, tex: &WebGlTexture, w: u16, h: u16, bitmap: &[u8]) {
    // gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&tex));
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        constants::TARGET,
        constants::LEVEL,
        constants::INTERNAL_FORMAT,
        w as i32,
        h as i32,
        constants::BORDER,
        constants::SRC_FORMAT,
        constants::SRC_TYPE,
        Some(&bitmap),
    )
        .expect("Cannot generate tex");
}

pub fn set_active_tex(gl: &WebGl2RenderingContext, gl_tex: &WebGlTexture) {
    gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(gl_tex));
}

pub fn setup_all_buffers(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    w: u16,
    h: u16,
    bitmap: &[u8],
    origin: (f32, f32),
) -> (WebGlBuffer, WebGlTexture) {
    // Calculated vertices
    // Definitions
    //     Origin: the center/ starting point for the texture to be drawn
    // Calculation is as follows
    // Texture Rectangle Dimensions = Image's actual H x W
    // Texture Rectangle Coords = Defined Coords - Origin
    let vertices: Vec<f32> = vec![
        w as f32 - origin.0,
        -origin.1, // top right
        -origin.0,
        h as f32 - origin.1, // bottom left
        w as f32 - origin.0,
        h as f32 - origin.1, // bottom right
        w as f32 - origin.0,
        -origin.1, // top right
        -origin.0,
        h as f32 - origin.1, // bottom left
        -origin.0,
        -origin.1, // top left
    ];

    // Attributes here refer to the input parameters in the glsl scripts above
    // Gets location within the program so that the attributes can be used
    let att_a_position = gl.get_attrib_location(program, "a_position");

    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(&vao));

    // Creates new buffer to send data to GPU
    let buf_vert = gl.create_buffer().unwrap();
    // Binds buffer to ARRAY_BUFFER
    // ARRAY_BUFFER is something called a 'bind point' and is approx a global internal variable
    // Binds resource to a bind point
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_vert));
    // Now you can put data into the buffer by referencing it through that bind point
    // Eg
    // vertex_buffer --(Bind)--> ARRAY_BUFFER
    // Vertex Data --> ARRAY_BUFFER (vertex_buffer)
    // After buffer_data execution, said buffer is still bound to ARRAY_BUFFER
    // and you can safely bind a new buffer to ARRAY_BUFFER
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &create_js_f32arr(&vertices),
        WebGl2RenderingContext::STATIC_DRAW, // not sure what this changes because MDN docs don't fully explain
    );

    // Enables attribute because attributes are disabled by default
    gl.enable_vertex_attrib_array(att_a_position as u32);
    // Binds buffer that's current bound to ARRAY_BUFFER to vertex buffer object
    // https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/vertexAttribPointer
    gl.vertex_attrib_pointer_with_i32(
        att_a_position as u32,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        // Byte offset between consecutive attributes
        // Eg. [0,0,  1,1,  1,0] (pretend they are f32, not u8)
        // If you need to access the 1,1 vertex, you need to give stride of 2 * 4:
        // each value itself is 4 bytes (32 bit floating point) and there are 2 ahead of 1,1
        0,
        0,
    );
    // Texture coordinates refer to coordinates on texture image
    // 0,0 starts at bottom left and top right is 1,1
    let texture_coords: [f32; 12] = [
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        1.0, 0.0, // bottom right
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        0.0, 1.0, // top left
    ];
    let tex_coord_attr_loc = gl.get_attrib_location(program, "a_texCoord");

    let buf_tex = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_tex));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &create_js_f32arr(&texture_coords),
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

    gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    // Create texture object to hold the actual texture data
    let texture = gl.create_texture().expect("Cannot create gl texture");
    // Binds texture object to TEXTURE_2D (binding point)
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
    // Below essentially turns off mipmaps- this is needed to perform tex_image_2d call
    // tex_parameteri sets texture parameters
    // TEXTURE_MIN_FILTER defines filtering method for when textures are scaled down
    // LINEAR essentially tells WebGL to not use any mipmaps, will always sample from the base level of the texture (level 0)
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::LINEAR as i32,
    );

    // Populate texture object through bind point (TEXTURE_2D)
    // Tex Image 2D fails if the mipmap setting above isn't set probably because mipmaps are incomplete
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        constants::TARGET,
        constants::LEVEL,
        constants::INTERNAL_FORMAT,
        w as i32,
        h as i32,
        constants::BORDER,
        constants::SRC_FORMAT,
        constants::SRC_TYPE,
        Some(&bitmap),
    )
        .expect("Cannot generate tex");

    (buf_vert, texture)
}
pub fn setup_inst_buffer(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    coords: &[f32],
) -> WebGlBuffer {
    let buf_instance_positions = gl.create_buffer().unwrap();
    if constants::INSTANCED_DRAW {
        gl.bind_buffer(
            WebGl2RenderingContext::ARRAY_BUFFER,
            Some(&buf_instance_positions),
        );
        gl.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &create_js_f32arr(&coords),
            WebGl2RenderingContext::STATIC_DRAW,
        );
        let att_a_instance_position = gl.get_attrib_location(program, "a_instancePosition");
        gl.enable_vertex_attrib_array(att_a_instance_position as u32);
        gl.vertex_attrib_pointer_with_i32(
            att_a_instance_position as u32,
            2,
            WebGl2RenderingContext::FLOAT,
            false,
            2 * 4,
            0,
        );
        // Tells WebGL to treat att_a_instance_position attribute as per-instance (6 vertices)
        // so all points on a square (2 triangles) have this applied to it
        gl.vertex_attrib_divisor(att_a_instance_position as u32, 1);
    }

    buf_instance_positions
}

pub fn draw_triangle_at_coords_instanced_optimized(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    coords: &[f32],
    buf_instance_position: &WebGlBuffer,
) {
    let canvas = gl
        .canvas()
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    let canvas_h: f32 = canvas.client_height() as f32;
    let canvas_w: f32 = canvas.client_width() as f32;

    // print(&format!("{:?}", signboard_node.children.keys()));
    let u_canvas_size = gl.get_uniform_location(program, "u_canvas_size").unwrap();
    // Needs to be set once per program normally but we are updating it actively because resolution my change
    // Need to move this to vertex buffer
    gl.uniform2f(Some(&u_canvas_size), canvas_w, canvas_h);

    gl.bind_buffer(
        WebGl2RenderingContext::ARRAY_BUFFER,
        Some(buf_instance_position),
    );
    gl.buffer_sub_data_with_f64_and_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        0.0,
        &create_js_f32arr(&coords),
    );

    // gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
    // gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 6, 6);
    // Look into testing instanced draw calls for multiple texture draws
    gl.draw_arrays_instanced(
        WebGl2RenderingContext::TRIANGLES,
        0,
        6,
        (coords.len() / 2) as i32,
    );
}

pub fn draw_triangle_at_coords_optimized(
    gl: &WebGl2RenderingContext,
    coords: &[f32],
    program: &WebGlProgram,
    buf_vertex: &WebGlBuffer,
    w: u16,
    h: u16,
    origin: (f32, f32)
) {
    let canvas = gl
        .canvas()
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    let canvas_h: f32 = canvas.client_height() as f32;
    let canvas_w: f32 = canvas.client_width() as f32;

    let vertices: Vec<f32> = vec![
        w as f32 - origin.0 + coords[0],
        -origin.1 + coords[1], // top right
        -origin.0 + coords[0],
        h as f32 - origin.1 + coords[1], // bottom left
        w as f32 - origin.0 + coords[0],
        h as f32 - origin.1 + coords[1], // bottom right
        w as f32 - origin.0 + coords[0],
        -origin.1 + coords[1], // top right
        -origin.0 + coords[0],
        h as f32 - origin.1 + coords[1], // bottom left
        -origin.0 + coords[0],
        -origin.1 + coords[1], // top left
    ];

    let u_canvas_size = gl.get_uniform_location(program, "u_canvas_size").unwrap();

    gl.uniform2f(Some(&u_canvas_size), canvas_w, canvas_h);
    
    
    gl.bind_buffer(
        WebGl2RenderingContext::ARRAY_BUFFER,
        Some(buf_vertex),
    );
    gl.buffer_sub_data_with_f64_and_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        0.0,
        &create_js_f32arr(&vertices),
    );

    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
}

pub fn create_js_f32arr(input: &[f32]) -> js_sys::Float32Array {
    let f32arr = js_sys::Float32Array::new_with_length(input.len() as u32);
    f32arr.copy_from(&input);
    f32arr
}

pub fn draw_triangle_at_coords_instanced(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    coords: &[f32],
    origin: (f32, f32),
    w: u16,
    h: u16,
    bitmap: &[u8],
) {
    let canvas = gl
        .canvas()
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    let canvas_h: f32 = canvas.client_height() as f32;
    let canvas_w: f32 = canvas.client_width() as f32;

    // print(&format!("{:?}", signboard_node.children.keys()));

    // Calculated vertices
    // Definitions
    //     Origin: the center/ starting point for the texture to be drawn
    // Calculation is as follows
    // Texture Rectangle Dimensions = Image's actual H x W
    // Texture Rectangle Coords = Defined Coords - Origin
    let vertices: Vec<f32> = vec![
        w as f32 - origin.0,
        -origin.1, // top right
        -origin.0,
        h as f32 - origin.1, // bottom left
        w as f32 - origin.0,
        h as f32 - origin.1, // bottom right
        w as f32 - origin.0,
        -origin.1, // top right
        -origin.0,
        h as f32 - origin.1, // bottom left
        -origin.0,
        -origin.1, // top left
    ];

    // Texture coordinates refer to coordinates on texture image
    // 0,0 starts at bottom left and top right is 1,1
    let texture_coords: [f32; 12] = [
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        1.0, 0.0, // bottom right
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        0.0, 1.0, // top left
    ];

    // Attributes here refer to the input parameters in the glsl scripts above
    // Gets location within the program so that the attributes can be used
    let att_a_position = gl.get_attrib_location(program, "a_position");
    let att_a_instance_position = gl.get_attrib_location(program, "a_instancePosition");
    let tex_coord_attr_loc = gl.get_attrib_location(program, "a_texCoord");
    let u_canvas_size = gl.get_uniform_location(program, "u_canvas_size").unwrap();

    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(&vao));

    // Creates new buffer to send data to GPU
    let buf_vert = gl.create_buffer().unwrap();
    // Binds buffer to ARRAY_BUFFER
    // ARRAY_BUFFER is something called a 'bind point' and is approx a global internal variable
    // Binds resource to a bind point
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_vert));
    // Now you can put data into the buffer by referencing it through that bind point
    // Eg
    // vertex_buffer --(Bind)--> ARRAY_BUFFER
    // Vertex Data --> ARRAY_BUFFER (vertex_buffer)
    // After buffer_data execution, said buffer is still bound to ARRAY_BUFFER
    // and you can safely bind a new buffer to ARRAY_BUFFER
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &create_js_f32arr(&vertices),
        WebGl2RenderingContext::STATIC_DRAW, // not sure what this changes because MDN docs don't fully explain
    );

    // Enables attribute because attributes are disabled by default
    gl.enable_vertex_attrib_array(att_a_position as u32);
    // Binds buffer that's current bound to ARRAY_BUFFER to vertex buffer object
    // https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/vertexAttribPointer
    gl.vertex_attrib_pointer_with_i32(
        att_a_position as u32,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        // Byte offset between consecutive attributes
        // Eg. [0,0,  1,1,  1,0] (pretend they are f32, not u8)
        // If you need to access the 1,1 vertex, you need to give stride of 2 * 4:
        // each value itself is 4 bytes (32 bit floating point) and there are 2 ahead of 1,1
        0,
        0,
    );

    let buf_tex = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_tex));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &create_js_f32arr(&texture_coords),
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

    let buf_instance_positions = gl.create_buffer().unwrap();
    gl.bind_buffer(
        WebGl2RenderingContext::ARRAY_BUFFER,
        Some(&buf_instance_positions),
    );
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &create_js_f32arr(&coords),
        WebGl2RenderingContext::STATIC_DRAW,
    );
    gl.enable_vertex_attrib_array(att_a_instance_position as u32);
    // The below's stride isn't REALLY required because the data is tightly packed and  
    // webgl already knows to increment by 2 * num of bytes per float because size = 2
    gl.vertex_attrib_pointer_with_i32(
        att_a_instance_position as u32,
        2, // tells webgl the number of attributes to use
        WebGl2RenderingContext::FLOAT,
        false,
        2 * 4, // this refers to the # of bytes to skip until the next attribute
        0,
    );
    // Tells WebGL to treat att_a_instance_position attribute as per-instance (6 vertices)
    // the 6 vertices per instance is defined during the draw call count
    // so all points on a square (2 triangles) have this applied to it
    gl.vertex_attrib_divisor(att_a_instance_position as u32, 1);

    gl.use_program(Some(&program));
    gl.uniform2f(Some(&u_canvas_size), canvas_w, canvas_h);

    // Create texture object to hold the actual texture data
    let texture = gl.create_texture().expect("Cannot create gl texture");
    // Binds texture object to TEXTURE_2D (binding point)
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
    // Below essentially turns off mipmaps- this is needed to perform tex_image_2d call
    // tex_parameteri sets texture parameters
    // TEXTURE_MIN_FILTER defines filtering method for when textures are scaled down
    // LINEAR essentially tells WebGL to not use any mipmaps, will always sample from the base level of the texture (level 0)
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::LINEAR as i32,
    );

    // Populate texture object through bind point (TEXTURE_2D)
    // Tex Image 2D fails if the mipmap setting above isn't set probably because mipmaps are incomplete
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        constants::TARGET,
        constants::LEVEL,
        constants::INTERNAL_FORMAT,
        w as i32,
        h as i32,
        constants::BORDER,
        constants::SRC_FORMAT,
        constants::SRC_TYPE,
        Some(&bitmap),
    )
    .expect("Cannot generate tex");
    // gl.generate_mipmap(WebGl2RenderingContext::TEXTURE_2D);

    // gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
    // gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 6, 6);
    // Look into testing instanced draw calls for multiple texture draws
    gl.draw_arrays_instanced(
        WebGl2RenderingContext::TRIANGLES,
        0,
        6,
        (coords.len() / 2) as i32,
    );
}

pub async fn sleep(millis: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis)
            .unwrap();
    };
    let p = js_sys::Promise::new(&mut cb);
    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
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



pub struct DebugStats {
    pub fps_timing: VecDeque<f32>,
    pub frame_time: f32,
    pub draw_calls: u16,
    pub draw_calls_instanced: u16,
    pub draw_calls_single: u16,
    pub canvas_size: (u16, u16),

    pub hashmap_node: HashMap<String, web_sys::Node>,
}

impl DebugStats {
    pub fn new() -> DebugStats {
        DebugStats {
            fps_timing: VecDeque::new(),
            frame_time: 0.0,
            draw_calls: 0,
            draw_calls_instanced: 0,
            draw_calls_single: 0,
            canvas_size: (0, 0),
            hashmap_node: HashMap::new(),
        }
    }

    pub fn add_draw_call(&mut self, draws: Option<u16>) {
        match draws {
            None => self.draw_calls += 1,
            Some(draws) => {
                if draws > 0 {
                    self.draw_calls += draws
                }
            }
        }
    }

    pub fn add_debug_node(&mut self, name: &str, value: web_sys::Node) {
        self.hashmap_node.insert(name.to_string(), value);
    }

    pub fn set_node_val(&self, node: &str, value: &str) {
        if let Some(node) = self.hashmap_node.get(node) {
            node.set_text_content(Some(value));
        }
    }

    pub fn init(&mut self) {
        self.create_node("fps");
        self.create_node("frame_time");
        self.create_node("draw_calls");
        self.create_node("canvas_size");
        self.create_node("input");
        self.create_node("msg");
    }

    pub fn set_debug_msg(&self, msg: &str) {
        self.set_node_val("msg", msg);
    }

    pub fn create_node(&mut self, name: &str) {
        let doc = gloo_utils::document();
        let ele = doc
            .query_selector(&format!("#{}", name))
            .expect(&format!("Node #{} does not exist", name))
            .expect(&format!("Node #{} does not exist", name));
        let text = doc.create_text_node("");
        let node = ele.append_child(&**text).unwrap();
        self.hashmap_node.insert(name.to_string(), node);
    }

    pub fn calculate_frame_time(&mut self) {
        self.frame_time = 0.0;
        if self.fps_timing.len() > 1 {
            let idx = self.fps_timing.len() - 1;
            self.frame_time = self.fps_timing[idx] - self.fps_timing[idx - 1];
        }
    }
}
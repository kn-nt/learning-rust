mod constants;
mod misc;
mod websocket;

use getrandom::getrandom;
use nx::{NodeDataPopulated, NodeSH};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{js_sys, JsFuture};
use web_sys::{console, HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture, WebGlUniformLocation, WebSocket};
use websocket::WSResponse;

static COMPLETE_HASH_MAP: OnceLock<RwLock<NodeSH>> = OnceLock::new();

/// See the latest draw_triangle function's docstring and comments for a (relatively) detailed line-by-line explanation
#[wasm_bindgen(start)]
pub async unsafe fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    COMPLETE_HASH_MAP
        .set(RwLock::new(NodeSH {
            data: NodeDataPopulated::None,
            children: HashMap::new(),
        }))
        .expect("CANNOT INIT");

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
        // the below converts the incoming a_postion values (pixel coordinates)
        // to WebGL's clip space coordinates
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
        //outColor = vec4(0.5, 0.0, 0.5, 1.0);
        outColor = texture(u_image, v_texCoord);
    }
    "##;
    let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
    let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

    let mut program = link_program(&gl, &shader_v, &shader_f).unwrap();

    gl.clear_color(0.08, 0.08, 0.08, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
    let tmp_node_holder: Arc<Mutex<WSResponse>> = Arc::new(Mutex::new(WSResponse::Empty));

    let ws = websocket(&tmp_node_holder).await;

    match websocket::get_full_img_file(
        &ws,
        "UI.nx/MapLogin.img".to_string(),
        Arc::clone(&tmp_node_holder),
        &COMPLETE_HASH_MAP,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => print(&format!("Cannot dl file {}", e)),
    }
    ws.close().unwrap();

    // FPS Counter in HTML https://webgl2fundamentals.org/webgl/lessons/webgl-text-html.html
    let fps_ele = document.query_selector("#fps").unwrap().unwrap();
    let fps_text = document.create_text_node("");
    let fps_node = fps_ele.append_child(&**fps_text).unwrap();
    let draw_calls_ele = gloo_utils::document()
        .query_selector("#draw_calls")
        .unwrap()
        .unwrap();
    let draw_calls_text = gloo_utils::document().create_text_node("");
    let draw_calls_node = draw_calls_ele.append_child(&**draw_calls_text).unwrap();
    let msg_ele = document.query_selector("#msg").unwrap().unwrap();
    let msg_text = document.create_text_node("");
    let msg_node = msg_ele.append_child(&**msg_text).unwrap();

    let mut fps_timing: VecDeque<f64> = VecDeque::new();
    let mut draw_calls: u32 = 0u32;
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    let bitmap: Vec<u8>;
    let w: u16;
    let h: u16;
    let mut origin: (f32, f32) = (0.0, 0.0);

    let node = COMPLETE_HASH_MAP.get().unwrap().read().unwrap();
    // print(&format!("{:?}", node.children["UI.nx"].children["MapLogin.img"].children.keys()));
    // print(&format!("{:?}", node.children["Map.nx"].children["Obj"].children["login.img"].children["Title"].children["signboard"].children["0"].children.keys()));
    let signboard_node = &node.children["Map.nx"].children["Obj"].children["login.img"].children
        ["Title"]
        .children["signboard"]
        .children["0"]
        .children["0"];
    match signboard_node.data {
        NodeDataPopulated::Bitmap {
            data: _,
            width,
            height,
        } => {
            bitmap = signboard_node.data.decompress().unwrap();
            w = width.clone(); // 368 px
            h = height.clone(); // 236 px
        }
        _ => {
            panic!("Signboard node is not populated");
        }
    }

    match signboard_node.children["origin"].data {
        NodeDataPopulated::Vector(x, y) => {
            origin.0 = x as f32;
            origin.1 = y as f32;
        }
        _ => {
            panic!("Missing origin");
        }
    }

    let min = 1f32;
    let f_max = 2000f32;
    let mut bytes = [0u8; 1000 * 4]; // 500 f32 values, each 4 bytes
    let mut bytes_smol = [0u8; 2 * 4]; // 500 f32 values, each 4 bytes
    getrandom(&mut bytes).expect("random number generation failed");
    let coords: Vec<f32> = bytes
        .chunks_exact(1) // Each chunk represents one f32 (4 bytes)
        .map(|chunk| {
            let num = chunk[0];
            let normalized = num as f32 / u8::MAX as f32;
            min + ((f_max - min) * normalized)
        })
        .collect();

    // print(&format!("{:?}", coords));
    // print(&format!("{:?}", &coords[0..2]));


    // removes program above as the shader for instanced drawing is slightly different
    // can do dynamically generated GLSL but eh whatever
    if constants::INSTANCED_DRAW {
        gl.delete_program(Some(&program));
        program = setup_instanced_program(&gl);
    }
    let (buf_vert, texture) = setup_all_buffers(&gl, &program, w, h, &bitmap, origin);
    let buf_insta = setup_inst_buffer(&gl, &program, &coords);

    let other_tex = setup_tex2(&gl);

    // Below allows transparency to work
    // http://learnwebgl.brown37.net/11_advanced_rendering/alpha_blending.html
    gl.enable(WebGl2RenderingContext::BLEND);
    gl.blend_func(
        WebGl2RenderingContext::SRC_ALPHA,
        WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
    );

    // Lessons learned so far
    // Make sure to not constantly upload textures to buffer otherwise you run into memory issues
    //      VRAM was full -> spilled into RAM
    // Make sure to set uniform calls before draw every time or at least once per draw time
    //      It is set up per program and doesn't change by default
    *g.borrow_mut() = Some(Closure::new(move || {
        let now = now();
        while fps_timing.len() > 0 && *fps_timing.get(0).unwrap() <= now - 1000.0 {
            fps_timing.pop_front();
        }
        fps_timing.push_back(now);
        fps_node.set_node_value(Some(&fps_timing.len().to_string()));
        draw_calls = 0;
        gl.clear(
            WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT,
        );
        refresh_view(&gl, &canvas);
        // draw_triangle(&gl, &program);
        // draw_triangle_at_coords(&gl, &program, (200.0, 200.0));
        // way to dynamically control the number of draws easily
        let max = 1359540u32;
        let area = canvas.client_width() * canvas.client_height();
        let pct = (area as f32) / (max as f32);

        if constants::INSTANCED_DRAW {
            // let coords: Vec<f32> = vec![200.0; (500f32 * pct) as usize];
            // for _ in 0..(1f32 * pct * 2f32) as u32 {
            getrandom(&mut bytes).expect("random number generation failed");

            let coords: Vec<f32> = bytes
                .chunks_exact(1) // Each chunk represents one f32 (4 bytes)
                .map(|chunk| {
                    let num = chunk[0];
                    let normalized = num as f32 / u8::MAX as f32;
                    min + ((f_max - min) * normalized)
                })
                .collect();
            // draw_triangle_at_coords_instanced(&gl, &program, &coords, origin, w, h, &bitmap);

            draw_triangle_at_coords_instanced_optimized(&gl, &program, &coords, &buf_insta);
            draw_calls += (coords.len() / 2usize) as u32;
            
            // }
        } else {
            getrandom(&mut bytes).expect("random number generation failed");

            let coords: Vec<f32> = bytes
                .chunks_exact(1) // Each chunk represents one f32 (4 bytes)
                .map(|chunk| {
                    let num = chunk[0];
                    let normalized = num as f32 / u8::MAX as f32;
                    min + ((f_max - min) * normalized)
                })
                .collect();
            for i in 0..(500f32 * pct) as u32 {
                draw_triangle_at_coords_optimized(&gl, &coords[(i as usize)..(i as usize)+2], &program, &buf_vert, w, h, origin);
                // upload_tex(&gl, &texture, w, h, &bitmap);
                draw_calls += 1;
            }
            // for _ in 0..(250f32 * pct * 2f32) as u32 {
            //     draw_triangle_at_coords(&gl, &program, (0.0, 0.0));
            //     draw_calls += 1;
            // }
        }


        let u_texture = gl.get_uniform_location(&program, "u_image").unwrap();


        if pct > 1.0 {
            set_active_tex(&gl, WebGl2RenderingContext::TEXTURE0, &texture, &u_texture, 0);
            msg_node.set_node_value(Some(&format!("{} {} {} {} t0", pct, WebGl2RenderingContext::ACTIVE_TEXTURE, WebGl2RenderingContext::TEXTURE0, WebGl2RenderingContext::TEXTURE1)));
        } else {
            set_active_tex(&gl, WebGl2RenderingContext::TEXTURE1, &other_tex, &u_texture, 1);
            msg_node.set_node_value(Some(&format!("{} {} {} {} t1", pct, WebGl2RenderingContext::ACTIVE_TEXTURE, WebGl2RenderingContext::TEXTURE0, WebGl2RenderingContext::TEXTURE1)));
        }

        let mut prefix = "SINGLE";
        if constants::INSTANCED_DRAW {
            prefix = "INSTANCED";
        }
        draw_calls_node.set_node_value(Some(&format!("{} {}", prefix, &draw_calls.to_string())));

        // draw_triangle_at_coords_instanced(&gl, &program_i, &[0.0, 0.0, 400.0, 400.0, 500.0, 200.0]);
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn now() -> f64 {
    window().performance().unwrap().now()
}

fn refresh_view(gl: &WebGl2RenderingContext, canvas: &HtmlCanvasElement) {
    canvas.set_width(canvas.client_width() as u32);
    canvas.set_height(canvas.client_height() as u32);
    // Necessary because canvas is resized
    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
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

pub fn setup_tex2(gl: &WebGl2RenderingContext) -> WebGlTexture {
    let bitmap: Vec<u8>;
    let w: u16;
    let h: u16;
    let mut origin: (f32, f32) = (0.0, 0.0);
    let node = COMPLETE_HASH_MAP.get().unwrap().read().unwrap();
    // print(&format!("{:?}", node.children["UI.nx"].children["MapLogin.img"].children.keys()));
    // print(&format!("{:?}", node.children["Map.nx"].children["Obj"].children["login.img"].children["Title"].children["signboard"].children["0"].children.keys()));
    let signboard_node = &node.children["Map.nx"].children["Obj"].children["login.img"].children
        ["WorldSelect"]
        .children["signboard"]
        .children["0"]
        .children["0"];
    match signboard_node.data {
        NodeDataPopulated::Bitmap {
            data: _,
            width,
            height,
        } => {
            bitmap = signboard_node.data.decompress().unwrap();
            w = width.clone(); // 368 px
            h = height.clone(); // 236 px
        }
        _ => {
            panic!("Signboard node is not populated");
        }
    }

    match signboard_node.children["origin"].data {
        NodeDataPopulated::Vector(x, y) => {
            origin.0 = x as f32;
            origin.1 = y as f32;
        }
        _ => {
            panic!("Missing origin");
        }
    }
    gl.active_texture(WebGl2RenderingContext::TEXTURE1);

    let tex = gl.create_texture().unwrap();

    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&tex));
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::LINEAR as i32,
    );
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

    tex
}

pub fn set_active_tex(gl: &WebGl2RenderingContext, texture: u32, gl_tex: &WebGlTexture, u_tex: &WebGlUniformLocation, u_val: i32) {
    gl.active_texture(texture);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(gl_tex));

    gl.uniform1i(Some(&u_tex), u_val);
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

/// The below function does the following:
/// Takes in one coordinate and draws 2 of the login signboards, one at the proper origin and
/// another one right below it and ignoring the origin
/// This func uses two sets of vertices and texture coordinates for the underlying triangles and texture respectively
/// Finally performs 2 separate draw calls, the latter of which with an offset of 6 indices
pub unsafe fn draw_triangle_at_coords(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
    coords: (f32, f32),
) {
    let canvas = gl
        .canvas()
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    let canvas_h: f32 = canvas.client_height() as f32;
    let canvas_w: f32 = canvas.client_width() as f32;
    let bitmap: Vec<u8>;
    let w: u16;
    let h: u16;
    // let coords: (f32, f32) = (0.0, 0.0);
    let mut origin: (f32, f32) = (0.0, 0.0);

    let node = COMPLETE_HASH_MAP.get().unwrap().read().unwrap();
    // print(&format!("{:?}", node.children["UI.nx"].children["MapLogin.img"].children.keys()));
    // print(&format!("{:?}", node.children["Map.nx"].children["Obj"].children["login.img"].children["Title"].children["signboard"].children["0"].children.keys()));
    let signboard_node = &node.children["Map.nx"].children["Obj"].children["login.img"].children
        ["Title"]
        .children["signboard"]
        .children["0"]
        .children["0"];
    match signboard_node.data {
        NodeDataPopulated::Bitmap {
            data: _,
            width,
            height,
        } => {
            bitmap = signboard_node.data.decompress().unwrap();
            w = width.clone(); // 368 px
            h = height.clone(); // 236 px
        }
        _ => {
            panic!("Signboard node is not populated");
        }
    }

    match signboard_node.children["origin"].data {
        NodeDataPopulated::Vector(x, y) => {
            origin.0 = x as f32;
            origin.1 = y as f32;
        }
        _ => {
            panic!("Missing origin");
        }
    }

    // print(&format!("{:?}", signboard_node.children.keys()));

    // Calculated vertices
    // Definitions
    //     Origin: the center/ starting point for the texture to be drawn
    // Calculation is as follows
    // Texture Rectangle Dimensions = Image's actual H x W
    // Texture Rectangle Coords = Defined Coords - Origin
    let vertices: Vec<f32> = vec![
        coords.0 + w as f32 - origin.0,
        coords.1 - origin.1, // top right
        coords.0 - origin.0,
        coords.1 + h as f32 - origin.1, // bottom left
        coords.0 + w as f32 - origin.0,
        coords.1 + h as f32 - origin.1, // bottom right
        coords.0 + w as f32 - origin.0,
        coords.1 - origin.1, // top right
        coords.0 - origin.0,
        coords.1 + h as f32 - origin.1, // bottom left
        coords.0 - origin.0,
        coords.1 - origin.1, // top left
        // Second set of triangles
        coords.0 + w as f32,
        coords.1 + h as f32, // top right
        coords.0,
        coords.1 + h as f32 + h as f32, // bottom left
        coords.0 + w as f32,
        coords.1 + h as f32 + h as f32, // bottom right
        coords.0 + w as f32,
        coords.1 + h as f32, // top right
        coords.0,
        coords.1 + h as f32 + h as f32, // bottom left
        coords.0,
        coords.1 + h as f32, // top left
    ];

    // Texture coordinates refer to coordinates on texture image
    // 0,0 starts at bottom left and top right is 1,1
    let texture_coords: [f32; 24] = [
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        1.0, 0.0, // bottom right
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        0.0, 1.0, // top left
        // second set of texture coords
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        1.0, 0.0, // bottom right
        1.0, 1.0, // top right
        0.0, 0.0, // bottom left
        0.0, 1.0, // top left
    ];

    // Attributes here refer to the input parameters in the glsl scripts above
    // Gets location within the program so that the attributes can be used
    let att_a_position: u32 = gl.get_attrib_location(program, "a_position") as u32;
    let tex_coord_attr_loc = gl.get_attrib_location(&program, "a_texCoord");
    let u_canvas_size = gl.get_uniform_location(&program, "u_canvas_size").unwrap();

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
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &js_sys::Float32Array::view(&vertices),
        WebGl2RenderingContext::STATIC_DRAW, // not sure what this changes because MDN docs don't fully explain
    );

    // Enables attribute because attributes are disabled by default
    gl.enable_vertex_attrib_array(att_a_position);
    // Binds buffer that's current bound to ARRAY_BUFFER to vertex buffer object
    // https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/vertexAttribPointer
    gl.vertex_attrib_pointer_with_i32(
        att_a_position,
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

    // Create texture object to hold the actual texture data
    let texture = gl.create_texture().expect("Cannot create gl texture");
    // Binds texture object to TEXTURE_2D (binding point)
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
    // Below essentially turns off mipmaps- this is needed to perform tex_image_2d call
    // tex_parameteri sets texture parameters
    // TEXTURE_MIN_FILTER defines filtering method for when textures are scaled down
    // LINEAR essentially tells WebGL to not use any mipmaps
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::LINEAR as i32,
    );

    // Populate texture object through bind point (TEXTURE_2D)
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
    gl.generate_mipmap(WebGl2RenderingContext::TEXTURE_2D);

    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 6, 6);
    // Look into testing instanced draw calls for multiple texture draws
    // gl.draw_arrays_instanced(WebGl2RenderingContext::TRIANGLES, 0, 6, 2);

    // let vertices: Vec<f32> = vec![
    //     origin.0 + w as f32, origin.1 + h as f32, // top right
    //     origin.0, origin.1 + h as f32 + h as f32, // bottom left
    //     origin.0 + w as f32, origin.1 + h as f32 + h as f32, // bottom right
    //     origin.0 + w as f32, origin.1 + h as f32, // top right
    //     origin.0, origin.1 + h as f32 + h as f32, // bottom left
    //     origin.0, origin.1 + h as f32, // top left
    // ];
    // let buf_vert = gl.create_buffer().unwrap();
    // gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_vert));
    // gl.buffer_data_with_array_buffer_view(
    //     WebGl2RenderingContext::ARRAY_BUFFER,
    //     &js_sys::Float32Array::view(&vertices),
    //     WebGl2RenderingContext::STATIC_DRAW, // not sure what this changes because MDN docs don't fully explain
    // );
    // gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
}

/// The below function does the following:
/// Draws two triangles either combined or separated at a specific location in pixel space (not clip space)
/// Texture is loaded onto triangles and aspect ratio is not respected
pub unsafe fn draw_triangle(gl: &WebGl2RenderingContext, program: &WebGlProgram) {
    let canvas = gl
        .canvas()
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    let canvas_h: f32 = canvas.client_height() as f32;
    let canvas_w: f32 = canvas.client_width() as f32;

    // Separated triangles
    let vertices: Vec<f32> = vec![
        200.0, 0.0, // top right
        0.0, 200.0, // bottom left
        200.0, 200.0, // bottom right
        200.0, 200.0, // top right
        0.0, 400.0, // bottom left
        0.0, 200.0, // top left
    ];

    // Combined triangles
    let vertices: Vec<f32> = vec![
        200.0, 0.0, // top right
        0.0, 200.0, // bottom left
        200.0, 200.0, // bottom right
        200.0, 0.0, 0.0, 200.0, 0.0, 0.0,
    ];

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

    let buf_tex = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buf_tex));
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

    let texture = gl.create_texture().expect("Cannot create gl texture");
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
    gl.enable(WebGl2RenderingContext::BLEND);
    gl.blend_func(
        WebGl2RenderingContext::SRC_ALPHA,
        WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        WebGl2RenderingContext::LINEAR as i32,
    );

    let node = COMPLETE_HASH_MAP.get().unwrap().read().unwrap();
    // print(&format!("{:?}", node.children["UI.nx"].children["MapLogin.img"].children.keys()));
    // print(&format!("{:?}", node.children["Map.nx"].children["Obj"].children["login.img"].children["Title"].children["signboard"].children["0"].children.keys()));
    let signboard_node = &node.children["Map.nx"].children["Obj"].children["login.img"].children
        ["Title"]
        .children["signboard"]
        .children["0"]
        .children["0"]
        .data;
    let bitmap: Vec<u8>;
    let w: u16;
    let h: u16;
    match signboard_node {
        NodeDataPopulated::Bitmap {
            data,
            width,
            height,
        } => {
            bitmap = signboard_node.decompress().unwrap();
            w = width.clone();
            h = height.clone();
        }
        _ => {
            panic!("Signboard node is not populated");
        }
    }
    // print(&format!("{:?}", signboard_node));
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

    gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
}

pub async fn websocket(tnh: &Arc<Mutex<WSResponse>>) -> WebSocket {
    let ws = WebSocket::new(constants::WS_URL).unwrap();
    print(&format!("Attempting WS conn to {}", constants::WS_URL));
    let tmp_h_m_clone = Arc::clone(tnh);

    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
        let str_msg = e.data().into_serde::<String>().unwrap();
        let mut got_lock = tmp_h_m_clone.try_lock();

        while let Err(_) = got_lock {
            print("Unable to get lock on TMP_NODE, trying again");
            got_lock = tmp_h_m_clone.try_lock();
        }
        let mut tmp_node_changer = got_lock.unwrap();

        if str_msg.starts_with("{") {
            let ser_img = serde_json::from_str::<nx::NodeSH>(&str_msg).unwrap();

            match *tmp_node_changer {
                WSResponse::Empty => {}
                WSResponse::Ok(_) => print("Existing WSResponse found- overwriting"),
                WSResponse::Error(_) => print("Existing WSResponse found- overwriting"),
                WSResponse::Message(_) => print("Existing WSResponse found- overwriting"),
            }
            *tmp_node_changer = WSResponse::Ok(ser_img.clone());
        } else if str_msg.starts_with("ERROR") {
            *tmp_node_changer = WSResponse::Error(str_msg.clone());
        } else {
            // print(&format!("Other MESSAGE ONLY {}", str_msg));
            // Don't need to keep these messages, just print to console
            *tmp_node_changer = WSResponse::Message(str_msg.clone());
        }
    });
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::ErrorEvent| {
        print(&format!("error event: {:?}", e));
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        print("WebSocket Opened");
        print(&format!(
            "Open Time: {:?}",
            window().performance().unwrap().now()
        ));
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    let onclose_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::CloseEvent| {
        print(&format!(
            "Close Time: {:?}",
            window().performance().unwrap().now()
        ));
        print(&format!("CLOSING WS: {:?}", e.to_string()));
    });
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    let mut ws_state = ws.ready_state();
    while ws_state != WebSocket::OPEN {
        // print("Waiting for WebSocket to open");
        ws_state = ws.ready_state();

        match ws_state {
            WebSocket::CLOSING => {
                print("WS closing");
                panic!("WS closed");
            }
            WebSocket::CLOSED => {
                print("WS closed");
                panic!("WS closed");
            }
            WebSocket::OPEN => print("WS is open and ready"),
            WebSocket::CONNECTING => {
                print("WS connecting");
                sleep(250).await;
            }
            _ => {
                unreachable!()
            }
        }
    }

    ws
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

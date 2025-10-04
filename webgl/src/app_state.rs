use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, console, WebGlVertexArrayObject};
use crate::{constants, create_shader, input, link_program, misc, window, DebugStats};

pub(crate) struct ApplicationState {
    pub pressed_keys: Arc<Mutex<HashMap<String, f32>>>,
    pub mouse_position: Arc<Mutex<(i32, i32)>>,

    pub debug_stats: DebugStats,

    pub canvas: HtmlCanvasElement,
    pub gl: WebGl2RenderingContext,

    pub init_setup: bool,
    pub program: WebGlProgram,
    pub buffer_vertex: WebGlBuffer,
    pub vao_vertex: Option<WebGlVertexArrayObject>,
    pub vao_color: Option<WebGlVertexArrayObject>,

    pub draw_setting: u8,
}

impl ApplicationState {
    pub fn new() -> Self {
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
        let buffer_vertex = gl.create_buffer().unwrap();
        let program = gl
            .create_program()
            .ok_or_else(|| String::from("Unable to create program object"))
            .unwrap();
        ApplicationState {
            pressed_keys: Arc::new(Mutex::new(HashMap::new())),
            mouse_position: Arc::new(Mutex::new((0, 0))),
            debug_stats: DebugStats::new(),
            canvas,
            gl,
            init_setup: false,
            program,
            buffer_vertex,
            vao_vertex: None,
            vao_color: None,
            draw_setting: 2,
        }
    }

    pub fn init(&mut self) {
        self.debug_stats.init();
        input::Input::init_down(Arc::clone(&self.pressed_keys));
        input::Input::init_up(Arc::clone(&self.pressed_keys));
        // Input::init_mouse(Arc::clone(&self.mouse_position));
        input::Input::init_focus_lost_blur(Arc::clone(&self.pressed_keys));
        input::Input::init_focus_lost_visibilitychange(Arc::clone(&self.pressed_keys));

        self.reset_canvas();

        // Below allows transparency to work
        // http://learnwebgl.brown37.net/11_advanced_rendering/alpha_blending.html
        self.gl.enable(WebGl2RenderingContext::BLEND);
        self.gl.blend_func(
            WebGl2RenderingContext::SRC_ALPHA,
            WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
        );
    }

    pub fn reset_canvas(&self) {
        self.canvas.set_width(self.canvas.client_width() as u32);
        self.canvas.set_height(self.canvas.client_height() as u32);
        self.gl.viewport(0, 0, self.canvas.width() as i32, self.canvas.height() as i32);

        self.gl.clear_color(0.08, 0.08, 0.08, 1.0);
        self.gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT);
    }

    pub fn set_debug_stats(&mut self) {
        // FPS Counter in HTML https://webgl2fundamentals.org/webgl/lessons/webgl-text-html.html
        let fps_timing = &mut self.debug_stats.fps_timing;
        let time = misc::now();
        while fps_timing.len() > 0 && *fps_timing.get(0).unwrap() <= time - 1000.0 {
            fps_timing.pop_front();
        }
        fps_timing.push_back(time);

        self.debug_stats.calculate_frame_time();

        let fps = self.debug_stats.fps_timing.len().to_string();
        let frame_time = self.debug_stats.frame_time.to_string();
        let est_total_draw_calls = 0;
        let actual_draw_calls = 0;

        self.debug_stats.set_node_val("fps", &fps);
        self.debug_stats.set_node_val("frame_time", &frame_time);
        self.debug_stats.set_node_val("canvas_size", &format!("{}x{}", self.canvas.width(), self.canvas.height()));
        self.debug_stats.set_node_val("draw_calls", &format!("Total: {} Actual: {}", est_total_draw_calls, actual_draw_calls));
        self.debug_stats.set_node_val("input", &format!("{:?}", self.pressed_keys.lock().unwrap()));
    }

    pub fn can_activate_key(key: &str, keys: &mut HashMap<String, f32>, delay: Option<f32>) -> bool {
        let delay: f32 = delay.unwrap_or(constants::KEY_ACTIVATION_DELAY);
        if keys.contains_key(key) {
            let time = misc::now();
            if keys.get(key).unwrap() + delay < time {
                keys.insert(key.to_string(), time);
                return true
            }
        }
        false
    }

    pub fn handle_input(&mut self) {
        let mut keys = self.pressed_keys.lock().unwrap();
        let mut changed_setup = false;

        if Self::can_activate_key("-", &mut keys, Some(250.0)) {
            self.draw_setting = self.draw_setting - 1;
            misc::log(&format!("toggling draw_setting to: {}", self.draw_setting));
            changed_setup = true;
        }

        if Self::can_activate_key("=", &mut keys, Some(250.0)) {
            self.draw_setting = self.draw_setting + 1;
            misc::log(&format!("toggling draw_setting to: {}", self.draw_setting));
            changed_setup = true;
        }

        if Self::can_activate_key("0", &mut keys, Some(250.0)) {
            self.draw_setting = 0;
            misc::log(&format!("toggling draw_setting to: {}", self.draw_setting));
            changed_setup = true;
        }

        if Self::can_activate_key("1", &mut keys, Some(250.0)) {
            self.draw_setting = 1;
            misc::log(&format!("toggling draw_setting to: {}", self.draw_setting));
            changed_setup = true;
        }

        if Self::can_activate_key("2", &mut keys, Some(250.0)) {
            self.draw_setting = 2;
            misc::log(&format!("toggling draw_setting to: {}", self.draw_setting));
            changed_setup = true;
        }

        if changed_setup && self.init_setup {
            self.init_setup = false;
        }
    }

    fn draw_triangle_init(&mut self) {
        let gl = &self.gl;

        let glsl_v = r##"#version 300 es

        // an attribute is an input (in) to a vertex shader.
        // It will receive data from a buffer
        in vec2 a_position;

        // all shaders have a main function
        void main() {
            // the below converts the incoming a_postion values (pixel coordinates)
            // to WebGL's clip space coordinates
            //vec2 pixel_position = a_position / u_canvas_size;
            //vec2 clip_space = (pixel_position * 2.0) - 1.0;
            gl_Position = vec4(a_position, 0.0, 1.0);
        }"##;

        let glsl_f = r##"#version 300 es

        // fragment shaders don't have a default precision so we need
        // to pick one. highp is a good default. It means "high precision"
        precision highp float;

        out vec4 outColor;

        void main() {
            outColor = vec4(0.5, 0.0, 0.5, 1.0);
        }
        "##;
        let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
        let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

        let program = link_program(&gl, &shader_v, &shader_f).unwrap();
        self.program = program;
        gl.use_program(Some(&self.program));

        let att_a_position: u32 = gl.get_attrib_location(&self.program, "a_position") as u32;
        gl.enable_vertex_attrib_array(att_a_position);
        gl.vertex_attrib_pointer_with_i32(
            att_a_position,
            2,
            WebGl2RenderingContext::FLOAT,
            false,
            0,
            0,
        );
        let vertices = vec![0.0, 0.5, -0.5, -0.5, 0.5, -0.5];
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer_vertex));
        unsafe {
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &js_sys::Float32Array::view(&vertices),
                WebGl2RenderingContext::STATIC_DRAW,
            );
            self.init_setup = true;
        }
    }

    pub fn draw_triangle(&mut self) {
        if !self.init_setup {
            self.draw_triangle_init();
        }
        let gl = &self.gl;

        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer_vertex));
        let att_a_position: u32 = gl.get_attrib_location(&self.program, "a_position") as u32;
        gl.vertex_attrib_pointer_with_i32(
            att_a_position,
            2,
            WebGl2RenderingContext::FLOAT,
            false,
            0,
            0,
        );
        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
    }

    fn draw_triangle_vao_init(&mut self) {
        let gl = &self.gl;

        let glsl_v = r##"#version 300 es
        in vec2 a_position;
        void main() {
            gl_Position = vec4(a_position, 0.0, 1.0);
        }"##;

        let glsl_f = r##"#version 300 es
        precision highp float;
        out vec4 outColor;
        void main() {
            outColor = vec4(0.5, 0.5, 0.5, 1.0);
        }
        "##;
        let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
        let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

        let program = link_program(&gl, &shader_v, &shader_f).unwrap();
        self.program = program;
        gl.use_program(Some(&self.program));

        if let None = self.vao_vertex {
            self.vao_vertex = gl.create_vertex_array();
        }
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer_vertex));
        gl.bind_vertex_array(self.vao_vertex.as_ref());
        let att_a_position: u32 = gl.get_attrib_location(&self.program, "a_position") as u32;
        gl.enable_vertex_attrib_array(att_a_position);
        gl.vertex_attrib_pointer_with_i32(
            att_a_position,
            2,
            WebGl2RenderingContext::FLOAT,
            false,
            0,
            0,
        );
        gl.bind_vertex_array(None);
        let vertices = vec![0.0, 0.5, -0.5, -0.5, 0.5, -0.5];
        unsafe {
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &js_sys::Float32Array::view(&vertices),
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        self.init_setup = true;
    }

    pub fn draw_triangle_vao(&mut self) {
        if !self.init_setup {
            self.draw_triangle_vao_init();
        }
        let gl = &self.gl;

        // let buf_vert = gl.create_buffer().unwrap();
        // console::time_with_label("set attr");
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer_vertex));
        gl.bind_vertex_array(self.vao_vertex.as_ref());

        // console::time_end_with_label("set attr");
        // gl.buffer_sub_data_with_i32_and_array_buffer_view(
        //     WebGl2RenderingContext::ARRAY_BUFFER,
        //     0,
        //     &js_sys::Float32Array::view(&vertices),
        // );
        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 3);
        gl.bind_vertex_array(None);
    }

    fn draw_sierpinski_tri_simple_init(&mut self) {
        let gl = &self.gl;

        let glsl_v = r##"#version 300 es
        in vec2 a_position;
        void main() {
            gl_Position = vec4(a_position, 0.0, 1.0);
        }"##;

        let glsl_f = r##"#version 300 es
        precision highp float;
        out vec4 outColor;
        void main() {
            outColor = vec4(0.0, 0.5, 0.5, 1.0);
        }
        "##;
        let shader_v = create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
        let shader_f = create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

        let program = link_program(&gl, &shader_v, &shader_f).unwrap();
        self.program = program;
        gl.use_program(Some(&self.program));

        if let None = self.vao_vertex {
            self.vao_vertex = gl.create_vertex_array();
        }
        gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer_vertex));
        gl.bind_vertex_array(self.vao_vertex.as_ref());
        let att_a_position: u32 = gl.get_attrib_location(&self.program, "a_position") as u32;
        gl.enable_vertex_attrib_array(att_a_position);
        gl.vertex_attrib_pointer_with_i32(
            att_a_position,
            2,
            WebGl2RenderingContext::FLOAT,
            false,
            0,
            0,
        );
        gl.bind_vertex_array(None);
        self.init_setup = true;
    }

    pub fn draw_sierpinski_simple_tri(&mut self) {
        let mut vertices: Vec<f32> = vec![];
        let mut triangle = vec![
            -0.5, -0.5,
            0.5, -0.5,
            0.0, 0.5,
        ];
        vertices.append(&mut triangle.clone());
        calc_triangle(&mut vertices, triangle);
        if !self.init_setup {
            self.draw_sierpinski_tri_simple_init();
            let gl = &self.gl;
            unsafe {
                gl.buffer_data_with_array_buffer_view(
                    WebGl2RenderingContext::ARRAY_BUFFER,
                    &js_sys::Float32Array::view(&vertices),
                    WebGl2RenderingContext::STATIC_DRAW,
                );
            }
            // misc::log(&format!("loop count: {:?}", loop_counts));
            misc::log(&format!("vert: {:?} {}", vertices, vertices.len()));
        }
        let gl = &self.gl;
        gl.bind_vertex_array(self.vao_vertex.as_ref());

        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, (vertices.len()/ 2) as i32);
        gl.bind_vertex_array(None);
    }
}

fn split_x_and_y(vertices: &Vec<f32>) -> (Vec<f32>, Vec<f32>) {
    let mut x: Vec<f32> = vec![];
    let mut y: Vec<f32> = vec![];

    for (i, ele) in vertices.iter().enumerate() {
        if i % 2 == 0 {
            x.push(*ele);
        } else {
            y.push(*ele);
        }
    }

    (x, y)
}

fn get_min_max_vec_f32(vec: Vec<f32>) -> (f32, f32) {
    let min = vec.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Greater)).unwrap();
    let max = vec.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less)).unwrap();
    (*min, *max)
}

fn get_midpoint(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    ((a.0 + b.0)/ 2f32, (a.1 + b.1) / 2f32)
}

fn get_point(vec: &Vec<f32>, idx: usize) -> (f32, f32) {
    (vec[idx * 2], vec[idx * 2 + 1])
}

fn calc_triangle(full_verts: &mut Vec<f32>, triangle: Vec<f32>) {
    // let mut triangle: Vec<f32> = vec![];

    let (x, y) = split_x_and_y(&triangle);
    let (x_min, x_max) = get_min_max_vec_f32(x);
    let (y_min, y_max) = get_min_max_vec_f32(y);
    let diff = y_max - y_min;
    let unit_size = diff/ 2f32;


    let side_mid_point = get_midpoint(get_point(&triangle, 0), get_point(&triangle, 1));
    let new_triangle_0_1 = vec![
        side_mid_point.0 - (unit_size/ 2f32), side_mid_point.1 - unit_size,
        side_mid_point.0 + (unit_size/ 2f32), side_mid_point.1 - unit_size,
        side_mid_point.0, side_mid_point.1,
    ];
    full_verts.append(&mut new_triangle_0_1.clone());

    let side_mid_point = get_midpoint(get_point(&triangle, 1), get_point(&triangle, 2));
    let new_triangle_1_2 = vec![
        side_mid_point.0, side_mid_point.1,
        side_mid_point.0 + unit_size, side_mid_point.1,
        side_mid_point.0 + (unit_size/ 2f32), side_mid_point.1 + unit_size,
    ];
    full_verts.append(&mut new_triangle_1_2.clone());

    let side_mid_point = get_midpoint(get_point(&triangle, 2), get_point(&triangle, 0));
    let new_triangle_2_0 = vec![
        side_mid_point.0 - unit_size, side_mid_point.1,
        side_mid_point.0, side_mid_point.1,
        side_mid_point.0 - (unit_size/ 2f32), side_mid_point.1 + unit_size,
    ];
    full_verts.append(&mut new_triangle_2_0.clone());

    if unit_size > 0.01 {
        calc_triangle(full_verts, new_triangle_0_1);
        calc_triangle(full_verts, new_triangle_1_2);
        calc_triangle(full_verts, new_triangle_2_0.clone());
    }
}
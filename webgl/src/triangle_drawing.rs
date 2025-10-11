use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlVertexArrayObject};
use crate::{misc, app_state};
use getrandom::getrandom;

pub struct TriangleDrawing {
    pub init_setup: bool,
    // pub app_state: &'a app_state::ApplicationState,
    gl: WebGl2RenderingContext,
    pub program: WebGlProgram,
    pub buffer_vertex: WebGlBuffer,
    pub vao_vertex: Option<WebGlVertexArrayObject>,
    pub vao_color: Option<WebGlVertexArrayObject>,
    pub draw_setting: u8,
}

impl TriangleDrawing {
    pub fn new() -> TriangleDrawing {
        let document = misc::window().document().unwrap();
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
        TriangleDrawing {
            init_setup: false,
            gl,
            program,
            buffer_vertex,
            vao_vertex: None,
            vao_color: None,
            draw_setting: 2,
        }
    }

    pub fn update_init(&mut self, init: bool) {
        self.init_setup = init;
    }

    // fn test(&self) {
    //     let mut v: Vec<fn()> = Vec::new();
    //     v.push(self.draw_triangle)
    // }


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
        let shader_v = misc::create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
        let shader_f = misc::create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

        let program = misc::link_program(&gl, &shader_v, &shader_f).unwrap();
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

    fn draw_tri_vao_init(&mut self) {
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
        let shader_v = misc::create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
        let shader_f = misc::create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

        let program = misc::link_program(&gl, &shader_v, &shader_f).unwrap();
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

    /// Draws a basic triangle but using VAOs instead of manually updating each vertex attribute
    pub fn draw_tri_vao(&mut self) {
        if !self.init_setup {
            self.draw_tri_vao_init();
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
        let shader_v = misc::create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
        let shader_f = misc::create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

        let program = misc::link_program(&gl, &shader_v, &shader_f).unwrap();
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

    /// Draws basic sierpinski triangle
    pub fn draw_sierpinski_tri_simple(&mut self) {
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
            // misc::log(&format!("vert: {:?} {}", vertices, vertices.len()));
            misc::log(&format!("total triangles: {}", vertices.len()/ 2));
        }
        let gl = &self.gl;
        gl.bind_vertex_array(self.vao_vertex.as_ref());

        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, (vertices.len()/ 2) as i32);
        gl.bind_vertex_array(None);
    }

    fn draw_tri_random_init(&mut self) {
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
        let shader_v = misc::create_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, glsl_v).unwrap();
        let shader_f = misc::create_shader(&gl, WebGl2RenderingContext::FRAGMENT_SHADER, glsl_f).unwrap();

        let program = misc::link_program(&gl, &shader_v, &shader_f).unwrap();
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

    /// Randomly draws a ton of triangles across the entire canvas
    pub fn draw_tri_random(&mut self) {
        let min = -1f32;
        let f_max = 1f32;
        // produces 1000 f32 because it takes 4 u8 -> 1 f32
        // produces the "central" coordinate for the triangle
        let mut bytes = [0u8; 1000 * 1];
        getrandom(&mut bytes).expect("random number generation failed");
        let coords: Vec<f32> = bytes
            .chunks_exact(1)
            .map(|chunk| {
                let num = chunk[0];
                let normalized = num as f32 / u8::MAX as f32;
                min + ((f_max - min) * normalized)
            })
            .collect();


        let size_min = -0.05f32;
        let size_f_max = 0.05f32;
        // produces deviation for each triangle's points
        let mut size_bytes = [0u8; 1000 * 1 * 3];
        getrandom(&mut size_bytes).expect("random number generation failed");
        let mut size_coords: Vec<f32> = size_bytes
            .chunks_exact(1)
            .map(|chunk| {
                let num = chunk[0];
                let normalized = num as f32 / u8::MAX as f32;
                size_min + ((size_f_max - size_min) * normalized)
            })
            .collect();

        // divides total # of triangles to calculate by 2 as a small optimization
        for i in 0..coords.len()/2 {
            // convert the 1:3 ratio between coords:size_coords
            let x = i*2;
            size_coords[x*3] = size_coords[x*3] + coords[x];
            size_coords[x*3+2] = size_coords[x*3+2] + coords[x];
            size_coords[x*3+4] = size_coords[x*3+4] + coords[x];

            size_coords[x*3+1] = size_coords[x*3+1] + coords[x+1];
            size_coords[x*3+3] = size_coords[x*3+3] + coords[x+1];
            size_coords[x*3+5] = size_coords[x*3+5] + coords[x+1];
        }

        if !self.init_setup {
            self.draw_tri_random_init();
        }
        let gl = &self.gl;
        unsafe {
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &js_sys::Float32Array::view(&size_coords),
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        gl.bind_vertex_array(self.vao_vertex.as_ref());

        gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, (size_coords.len()/ 2) as i32);
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
        calc_triangle(full_verts, new_triangle_2_0);
    }
}
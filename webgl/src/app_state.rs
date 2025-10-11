use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, console, WebGlVertexArrayObject, WebGlShader};
use crate::{constants, input, misc, triangle_drawing};
use crate::triangle_drawing::TriangleDrawing;

pub(crate) struct ApplicationState {
    pub pressed_keys: Arc<Mutex<HashMap<String, f32>>>,
    pub mouse_position: Arc<Mutex<(i32, i32)>>,

    pub debug_stats: DebugStats,

    pub canvas: HtmlCanvasElement,
    pub gl: WebGl2RenderingContext,

    pub init_setup: bool,
    // pub program: WebGlProgram,
    // pub buffer_vertex: WebGlBuffer,
    // pub vao_vertex: Option<WebGlVertexArrayObject>,
    // pub vao_color: Option<WebGlVertexArrayObject>,

    pub draw_setting: u8,
    pub triangle_drawing: TriangleDrawing,
}

impl ApplicationState {
    pub fn new() -> Self {
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
        let mut triangle_drawing = triangle_drawing::TriangleDrawing::new();
        // let buffer_vertex = gl.create_buffer().unwrap();
        // let program = gl
        //     .create_program()
        //     .ok_or_else(|| String::from("Unable to create program object"))
        //     .unwrap();
        ApplicationState {
            pressed_keys: Arc::new(Mutex::new(HashMap::new())),
            mouse_position: Arc::new(Mutex::new((0, 0))),
            debug_stats: DebugStats::new(),
            canvas,
            gl,
            init_setup: false,
            // program,
            // buffer_vertex,
            // vao_vertex: None,
            // vao_color: None,
            draw_setting: constants::DEFAULT_DRAW_SETTING,
            triangle_drawing,
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

        // TODO: replace this with an input-able system
        if keys.len() == 1 {
            let mut key = "".to_owned();
            let mut val = 0u8;
            if let Some((k, v)) = keys.iter().next() {
                match k.parse::<u8>() {
                    Ok(x) => {
                        key = k.clone();
                        val = x;
                    },
                    Err(_) => {},
                }
            }
            if key.len() > 0 {
                if Self::can_activate_key(&key, &mut keys, Some(250.0)) {
                    self.draw_setting = val;
                    misc::log(&format!("toggling draw_setting to: {}", self.draw_setting));
                    changed_setup = true;
                }
            }
        }

        // TODO: figure out better way to pass along the initial setup flag
        if self.triangle_drawing.init_setup {
            self.init_setup = self.triangle_drawing.init_setup;
        }

        if changed_setup && self.init_setup {
            self.init_setup = false;
            self.triangle_drawing.update_init(self.init_setup);
        }
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
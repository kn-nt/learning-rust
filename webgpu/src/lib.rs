mod misc;
mod constants;
mod generic_helpers;
mod triangle_drawing;
mod compute;
mod helpers;

use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement};
use wasm_bindgen::JsCast;

/// # Steps for webgpu
/// ## General Setup
/// - Create canvas -> generate surface from canvas
/// - Create adapter -> use it to get device & queue
///     - Adapter is abstraction to represent physical or virtual GPU
///     - Configure surface
/// ## Shaders
/// - Create shader source -> module
/// - Create pipeline layout -> use for render pipeline
///     - Pipeline Layout is for defining how shaders access resources
///     - Render Pipeline defines which shaders, resources, data, etc. to use and how to output data
/// ## Render
/// - Create render pass descriptor
///     - Render Pass Descriptor describes how one Render Pass should work
///         - Render Pass is a batch of drawing commands
/// - Create encoder
///     - Encoder records GPU commands into a command buffer for the GPU to queue and work on
/// ## Drawing
/// - Using Render Pass, set pipeline & issue draw command
/// ## Submit
/// - Submit work using queue
#[wasm_bindgen(start)]
async fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).expect("Cannot init console_log");
    let instance = wgpu::Instance::default();

    let win = web_sys::window().unwrap();
    let canvas: HtmlCanvasElement = win.document().unwrap().get_element_by_id("canvas").unwrap().dyn_into().unwrap();

    log::info!("{} {}", canvas.client_width(), canvas.client_height());
    canvas.set_width(canvas.client_width() as u32);
    canvas.set_height(canvas.client_height() as u32);

    // Ref for creating surface: https://github.com/gfx-rs/wgpu/discussions/2893#discussioncomment-8762390
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).unwrap();

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }).await.unwrap();

    let (device, queue) = adapter.request_device(&Default::default()).await.expect("Failed to request GPU device");

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps.formats[0];

    surface.configure(&device, &wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: canvas.width(),
        height: canvas.height(),
        present_mode: wgpu::PresentMode::Fifo, // vsync
        desired_maximum_frame_latency: 2,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
    });

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps.formats[0];

    let draw_tri = triangle_drawing::DrawTriangle::new(&device, &queue, &surface, &format);

    // compute::multiply_vec_by_two(&device, &queue).await;

    // draw_tri.draw_triangle();
    // draw_tri.draw_rainbow_triangle();

    let vertex_data: Vec<f32> = vec![
        0.0,  0.5,  // top center
        -0.5, -0.5,  // bottom left
        0.5, -0.5,  // bottom right
        0.5,  0.5,  // top center
        0.5,  0.0,  // bottom left
        1.0,  0.0   // bottom right
    ];
    draw_tri.draw_triangles_with_input(&vertex_data);
    let vertex_data: Vec<f32> = vec![
        -0.5,  0.5,  // top center
        -0.5, 0.0,  // bottom left
        -1.0, 0.0,  // bottom right
    ];
    draw_tri.draw_triangles_with_input(&vertex_data);


    // log::info!("{:?}", format);
}


fn init() {
    misc::debug("Initializing font");
    use png::Decoder;
    let font_data = constants::FONT_ATLAS.to_vec();
    let data = font_data.clone();

    let decoder = Decoder::new(data.as_slice());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let data = &buf[..info.buffer_size()];

    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.chunks(3).flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255]).collect(),
        png::ColorType::Grayscale => data
            .iter()
            .flat_map(|g| [*g, *g, *g, 255])
            .collect(),
        _ => panic!("Unsupported color type {:?}", info.color_type),
    };

    let atlas_w = info.width as f32;
    let atlas_h = info.height as f32;
    // self.webgl.upload_texture_bytes("Font1", &rgba, atlas_w as u16, atlas_h as u16, (0., 0.));
    let atlas_map = parse_font_atlas_map(String::from_utf8(Vec::from(constants::FONT_ATLAS_MAP)).unwrap().as_str());

    for (letter, details) in &atlas_map {
        // if letter.to_string() == "@" {
        //     misc::log(&format!("@ details {:?}", details));
        let gl_name = format!("{}", letter.to_string());
        let w = details["width"].parse::<f32>().unwrap();
        let h = details["height"].parse::<f32>().unwrap();
        let x = details["x"].parse::<f32>().unwrap();
        let y = details["y"].parse::<f32>().unwrap();
        let xoffset = details["xoffset"].parse::<f32>().unwrap();
        let yoffset = details["yoffset"].parse::<f32>().unwrap();
        let texture_coords = [
            x + w, atlas_h - y,
            x, atlas_h - (y + h),
            x + w, atlas_h - (y + h),
            x + w, atlas_h - y,
            x, atlas_h - (y + h),
            x, atlas_h - y
        ].map(|x| x / 256.0);

        // self.webgl.upload_texture_bytes_buffer_reuse_tex(&gl_name, &texture_coords,"Font1", w as u16, h as u16, (xoffset, yoffset));
    }
}


pub fn parse_font_atlas_map(atlas_map: &str) -> HashMap<char, HashMap<String, String>> {
    let mut map = HashMap::new();

    for line in atlas_map
        .lines()
        .filter(|x| x.starts_with("char") && !x.starts_with("chars count=")) {
        // misc::print(line);
        // misc::print(&format!("Parsing {:?}", line.split(" ")));
        let chars = line
            .split(" ")
            .filter(|x| !x.is_empty() && !x.starts_with("char"))
            .map(|x| x.split("=").collect::<Vec<&str>>())
            .collect::<Vec<Vec<&str>>>();
        // .map(|x| x.split("=").collect::<Vec<&str>>()).collect::<Vec<Vec<&str>>>();

        let mut map_details: HashMap<String, String> = HashMap::new();
        // misc::print(&format!("{:?}", chars));
        map_details.insert("x".to_owned(), chars[1][1].to_owned());
        map_details.insert("y".to_owned(), chars[2][1].to_owned());
        map_details.insert("width".to_owned(), chars[3][1].to_owned());
        map_details.insert("height".to_owned(), chars[4][1].to_owned());
        map_details.insert("xoffset".to_owned(), chars[5][1].to_owned());
        map_details.insert("yoffset".to_owned(), chars[6][1].to_owned());
        map_details.insert("xadvance".to_owned(), chars[7][1].to_owned());
        // misc::print(&format!("{:?}", map_details));

        let ascii_char = match chars[0][0] {
            "id" => {
                match generic_helpers::convert_str_u32_to_char(&chars[0][1]) {
                    Some(chr) => chr.to_owned(),
                    None => panic!("Failed to convert id {} to char", chars[0][1]),
                }
            },
            _ => panic!("Unexpected key in position 0 for font atlas")
        };

        map.insert(ascii_char, map_details);
    }

    // misc::log(&format!("{:?}", map));
    map
}
mod misc;
mod constants;
mod generic_helpers;

use std::collections::HashMap;
use log::debug;
use wasm_bindgen::prelude::*;
use web_sys::{console, HtmlCanvasElement};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::js_sys;
use wgpu::{BindGroupEntry, BindingResource, BufferBinding, ColorTargetState, ComputePassDescriptor, FragmentState, Label, LoadOp, Operations, PipelineLayout, RenderPassColorAttachment, ShaderSource, StoreOp, VertexState};
use bytemuck::cast_slice;

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

    // debug(&format!("{:?}", device));
    // debug(&format!("{:?}", device.features()));
    // debug(&format!("{:?}", device.features().features_webgpu));
    // debug(&format!("{:?}", device.features().features_wgpu));
    // debug(&format!("{:?}", adapter.get_info()));

    // log::info!("{:?}", adapter.get_info());

    let navigator = web_sys::window().unwrap().navigator();
    // log::info!("{:?}", device.);

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


    // log::info!("{:?}", format);

    let shader_source = r#"      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        return vec4f(pos[vertexIndex], 0.0, 1.0);
      }

      @fragment fn fs() -> @location(0) vec4f {
        return vec4f(1.0, 0.0, 0.0, 1.0);
      }"#;

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Label::from("shader for hard coded triangle"),

        source: ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Label::from("hardcoded red triangle pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &module,
            // don't need to explicitly do this unless there is more than 1 fn of this type @vertex
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(FragmentState {
            module: &module,
            // don't need to explicitly do this unless there is more than 1 fn of this type @fragment
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(ColorTargetState {
                format,
                // Allows opacity to work
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: Default::default(),
            })],
        }),
        multiview: None,
        cache: None,
    });

    let texture = &surface.get_current_texture().unwrap().texture;
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let render_pass_desc = wgpu::RenderPassDescriptor {
        label: Label::from("basic canvas render pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Label::from("encoder") });

    // need this in separate brackets so pass: RenderPass will be dropped afterward as we need to use encoder again
    {
        let mut pass = encoder.begin_render_pass(&render_pass_desc);
        pass.set_pipeline(&render_pipeline);
        pass.draw(0..3, 0..1);
    }


    // Compute Shader Testing

    let compute_shader_source = r#"@group(0) @binding(0) var<storage, read_write> data: array<f32>;
      @compute @workgroup_size(1) fn computeSomething(
        @builtin(global_invocation_id) id: vec3u
      ) {
        let i = id.x;
        data[i] = data[i] * 2.0;
      }"#;

    let compute_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Label::from("compute shader for math"),
        source: (ShaderSource::Wgsl(compute_shader_source.into())),
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Label::from("compute pipeline"),
        layout: None,
        module: &compute_module,
        // None needed to be declared, only one @compute fn
        entry_point: None,
        compilation_options: Default::default(),
        cache: None,
    });

    let input: Vec<f32> = vec![1., 2., 3., 5.];

    let work_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Label::from("compute work buffer"),
        size: input.len() as u64 * 4,
        // these flags need to match the compute shader data var declaration for storage, read_write
        // note - to read output data we need to make another buffer
        //        we need to map a buffer to read it in js, one buffer cannot be tagged as mappable and STORAGE
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // needs cast_slice because need to convert the data into a byte array
    queue.write_buffer(&work_buffer, 0, cast_slice(&input));

    let result_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Label::from("compute result buffer"),
        size: input.len() as u64 * 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Label::from("compute bind group"),
        layout: &compute_pipeline.get_bind_group_layout(0),
        entries: &[BindGroupEntry { binding: 0, resource: BindingResource::Buffer { 0: BufferBinding {
            buffer: &work_buffer,
            offset: 0,
            size: None,
        } } }],
    });

    let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Label::from("compute encoder") });

    {
        let mut pass = compute_encoder.begin_compute_pass(&ComputePassDescriptor { label: Label::from("compute pass"), timestamp_writes: None });
        pass.set_pipeline(&compute_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // this is where you control the # of times the shader computeSomething fn runs x * y * z times
        pass.dispatch_workgroups(input.len() as u32-1, 1, 1);
    }

    compute_encoder.copy_buffer_to_buffer(&work_buffer, 0, &result_buffer, 0, result_buffer.size());



    queue.submit(Some(encoder.finish()));
    queue.submit(Some(compute_encoder.finish()));


    // result_buffer.map_async()
    let data = read_buffer(&device, &result_buffer, input.len()).await;
    let floats: &[f32] = cast_slice(&data);

    log::info!("{:?}", floats);
}


async fn read_buffer(device: &wgpu::Device, buffer: &wgpu::Buffer, size: usize) -> Vec<u8> {
    use wgpu::{Buffer, MapMode};
    use futures_intrusive::channel::shared::oneshot_channel;
    let slice = buffer.slice(..);
    let (sender, receiver) = oneshot_channel();

    slice.map_async(MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    receiver.receive().await.unwrap().unwrap();

    let data = slice.get_mapped_range().to_vec();

    buffer.unmap(); // always unmap after reading
    data
}


pub fn debug(s: &str) {
    console::debug_1(&s.into());
}


fn init() {
    misc::debug("Initializing font");
    use png::Decoder;
    let mut font_data = constants::FONT_ATLAS.to_vec();
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
        _ => return panic!("Unsupported color type {:?}", info.color_type),
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
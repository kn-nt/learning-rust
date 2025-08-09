use wgpu::{ColorTargetState, FragmentState, Label, LoadOp, Operations, RenderPassColorAttachment, ShaderSource, StoreOp, VertexState};

pub async fn draw_triangle(device: &wgpu::Device, queue: &wgpu::Queue, surface: &wgpu::Surface<'_>, adapter: &wgpu::Adapter) {
    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps.formats[0];

    let shader_source = r#"      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5),  // bottom right
          vec2f( 0.5,  0.5),  // top center
          vec2f( 0.5,  0.0),  // bottom left
          vec2f( 1.0,  0.0)   // bottom right
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
        pass.draw(0..6, 0..1);
    }

    queue.submit(Some(encoder.finish()));

}
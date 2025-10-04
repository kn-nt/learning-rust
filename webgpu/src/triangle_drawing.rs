use wgpu::{ColorTargetState, FragmentState, Label, LoadOp, Operations, RenderPassColorAttachment, ShaderSource, StoreOp, VertexAttribute, VertexFormat, VertexState};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use core::ops::Range;
use crate::helpers;


pub struct DrawTriangle<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    surface: &'a wgpu::Surface<'a>,
    format: &'a wgpu::TextureFormat,
}

impl<'a> DrawTriangle<'a> {
    pub fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue, surface: &'a wgpu::Surface, format: &'a wgpu::TextureFormat) -> Self {
        DrawTriangle {
            device,
            queue,
            surface,
            format
        }
    }

    /// Draws 2 triangles with hard coded clip space coordinates within vertex shader
    pub fn draw_triangle(&self) {
        let shader_source = r#"
      @vertex fn vs(
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

        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Label::from("shader for hard coded triangle"),

            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    format: *self.format,
                    // Allows opacity to work
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: Default::default(),
                })],
            }),
            multiview: None,
            cache: None,
        });

        let texture = &self.surface.get_current_texture().unwrap().texture;
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

        let encoder = helpers::setup_encoder_finish(self.device, &render_pass_desc, &render_pipeline, 0..6, 0..1);

        self.queue.submit(Some(encoder.finish()));

    }


    /// Draws 1 rainbow triangle
    pub fn draw_rainbow_triangle(&self) {
        let shader_source = r#"
        struct VSOutput {
            @builtin(position) pos: vec4f,
            @location(0) color: vec4f
        }

        @vertex
        fn vs(@builtin(vertex_index) vertexIndex : u32) -> VSOutput {
            var out: VSOutput;

            let pos = array(
              vec2f( 0.0,  0.5),  // top center
              vec2f(-0.5, -0.5),  // bottom left
              vec2f( 0.5, -0.5),  // bottom right
            );

            // although there are only 3 colors,
            // interpolation of pixel color creates the rainbow effect
            let color = array(
                vec3f(1.0, 0.0, 0.0),
                vec3f(0.0, 1.0, 0.0),
                vec3f(0.0, 0.0, 1.0),
            );

            out.pos = vec4f(pos[vertexIndex], 0.0, 1.0);
            out.color = vec4f(color[vertexIndex], 1.0);

            return out;
        }

        @fragment
        fn fs(@location(0) color: vec4f) -> @location(0) vec4f {
            return color;
        }"#;

        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Label::from("shader for rainbow triangle"),

            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Label::from("rainbow triangle pipeline"),
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
                    format: *self.format,
                    // Allows opacity to work
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: Default::default(),
                })],
            }),
            multiview: None,
            cache: None,
        });

        let texture = &self.surface.get_current_texture().unwrap().texture;
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

        let encoder = helpers::setup_encoder_finish(self.device, &render_pass_desc, &render_pipeline, 0..3, 0..1);

        self.queue.submit(Some(encoder.finish()));
    }


    pub fn draw_triangles_with_input(&self, vertex_data: &[f32]) {
        // remember to declare as f32s as buffer is expecting that amount of data

        let shader_source = r#"
        struct VSOutput {
            @builtin(position) pos: vec4f,
            //@location(0) color: vec4f
        }

        @vertex
        fn vs(@location(0) position: vec2f) -> VSOutput {
            var out: VSOutput;

            out.pos = vec4f(position, 0.0, 1.0);

            let color = array(
                vec3f(1.0, 0.0, 0.0),
                vec3f(0.0, 1.0, 0.0),
                vec3f(0.0, 0.0, 1.0),
            );
            //out.color = vec4f(1.0, 0.0, 0.0, 1.0);

            return out;
        }

        @fragment
        fn fs() -> @location(0) vec4f {
            return vec4f(1.0, 0.0, 0.0, 1.0);
        }"#;

        // let frame = self.surface.get_current_texture().unwrap();
        // let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let vertex_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Label::from("vertex buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        };

        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Label::from("shader for rainbow triangle"),

            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Label::from("rainbow triangle pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &module,
                // don't need to explicitly do this unless there is more than 1 fn of this type @vertex
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[vertex_buffer_layout],
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
                    format: *self.format,
                    // Allows opacity to work
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: Default::default(),
                })],
            }),
            multiview: None,
            cache: None,
        });

        let texture = &self.surface.get_current_texture().unwrap().texture;
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Label::from("basic canvas render pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    // this describes whether to clear the attachment
                    load: LoadOp::Load,

                    // LoadOp::Clear(wgpu::Color {
                    //     r: 0.0,
                    //     g: 0.0,
                    //     b: 0.0,
                    //     a: 1.0,
                    // }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        // let encoder = helpers::setup_encoder_vertex_finish(&device, &render_pass_desc, &render_pipeline, &vertex_buffer, 0..3, 0..1);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Label::from("encoder") });

        // need this in separate brackets so pass: RenderPass will be dropped afterward as we need to use encoder again
        {
            let mut pass = encoder.begin_render_pass(&render_pass_desc);
            pass.set_pipeline(&render_pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..(vertex_data.len()/ 2) as u32, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}




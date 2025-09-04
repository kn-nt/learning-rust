use std::ops::Range;
use wgpu::Label;

/// Sets up encoder and preps pipeline to be submitted
pub fn setup_encoder_finish(
    device: &wgpu::Device,
    render_pass_desc: &wgpu::RenderPassDescriptor,
    render_pipeline: &wgpu::RenderPipeline,
    vertices: Range<u32>,
    instances: Range<u32>
) -> wgpu::CommandEncoder {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Label::from("encoder") });

    // need this in separate brackets so pass: RenderPass will be dropped afterward as we need to use encoder again
    {
        let mut pass = encoder.begin_render_pass(&render_pass_desc);
        pass.set_pipeline(&render_pipeline);
        pass.draw(vertices, instances);
    }

    encoder
}


pub fn setup_encoder_vertex_finish(
    device: &wgpu::Device,
    render_pass_desc: &wgpu::RenderPassDescriptor,
    render_pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    vertices: Range<u32>,
    instances: Range<u32>
) -> wgpu::CommandEncoder {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Label::from("encoder") });

    // need this in separate brackets so pass: RenderPass will be dropped afterward as we need to use encoder again
    {
        let mut pass = encoder.begin_render_pass(&render_pass_desc);
        pass.set_pipeline(&render_pipeline);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.draw(vertices, instances);
    }

    encoder
}
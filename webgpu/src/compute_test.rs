use bytemuck::cast_slice;
use wgpu::{BindGroupEntry, BindingResource, BufferBinding, ComputePassDescriptor, Label, ShaderSource};
use crate::misc;

pub async fn compute_test(device: &wgpu::Device, queue: &wgpu::Queue) {
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

    queue.submit(Some(compute_encoder.finish()));

    let data = misc::read_buffer(&result_buffer, input.len()).await;
    let floats: &[f32] = cast_slice(&data);

    log::info!("Raw: {:?}", input);
    log::info!("New: {:?}", floats);
}
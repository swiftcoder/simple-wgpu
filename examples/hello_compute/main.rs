use simple_wgpu::{
    BindGroupBuilder, Buffer, CommandEncoder, ComputePipelineBuilder, Context, Dispatch, Shader,
};
use std::str::FromStr;
use wgpu::include_wgsl;

// Indicates a u32 overflow in an intermediate Collatz value
const OVERFLOW: u32 = 0xffffffff;

async fn run() {
    let numbers = if std::env::args().len() <= 1 {
        let default = vec![1, 2, 3, 4];
        println!("No numbers were provided, defaulting to {default:?}");
        default
    } else {
        std::env::args()
            .skip(1)
            .map(|s| u32::from_str(&s).expect("You must pass a list of positive integers!"))
            .collect()
    };

    let steps = execute_gpu(&numbers).await.unwrap();

    let disp_steps: Vec<String> = steps
        .iter()
        .map(|&n| match n {
            OVERFLOW => "OVERFLOW".to_string(),
            _ => n.to_string(),
        })
        .collect();

    println!("Steps: [{}]", disp_steps.join(", "));
    #[cfg(target_arch = "wasm32")]
    log::info!("Steps: [{}]", disp_steps.join(", "));
}

async fn execute_gpu(numbers: &[u32]) -> Option<Vec<u32>> {
    // Instantiates instance of WebGPU
    let instance = wgpu::Instance::default();

    // `request_adapter` instantiates the general connection to the GPU
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;

    // `request_device` instantiates the feature specific connection to the GPU, defining some parameters,
    //  `features` being the available features.
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let info = adapter.get_info();
    // skip this on LavaPipe temporarily
    if info.vendor == 0x10005 {
        return None;
    }

    execute_gpu_inner(device, queue, numbers).await
}

async fn execute_gpu_inner(
    device: wgpu::Device,
    queue: wgpu::Queue,
    numbers: &[u32],
) -> Option<Vec<u32>> {
    let context = Context::new(device, queue);

    // Loads the shader from WGSL
    let cs_module = Shader::new(include_wgsl!("shader.wgsl"), &context);

    // Gets the size in bytes of the buffer.
    let size = numbers.len() * std::mem::size_of::<u32>();

    // Instantiates buffer without data.
    // `usage` of buffer specifies how it can be used:
    //   `BufferUsages::MAP_READ` allows it to be read (outside the shader).
    //   `BufferUsages::COPY_DST` allows it to be the destination of the copy.
    let staging_buffer = Buffer::new(
        None,
        wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        size,
        &context,
    );

    // Instantiates buffer with data (`numbers`).
    // Usage allowing the buffer to be:
    //   A storage buffer (can be bound within a bind group and thus available to a shader).
    //   The destination of a copy.
    //   The source of a copy.
    let storage_buffer = Buffer::with_data(
        Some("Storage Buffer"),
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        bytemuck::cast_slice(numbers),
        &context,
    );

    // A bind group defines how buffers are accessed by shaders.
    // It is to WebGPU what a descriptor set is to Vulkan.
    // `binding` here refers to the `binding` of a buffer in the shader (`layout(set = 0, binding = 0) buffer`).

    // A pipeline specifies the operation of a shader

    // Instantiates the pipeline.
    let compute_pipeline =
        ComputePipelineBuilder::with_entry_point(&cs_module.entry_point("main")).build();

    // Instantiates the bind group,
    let bind_group = BindGroupBuilder::new()
        .buffer(
            0,
            wgpu::ShaderStages::COMPUTE,
            &storage_buffer.storage_binding(false),
            None,
        )
        .build();

    {
        let mut frame = CommandEncoder::new(None, &context);

        {
            let mut cpass = frame.compute_pass(Some("compute"));
            cpass.dispatch(Dispatch {
                bind_groups: vec![bind_group],
                bind_group_offsets: vec![vec![]],
                pipeline: compute_pipeline,
                extent: (numbers.len() as u32, 1, 1), // Number of cells to run, the (x,y,z) size of item being processed
            });
        }

        frame.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);
    }

    // Note that we're not calling `.await` here.
    let buffer_slice = staging_buffer.slice(..);
    let slice = buffer_slice.get();
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    context.device().poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Some(Ok(())) = receiver.receive().await {
        // Gets contents of buffer
        let data = slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result = bytemuck::cast_slice(&data).to_vec();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory

        // Returns data from buffer
        Some(result)
    } else {
        panic!("failed to run compute on gpu!")
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(run());
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        wasm_bindgen_futures::spawn_local(run());
    }
}

use std::sync::Arc;

use simple_wgpu::{
    ColorAttachment, CommandEncoder, Context, DrawCall, RenderPipeline, RenderPipelineBuilder,
    RenderTexture, Shader,
};
use wgpu::{include_wgsl, Surface};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

struct State {
    window: Arc<Window>,
    size: winit::dpi::PhysicalSize<u32>,
    surface_format: wgpu::TextureFormat,
    surface: Surface<'static>,
    context: Context,
    render_pipeline: RenderPipeline,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor::default(),
                None, // Trace path
            )
            .await
            .unwrap();

        let context = Context::new(device, queue);

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        // Load the shaders from disk
        let shader = Shader::new(include_wgsl!("shader.wgsl"), &context);

        let render_pipeline =
            RenderPipelineBuilder::with_vertex(&shader.entry_point("vs_main"), [])
                .fragment(&shader.entry_point("fs_main"), [Some(Default::default())])
                .build();

        let state = State {
            window,
            size,
            surface_format,
            surface,
            context,
            render_pipeline,
        };

        state.configure_surface();

        state
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface
            .configure(&self.context.device(), &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self) {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let target =
            RenderTexture::from_texture_view(&texture_view, &self.surface_format.add_srgb_suffix());

        {
            let mut frame = CommandEncoder::new(None, &self.context);

            {
                let mut rpass = frame.render_pass(
                    None,
                    vec![ColorAttachment {
                        target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                            store: wgt::StoreOp::Store,
                        },
                    }],
                    None,
                    None,
                );

                rpass.draw(DrawCall {
                    bind_groups: vec![],
                    bind_group_offsets: vec![],
                    pipeline: self.render_pipeline.clone(),
                    vertices: vec![],
                    indices: None,
                    element_range: 0..3,
                    instance_range: 0..1,
                    rasteriser_state: Default::default(),
                });
            }
        }

        surface_texture.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Create window object
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                // Emits a new redraw requested event.
                state.window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                // Reconfigures the size of the surface. We do not re-render
                // here as this event is always followed up by redraw request.
                state.resize(size);
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}

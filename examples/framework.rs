use simple_wgpu::{Context, RenderTexture};
use std::sync::Arc;
use wgpu::Surface;
use wgt::TextureFormat;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::{mem::size_of, slice::from_raw_parts};

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

pub trait Example: 'static + Sized {
    fn init(
        config: &wgpu::SurfaceConfiguration,
        adapter: &wgpu::Adapter,
        context: &Context,
    ) -> Self;
    fn resize(&mut self, config: &wgpu::SurfaceConfiguration, context: &Context);
    fn update(&mut self, event: WindowEvent);
    fn render(&mut self, target: &RenderTexture, context: &Context);
}

pub fn run<E: Example>(title: &str) {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::<E> { state: None };
    event_loop.run_app(&mut app).unwrap();
}

struct State<E: Example> {
    window: Arc<Window>,
    size: winit::dpi::PhysicalSize<u32>,
    surface_format: wgpu::TextureFormat,
    surface: Surface<'static>,
    context: Context,
    example: E,
}

fn build_surface_config(
    surface_format: &TextureFormat,
    size: PhysicalSize<u32>,
) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: *surface_format,
        // Request compatibility with the sRGB-format texture view we‘re going to create later.
        view_formats: vec![surface_format.add_srgb_suffix()],
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        width: size.width,
        height: size.height,
        desired_maximum_frame_latency: 2,
        present_mode: wgpu::PresentMode::AutoVsync,
    }
}

impl<E: Example> State<E> {
    async fn new(window: Arc<Window>) -> State<E> {
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

        let config = build_surface_config(&surface_format, size);

        log::info!("Initializing the example...");
        let example = E::init(&config, &adapter, &context);

        let state = State {
            window,
            size,
            surface_format,
            surface,
            context,
            example,
        };

        state.configure_surface();

        state
    }

    fn configure_surface(&self) {
        let surface_config = build_surface_config(&self.surface_format, self.size);
        self.surface
            .configure(&self.context.device(), &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();

        let config = build_surface_config(&self.surface_format, self.size);

        self.example.resize(&config, &self.context);
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

        self.example.render(&target, &self.context);

        surface_texture.present();
    }

    fn update(&mut self, event: WindowEvent) {
        self.example.update(event);
    }
}

struct App<E: Example> {
    state: Option<State<E>>,
}

impl<E: Example> ApplicationHandler for App<E> {
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
            _ => state.update(event),
        }
    }
}

// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}

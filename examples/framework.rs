use simple_wgpu::{Context, RenderTexture};
use std::future::Future;
#[cfg(target_arch = "wasm32")]
use std::str::FromStr;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_sys::{ImageBitmapRenderingContext, OffscreenCanvas};
use winit::{
    event::{self, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
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
    fn optional_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities {
            flags: wgpu::DownlevelFlags::empty(),
            shader_model: wgpu::ShaderModel::Sm5,
            ..wgpu::DownlevelCapabilities::default()
        }
    }
    fn required_limits() -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults() // These downlevel limits will allow the code to run on all possible hardware
    }
    fn init(
        config: &wgpu::SurfaceConfiguration,
        adapter: &wgpu::Adapter,
        context: &Context,
    ) -> Self;
    fn resize(&mut self, config: &wgpu::SurfaceConfiguration, context: &Context);
    fn update(&mut self, event: WindowEvent);
    fn render(&mut self, target: &RenderTexture, context: &Context, spawner: &Spawner);
}

struct Setup {
    window: winit::window::Window,
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    context: Context,
    #[cfg(target_arch = "wasm32")]
    offscreen_canvas_setup: Option<OffscreenCanvasSetup>,
}

#[cfg(target_arch = "wasm32")]
struct OffscreenCanvasSetup {
    offscreen_canvas: OffscreenCanvas,
    bitmap_renderer: ImageBitmapRenderingContext,
}

async fn setup<E: Example>(title: &str) -> Setup {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    };

    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title(title);
    #[cfg(windows_OFF)] // TODO
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        let query_string = web_sys::window().unwrap().location().search().unwrap();
        let level: log::Level = parse_url_query_string(&query_string, "RUST_LOG")
            .and_then(|x| x.parse().ok())
            .unwrap_or(log::Level::Error);
        console_log::init_with_level(level).expect("could not initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
    }

    #[cfg(target_arch = "wasm32")]
    let mut offscreen_canvas_setup: Option<OffscreenCanvasSetup> = None;
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowExtWebSys;

        let query_string = web_sys::window().unwrap().location().search().unwrap();
        if let Some(offscreen_canvas_param) =
            parse_url_query_string(&query_string, "offscreen_canvas")
        {
            if FromStr::from_str(offscreen_canvas_param) == Ok(true) {
                log::info!("Creating OffscreenCanvasSetup");

                let offscreen_canvas =
                    OffscreenCanvas::new(1024, 768).expect("couldn't create OffscreenCanvas");

                let bitmap_renderer = window
                    .canvas()
                    .get_context("bitmaprenderer")
                    .expect("couldn't create ImageBitmapRenderingContext (Result)")
                    .expect("couldn't create ImageBitmapRenderingContext (Option)")
                    .dyn_into::<ImageBitmapRenderingContext>()
                    .expect("couldn't convert into ImageBitmapRenderingContext");

                offscreen_canvas_setup = Some(OffscreenCanvasSetup {
                    offscreen_canvas,
                    bitmap_renderer,
                })
            }
        }
    };

    log::info!("Initializing the surface...");

    let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        dx12_shader_compiler,
    });
    let (size, surface) = unsafe {
        let size = window.inner_size();

        #[cfg(any(not(target_arch = "wasm32"), target_os = "emscripten"))]
        let surface = instance.create_surface(&window).unwrap();
        #[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
        let surface = {
            if let Some(offscreen_canvas_setup) = &offscreen_canvas_setup {
                log::info!("Creating surface from OffscreenCanvas");
                instance
                    .create_surface_from_offscreen_canvas(&offscreen_canvas_setup.offscreen_canvas)
            } else {
                instance.create_surface(&window)
            }
        }
        .unwrap();

        (size, surface)
    };
    let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(&instance, backends, Some(&surface))
            .await
            .expect("No suitable GPU adapters found on the system!");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    let optional_features = E::optional_features();
    let required_features = E::required_features();
    let adapter_features = adapter.features();
    assert!(
        adapter_features.contains(required_features),
        "Adapter does not support required features for this example: {:?}",
        required_features - adapter_features
    );

    let required_downlevel_capabilities = E::required_downlevel_capabilities();
    let downlevel_capabilities = adapter.get_downlevel_capabilities();
    assert!(
        downlevel_capabilities.shader_model >= required_downlevel_capabilities.shader_model,
        "Adapter does not support the minimum shader model required to run this example: {:?}",
        required_downlevel_capabilities.shader_model
    );
    assert!(
        downlevel_capabilities
            .flags
            .contains(required_downlevel_capabilities.flags),
        "Adapter does not support the downlevel capabilities required to run this example: {:?}",
        required_downlevel_capabilities.flags - downlevel_capabilities.flags
    );

    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
    let needed_limits = E::required_limits().using_resolution(adapter.limits());

    let trace_dir = std::env::var("WGPU_TRACE");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: (optional_features & adapter_features) | required_features,
                limits: needed_limits,
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        )
        .await
        .expect("Unable to find a suitable GPU adapter!");

    let context = Context::new(device, queue);

    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        context,
        #[cfg(target_arch = "wasm32")]
        offscreen_canvas_setup,
    }
}

fn start<E: Example>(
    #[cfg(not(target_arch = "wasm32"))] Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        context,
    }: Setup,
    #[cfg(target_arch = "wasm32")] Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        context,
        offscreen_canvas_setup,
    }: Setup,
) {
    let spawner = Spawner::new();
    let mut config = surface
        .get_default_config(&adapter, size.width, size.height)
        .expect("Surface isn't supported by the adapter.");
    surface.configure(&context.device(), &config);

    log::info!("Initializing the example...");
    let mut example = E::init(&config, &adapter, &context);

    #[cfg(not(target_arch = "wasm32"))]
    let mut last_frame_inst = Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let (mut frame_count, mut accum_time) = (0, 0.0);

    log::info!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter); // force ownership by the closure
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            event::Event::RedrawEventsCleared => {
                #[cfg(not(target_arch = "wasm32"))]
                spawner.run_until_stalled();

                window.request_redraw();
            }
            event::Event::WindowEvent {
                event:
                    WindowEvent::Resized(size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut size,
                        ..
                    },
                ..
            } => {
                log::info!("Resizing to {:?}", size);
                config.width = size.width.max(1);
                config.height = size.height.max(1);
                example.resize(&config, &context);
                surface.configure(&context.device(), &config);
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::Escape),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                #[cfg(not(target_arch = "wasm32"))]
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::R),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    println!("{:#?}", instance.generate_report());
                }
                _ => {
                    example.update(event);
                }
            },
            event::Event::RedrawRequested(_) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    accum_time += last_frame_inst.elapsed().as_secs_f32();
                    last_frame_inst = Instant::now();
                    frame_count += 1;
                    if frame_count == 100 {
                        println!(
                            "Avg frame time {}ms",
                            accum_time * 1000.0 / frame_count as f32
                        );
                        accum_time = 0.0;
                        frame_count = 0;
                    }
                }

                let surface_texture = match surface.get_current_texture() {
                    Ok(surface_texture) => surface_texture,
                    Err(_) => {
                        surface.configure(&context.device(), &config);
                        surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!")
                    }
                };

                let target = RenderTexture::from_surface_texture(&surface_texture);

                example.render(&target, &context, &spawner);

                surface_texture.present();

                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(offscreen_canvas_setup) = &offscreen_canvas_setup {
                        let image_bitmap = offscreen_canvas_setup
                            .offscreen_canvas
                            .transfer_to_image_bitmap()
                            .expect("couldn't transfer offscreen canvas to image bitmap.");
                        offscreen_canvas_setup
                            .bitmap_renderer
                            .transfer_from_image_bitmap(&image_bitmap);

                        log::info!("Transferring OffscreenCanvas to ImageBitmapRenderer");
                    }
                }
            }
            _ => {}
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Spawner<'a> {
    executor: async_executor::LocalExecutor<'a>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> Spawner<'a> {
    fn new() -> Self {
        Self {
            executor: async_executor::LocalExecutor::new(),
        }
    }

    #[allow(dead_code)]
    pub fn spawn_local(&self, future: impl Future<Output = ()> + 'a) {
        self.executor.spawn(future).detach();
    }

    fn run_until_stalled(&self) {
        while self.executor.try_tick() {}
    }
}

#[cfg(target_arch = "wasm32")]
pub struct Spawner {}

#[cfg(target_arch = "wasm32")]
impl Spawner {
    fn new() -> Self {
        Self {}
    }

    #[allow(dead_code)]
    pub fn spawn_local(&self, future: impl Future<Output = ()> + 'static) {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<E: Example>(title: &str) {
    let setup = pollster::block_on(setup::<E>(title));
    start::<E>(setup);
}

#[cfg(target_arch = "wasm32")]
pub fn run<E: Example>(title: &str) {
    use wasm_bindgen::{prelude::*, JsCast};

    let title = title.to_owned();
    wasm_bindgen_futures::spawn_local(async move {
        let setup = setup::<E>(&title).await;
        let start_closure = Closure::once_into_js(move || start::<E>(setup));

        // make sure to handle JS exceptions thrown inside start.
        // Otherwise wasm_bindgen_futures Queue would break and never handle any tasks again.
        // This is required, because winit uses JS exception for control flow to escape from `run`.
        if let Err(error) = call_catch(&start_closure) {
            let is_control_flow_exception = error.dyn_ref::<js_sys::Error>().map_or(false, |e| {
                e.message().includes("Using exceptions for control flow", 0)
            });

            if !is_control_flow_exception {
                web_sys::console::error_1(&error);
            }
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(catch, js_namespace = Function, js_name = "prototype.call.call")]
            fn call_catch(this: &JsValue) -> Result<(), JsValue>;
        }
    });
}

#[cfg(target_arch = "wasm32")]
/// Parse the query string as returned by `web_sys::window()?.location().search()?` and get a
/// specific key out of it.
pub fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
    let query_string = query.strip_prefix('?')?;

    for pair in query_string.split('&') {
        let mut pair = pair.split('=');
        let key = pair.next()?;
        let value = pair.next()?;

        if key == search_key {
            return Some(value);
        }
    }

    None
}

// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}

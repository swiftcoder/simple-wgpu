#[path = "../framework.rs"]
mod framework;

use bytemuck::{Pod, Zeroable};
use simple_wgpu::{
    BindGroup, BindGroupBuilder, Buffer, ColorAttachment, ColorTargetState, CommandEncoder,
    Context, DrawCall, RasteriserState, RenderPipeline, RenderPipelineBuilder, RenderTexture,
    Shader, Texture, VertexBufferLayout,
};
use std::{f32::consts, future::Future, mem, num::NonZeroU32, pin::Pin, task};
use wgpu::include_wgsl;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        _tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        // top (0, 0, 1)
        vertex([-1, -1, 1], [0, 0]),
        vertex([1, -1, 1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([-1, 1, 1], [0, 1]),
        // bottom (0, 0, -1)
        vertex([-1, 1, -1], [1, 0]),
        vertex([1, 1, -1], [0, 0]),
        vertex([1, -1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // right (1, 0, 0)
        vertex([1, -1, -1], [0, 0]),
        vertex([1, 1, -1], [1, 0]),
        vertex([1, 1, 1], [1, 1]),
        vertex([1, -1, 1], [0, 1]),
        // left (-1, 0, 0)
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, 1, 1], [0, 0]),
        vertex([-1, 1, -1], [0, 1]),
        vertex([-1, -1, -1], [1, 1]),
        // front (0, 1, 0)
        vertex([1, 1, -1], [1, 0]),
        vertex([-1, 1, -1], [0, 0]),
        vertex([-1, 1, 1], [0, 1]),
        vertex([1, 1, 1], [1, 1]),
        // back (0, -1, 0)
        vertex([1, -1, 1], [0, 0]),
        vertex([-1, -1, 1], [1, 0]),
        vertex([-1, -1, -1], [1, 1]),
        vertex([1, -1, -1], [0, 1]),
    ];

    let index_data: &[u16] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    (vertex_data.to_vec(), index_data.to_vec())
}

fn create_texels(size: usize) -> Vec<u8> {
    (0..size * size)
        .map(|id| {
            // get high five for recognizing this ;)
            let cx = 3.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
            let cy = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
            let (mut x, mut y, mut count) = (cx, cy, 0);
            while count < 0xFF && x * x + y * y < 4.0 {
                let old_x = x;
                x = x * x - y * y + cx;
                y = 2.0 * old_x * y + cy;
                count += 1;
            }
            count
        })
        .collect()
}

/// A wrapper for `pop_error_scope` futures that panics if an error occurs.
///
/// Given a future `inner` of an `Option<E>` for some error type `E`,
/// wait for the future to be ready, and panic if its value is `Some`.
///
/// This can be done simpler with `FutureExt`, but we don't want to add
/// a dependency just for this small case.
struct ErrorFuture<F> {
    inner: F,
}
impl<F: Future<Output = Option<wgpu::Error>>> Future for ErrorFuture<F> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<()> {
        let inner = unsafe { self.map_unchecked_mut(|me| &mut me.inner) };
        inner.poll(cx).map(|error| {
            if let Some(e) = error {
                panic!("Rendering {e}");
            }
        })
    }
}

struct Example {
    vertex_buf: Buffer,
    index_buf: Buffer,
    index_count: usize,
    bind_group: BindGroup,
    uniform_buf: Buffer,
    pipeline: RenderPipeline,
    pipeline_wire: Option<RenderPipeline>,
}

impl Example {
    fn generate_matrix(aspect_ratio: f32) -> glam::Mat4 {
        let projection = glam::Mat4::perspective_rh(consts::FRAC_PI_4, aspect_ratio, 1.0, 10.0);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(1.5f32, -5.0, 3.0),
            glam::Vec3::ZERO,
            glam::Vec3::Z,
        );
        projection * view
    }
}

impl framework::Example for Example {
    fn optional_features() -> wgt::Features {
        wgt::Features::POLYGON_MODE_LINE
    }

    fn init(
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        context: &Context,
    ) -> Self {
        // Create the vertex and index buffers
        let vertex_size = mem::size_of::<Vertex>();
        let (vertex_data, index_data) = create_vertices();

        let vertex_buf = Buffer::with_data(
            Some("Vertex Buffer"),
            wgpu::BufferUsages::VERTEX,
            bytemuck::cast_slice(&vertex_data),
            context,
        );

        let index_buf = Buffer::with_data(
            Some("Index Buffer"),
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(&index_data),
            context,
        );

        // Create the texture
        let size = 256u32;
        let texels = create_texels(size as usize);
        let texture_extent = wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        };
        let texture = Texture::with_data(
            &wgpu::TextureDescriptor {
                label: None,
                size: texture_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Uint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            &texels,
            NonZeroU32::new(size),
            context,
        );

        // Create other resources
        let mx_total = Self::generate_matrix(config.width as f32 / config.height as f32);
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        let uniform_buf = Buffer::with_data(
            Some("Uniform Buffer"),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            bytemuck::cast_slice(mx_ref),
            context,
        );

        // Create bind group
        let bind_group = BindGroupBuilder::new()
            .buffer(
                0,
                wgpu::ShaderStages::VERTEX,
                &uniform_buf.uniform_binding(),
                None,
            )
            .texture(1, wgpu::ShaderStages::FRAGMENT, &texture.texture_binding())
            .build();

        let shader = Shader::new(include_wgsl!("shader.wgsl"), context);

        let vertex_buffers = [VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: vec![
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 4,
                    shader_location: 1,
                },
            ],
        }];

        let pipeline = RenderPipelineBuilder::with_vertex(
            &shader.entry_point("vs_main"),
            vertex_buffers.clone(),
        )
        .fragment(&shader.entry_point("fs_main"), [Some(Default::default())])
        .build();

        let pipeline_wire = if context
            .device()
            .features()
            .contains(wgt::Features::POLYGON_MODE_LINE)
        {
            let pipeline_wire =
                RenderPipelineBuilder::with_vertex(&shader.entry_point("vs_main"), vertex_buffers)
                    .fragment(
                        &shader.entry_point("fs_main"),
                        [Some(ColorTargetState {
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    operation: wgpu::BlendOperation::Add,
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                },
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    )
                    .build();

            Some(pipeline_wire)
        } else {
            None
        };

        // Done
        Example {
            vertex_buf,
            index_buf,
            index_count: index_data.len(),
            bind_group,
            uniform_buf,
            pipeline,
            pipeline_wire,
        }
    }

    fn update(&mut self, _event: winit::event::WindowEvent) {
        //empty
    }

    fn resize(&mut self, config: &wgpu::SurfaceConfiguration, context: &Context) {
        let mx_total = Self::generate_matrix(config.width as f32 / config.height as f32);
        let mx_ref: &[f32; 16] = mx_total.as_ref();

        self.uniform_buf
            .write(bytemuck::cast_slice(mx_ref), context);
    }

    fn render(&mut self, target: &RenderTexture, context: &Context, spawner: &framework::Spawner) {
        context
            .device()
            .push_error_scope(wgpu::ErrorFilter::Validation);

        let mut frame = CommandEncoder::new(None, &context);

        {
            let mut rpass = frame.render_pass(
                None,
                vec![ColorAttachment {
                    target: target.clone(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                None,
                Some(Default::default()),
            );

            rpass.draw(DrawCall {
                bind_groups: vec![self.bind_group.clone()],
                bind_group_offsets: vec![vec![]],
                pipeline: self.pipeline.clone(),
                vertices: vec![self.vertex_buf.slice(..)],
                indices: Some(self.index_buf.slice(..)),
                element_range: 0..self.index_count,
                instance_range: 0..1,
                rasteriser_state: RasteriserState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
            });

            if let Some(ref pipe) = self.pipeline_wire {
                rpass.draw(DrawCall {
                    bind_groups: vec![self.bind_group.clone()],
                    bind_group_offsets: vec![vec![]],
                    pipeline: pipe.clone(),
                    vertices: vec![self.vertex_buf.slice(..)],
                    indices: Some(self.index_buf.slice(..)),
                    element_range: 0..self.index_count,
                    instance_range: 0..1,
                    rasteriser_state: RasteriserState {
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Line,
                        ..Default::default()
                    },
                });
            }
        }

        // If an error occurs, report it and panic.
        spawner.spawn_local(ErrorFuture {
            inner: context.device().pop_error_scope(),
        });
    }
}

fn main() {
    framework::run::<Example>("cube");
}

use std::sync::Arc;

use crate::{
    bind_group::BindGroup, context::Context, draw_call::RasteriserState,
    pipeline_layout::PipelineLayout, shader::EntryPoint,
};

/// Describes the layout of a vertex buffer
///
/// Equivalent to [wgpu::VertexBufferLayout]
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct VertexBufferLayout {
    pub array_stride: wgpu::BufferAddress,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

/// Sets blend modes and color masks for a render target
///
/// Loosely equivalent to [wgpu::ColorTargetState]
#[derive(Clone, Default, Hash, PartialEq, Eq, Debug)]
pub struct ColorTargetState {
    pub blend: Option<wgpu::BlendState>,
    pub write_mask: wgpu::ColorWrites,
}

/// A render pipeline
///
/// Loosely equivalent to [wgpu::RenderPipeline],
/// but minus some state that is easier to handle dynamically
#[derive(Clone, Debug)]
pub struct RenderPipeline {
    vertex: (EntryPoint, Vec<VertexBufferLayout>),
    fragment: Option<(EntryPoint, Vec<Option<ColorTargetState>>)>,
    label: Option<String>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub(crate) struct RenderPipelineCacheKey {
    layout: PipelineLayout,
    vertex: (EntryPoint, Vec<VertexBufferLayout>),
    fragment: Option<(EntryPoint, Vec<Option<ColorTargetState>>)>,
    rasteriser_state: RasteriserState,
}

impl RenderPipeline {
    pub(crate) fn get_or_build(
        &self,
        color_formats: &[wgpu::TextureFormat],
        depth_format: Option<wgpu::TextureFormat>,
        multisample: &Option<wgpu::MultisampleState>,
        rasteriser_state: &RasteriserState,
        bind_groups: &[BindGroup],
        context: &Context,
    ) -> Arc<wgpu::RenderPipeline> {
        let layout = PipelineLayout {
            bind_group_layouts: bind_groups.iter().map(|b| b.build_layout()).collect(),
        };

        let mut pipeline_cache = context.ctx.caches.render_pipeline_cache.borrow_mut();

        let key = RenderPipelineCacheKey {
            layout: layout.clone(),
            vertex: self.vertex.clone(),
            fragment: self.fragment.clone(),
            rasteriser_state: rasteriser_state.clone(),
        };

        pipeline_cache
            .get_or_insert_with(key, || {
                let layout = layout.get_or_build(context);

                let buffers = self
                    .vertex
                    .1
                    .iter()
                    .map(|b| wgpu::VertexBufferLayout {
                        array_stride: b.array_stride,
                        step_mode: b.step_mode,
                        attributes: &b.attributes,
                    })
                    .collect::<Vec<_>>();

                let targets = if let Some((_, targets)) = &self.fragment {
                    targets
                        .iter()
                        .zip(color_formats.iter())
                        .map(|(t, f)| {
                            t.as_ref().map(|t| wgpu::ColorTargetState {
                                format: *f,
                                blend: t.blend,
                                write_mask: t.write_mask,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                };

                Arc::new(context.device().create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: self.label.as_deref(),
                        layout: Some(&layout),
                        primitive: wgpu::PrimitiveState {
                            front_face: rasteriser_state.front_face,
                            cull_mode: rasteriser_state.cull_mode,
                            polygon_mode: rasteriser_state.polygon_mode,
                            ..Default::default()
                        },
                        vertex: wgpu::VertexState {
                            module: &self.vertex.0.shader,
                            entry_point: &self.vertex.0.entry_point,
                            buffers: &buffers,
                        },
                        fragment: self.fragment.as_ref().map(|(entry_point, _)| {
                            wgpu::FragmentState {
                                module: &entry_point.shader,
                                entry_point: &entry_point.entry_point,
                                targets: &targets,
                            }
                        }),
                        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                            format,
                            depth_compare: rasteriser_state.depth_compare,
                            depth_write_enabled: rasteriser_state.depth_write,
                            stencil: Default::default(),
                            bias: Default::default(),
                        }),
                        multisample: multisample.unwrap_or_default(),
                        multiview: None,
                    },
                ))
            })
            .clone()
    }
}

/// Builds a [RenderPipeline]
#[derive(Clone)]
pub struct RenderPipelineBuilder {
    vertex: (EntryPoint, Vec<VertexBufferLayout>),
    fragment: Option<(EntryPoint, Vec<Option<ColorTargetState>>)>,
    label: Option<String>,
}

impl RenderPipelineBuilder {
    pub fn with_vertex<I>(entry_point: &EntryPoint, vertex_buffer_layout: I) -> Self
    where
        I: Into<Vec<VertexBufferLayout>>,
    {
        Self {
            vertex: (entry_point.clone(), vertex_buffer_layout.into()),
            fragment: None,
            label: None,
        }
    }

    pub fn vertex<I>(mut self, entry_point: &EntryPoint, vertex_buffer_layout: I) -> Self
    where
        I: Into<Vec<VertexBufferLayout>>,
    {
        self.vertex = (entry_point.clone(), vertex_buffer_layout.into());
        self
    }

    pub fn fragment<I>(mut self, entry_point: &EntryPoint, targets: I) -> Self
    where
        I: Into<Vec<Option<ColorTargetState>>>,
    {
        self.fragment = Some((entry_point.clone(), targets.into()));
        self
    }

    pub fn no_fragment(mut self) -> Self {
        self.fragment = None;
        self
    }

    /// Set the optional debug name. This may appear in error messages and GPU profiler traces
    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn build(self) -> RenderPipeline {
        RenderPipeline {
            vertex: self.vertex,
            fragment: self.fragment,
            label: self.label,
        }
    }
}

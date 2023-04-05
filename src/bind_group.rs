use std::{collections::HashMap, hash::Hash, num::NonZeroU64, sync::Arc};

use crate::{buffer::BufferBinding, context::Context, sampler::Sampler, texture::TextureBinding};

#[derive(Hash, PartialEq, Clone, Eq, Debug)]
pub(crate) struct Binding {
    /// The binding index. Must be unique within a single bind group
    pub binding: usize,
    /// What shader stages the binding will be visible to
    pub visibility: wgpu::ShaderStages,
    /// The resource to bind
    pub resource: BindingResource,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) enum BindingResource {
    Buffer(BufferBinding, Option<usize>),
    Texture(TextureBinding),
    Sampler(Sampler),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct BindGroupLayout {
    layout: Vec<wgpu::BindGroupLayoutEntry>,
}

impl BindGroupLayout {
    pub(crate) fn get_or_build(&self, context: &Context) -> Arc<wgpu::BindGroupLayout> {
        let mut bind_group_layout_cache = context.ctx.caches.bind_group_layout_cache.borrow_mut();

        bind_group_layout_cache
            .get_or_insert_with(self.clone(), || {
                Arc::new(context.device().create_bind_group_layout(
                    &wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries: &self.layout,
                    },
                ))
            })
            .clone()
    }
}

/// A handle to a binding group
///
/// Binding groups let you bind GPU resources to a [DrawCall](crate::DrawCall).
///  
/// The equivalent to [wgpu::BindGroup]

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct BindGroup {
    bindings: Vec<Binding>,
    name: Option<String>,
}

impl BindGroup {
    pub(crate) fn build_layout(&self) -> BindGroupLayout {
        let layout = self
            .bindings
            .iter()
            .map(|b| match &b.resource {
                BindingResource::Sampler(sampler) => wgpu::BindGroupLayoutEntry {
                    binding: b.binding as u32,
                    visibility: b.visibility,
                    ty: wgpu::BindingType::Sampler(sampler.sampler_type()),
                    count: None,
                },
                BindingResource::Texture(texture) => wgpu::BindGroupLayoutEntry {
                    binding: b.binding as u32,
                    visibility: b.visibility,
                    ty: texture.binding_type,
                    count: None,
                },
                BindingResource::Buffer(buffer, _) => wgpu::BindGroupLayoutEntry {
                    binding: b.binding as u32,
                    visibility: b.visibility,
                    ty: wgpu::BindingType::Buffer {
                        ty: buffer.binding_type,
                        has_dynamic_offset: buffer.has_dynamic_offset,
                        min_binding_size: buffer.min_binding_size,
                    },
                    count: None,
                },
            })
            .collect();

        BindGroupLayout { layout }
    }

    pub(crate) fn get_or_build(&self, context: &Context) -> Arc<wgpu::BindGroup> {
        let mut bind_group_cache = context.ctx.caches.bind_group_cache.borrow_mut();

        bind_group_cache
            .get_or_insert_with(self.clone(), || {
                let gpu_layout = self.build_layout().get_or_build(context);

                let mut texture_views = HashMap::new();
                let mut samplers = HashMap::new();

                for b in &self.bindings {
                    match &b.resource {
                        BindingResource::Texture(texture) => {
                            texture_views
                                .insert(&texture.texture, texture.texture.get_or_build(context));
                        }
                        BindingResource::Sampler(sampler) => {
                            samplers.insert(sampler, sampler.get_or_build(context));
                        }
                        _ => {}
                    }
                }

                let gpu_bindings = self
                    .bindings
                    .iter()
                    .map(|b| match &b.resource {
                        BindingResource::Buffer(buffer, size) => wgpu::BindGroupEntry {
                            binding: b.binding as u32,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: buffer.buffer.buffer(),
                                offset: 0,
                                size: size.and_then(|s| NonZeroU64::new(s as u64)),
                            }),
                        },
                        BindingResource::Texture(texture) => wgpu::BindGroupEntry {
                            binding: b.binding as u32,
                            resource: wgpu::BindingResource::TextureView(
                                &texture_views.get(&texture.texture).unwrap(),
                            ),
                        },
                        BindingResource::Sampler(sampler) => wgpu::BindGroupEntry {
                            binding: b.binding as u32,
                            resource: wgpu::BindingResource::Sampler(
                                &samplers.get(sampler).unwrap(),
                            ),
                        },
                    })
                    .collect::<Vec<_>>();

                Arc::new(
                    context
                        .device()
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            label: self.name.as_deref(),
                            layout: &gpu_layout,
                            entries: &gpu_bindings,
                        }),
                )
            })
            .clone()
    }
}

/// Builds a [BindGroup]
pub struct BindGroupBuilder {
    bindings: Vec<Binding>,
    name: Option<String>,
}

impl BindGroupBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            bindings: vec![],
            name: None,
        }
    }

    /// Set the optional debug name. This may appear in error messages and GPU profiler traces
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Bind a [Buffer](crate::Buffer) to this bind group
    pub fn buffer(
        mut self,
        binding: usize,
        visibility: wgpu::ShaderStages,
        buffer: &BufferBinding,
        size: Option<usize>,
    ) -> Self {
        self.bindings.push(Binding {
            binding,
            visibility,
            resource: BindingResource::Buffer(buffer.clone(), size),
        });
        self
    }

    /// Bind a [Texture](crate::Texture) to this bind group
    pub fn texture(
        mut self,
        binding: usize,
        visibility: wgpu::ShaderStages,
        texture: &TextureBinding,
    ) -> Self {
        self.bindings.push(Binding {
            binding,
            visibility,
            resource: BindingResource::Texture(texture.clone()),
        });
        self
    }

    /// Bind a [Sampler](crate::Sampler) to this bind group
    pub fn sampler(
        mut self,
        binding: usize,
        visibility: wgpu::ShaderStages,
        sampler: &Sampler,
    ) -> Self {
        self.bindings.push(Binding {
            binding,
            visibility,
            resource: BindingResource::Sampler(sampler.clone()),
        });
        self
    }

    /// Consume this builder and return a [BindGroup]
    pub fn build(self) -> BindGroup {
        BindGroup {
            bindings: self.bindings,
            name: self.name,
        }
    }
}

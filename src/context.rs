use std::{cell::RefCell, sync::Arc};

use crate::{
    bind_group::{BindGroup, BindGroupLayout},
    compute_pipeline::ComputePipelineCacheKey,
    keyed_cache::KeyedCache,
    pipeline_layout::PipelineLayout,
    render_pipeline::RenderPipelineCacheKey,
    sampler::Sampler,
    texture::Texture,
};

pub(crate) struct Caches {
    pub bind_group_layout_cache: RefCell<KeyedCache<BindGroupLayout, Arc<wgpu::BindGroupLayout>>>,
    pub bind_group_cache: RefCell<KeyedCache<BindGroup, Arc<wgpu::BindGroup>>>,
    pub texture_view_cache: RefCell<KeyedCache<Texture, Arc<wgpu::TextureView>>>,
    pub sampler_cache: RefCell<KeyedCache<Sampler, Arc<wgpu::Sampler>>>,
    pub pipeline_layout_cache: RefCell<KeyedCache<PipelineLayout, Arc<wgpu::PipelineLayout>>>,
    pub render_pipeline_cache:
        RefCell<KeyedCache<RenderPipelineCacheKey, Arc<wgpu::RenderPipeline>>>,
    pub compute_pipeline_cache:
        RefCell<KeyedCache<ComputePipelineCacheKey, Arc<wgpu::ComputePipeline>>>,
}

impl Caches {
    pub(crate) fn age(&self) {
        self.bind_group_layout_cache.borrow_mut().age();
        self.bind_group_cache.borrow_mut().age();
        self.texture_view_cache.borrow_mut().age();
        self.sampler_cache.borrow_mut().age();
        self.pipeline_layout_cache.borrow_mut().age();
        self.render_pipeline_cache.borrow_mut().age();
        self.compute_pipeline_cache.borrow_mut().age();
    }
}

pub(crate) struct PrivateContext {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) caches: Caches,
}

/// Wraps the wgpu [Device](wgpu::Device) and [Queue](wgpu::Queue), and caches all of the wgpu resource types
#[derive(Clone)]
pub struct Context {
    pub(crate) ctx: Arc<PrivateContext>,
}

impl Context {
    /// Create a context from the wgpu [Device](wgpu::Device) and [Queue](wgpu::Queue)
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let caches = Caches {
            bind_group_layout_cache: RefCell::new(KeyedCache::new()),
            bind_group_cache: RefCell::new(KeyedCache::new()),
            texture_view_cache: RefCell::new(KeyedCache::new()),
            sampler_cache: RefCell::new(KeyedCache::new()),
            pipeline_layout_cache: RefCell::new(KeyedCache::new()),
            render_pipeline_cache: RefCell::new(KeyedCache::new()),
            compute_pipeline_cache: RefCell::new(KeyedCache::new()),
        };

        let ctx = PrivateContext {
            device,
            queue,
            caches,
        };

        Self { ctx: Arc::new(ctx) }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.ctx.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.ctx.queue
    }

    pub(crate) fn caches(&self) -> &Caches {
        &self.ctx.caches
    }
}

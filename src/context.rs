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
    pub bind_group_layout_cache: RefCell<KeyedCache<BindGroupLayout, wgpu::BindGroupLayout>>,
    pub bind_group_cache: RefCell<KeyedCache<BindGroup, wgpu::BindGroup>>,
    pub texture_view_cache: RefCell<KeyedCache<Texture, wgpu::TextureView>>,
    pub sampler_cache: RefCell<KeyedCache<Sampler, wgpu::Sampler>>,
    pub pipeline_layout_cache: RefCell<KeyedCache<PipelineLayout, wgpu::PipelineLayout>>,
    pub render_pipeline_cache: RefCell<KeyedCache<RenderPipelineCacheKey, wgpu::RenderPipeline>>,
    pub compute_pipeline_cache: RefCell<KeyedCache<ComputePipelineCacheKey, wgpu::ComputePipeline>>,
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

/// Wraps the wgpu [Device](wgpu::Device) and [Queue](wgpu::Queue), and caches all of the wgpu resource types
#[derive(Clone)]
pub struct Context {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) caches: Arc<Caches>,
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

        Self {
            device,
            queue,
            caches: Arc::new(caches),
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub(crate) fn caches(&self) -> &Caches {
        &self.caches
    }
}

use std::sync::Arc;

use crate::context::Context;

/// A texture sampler
///
/// Samplers configure texture addressing and filtering modes.
///
/// Equivalent to [wgpu::Sampler]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Sampler {
    clamp: bool,
    linear: bool,
    mipmap_linear: bool,
}

impl Sampler {
    pub(crate) fn sampler_type(&self) -> wgpu::SamplerBindingType {
        if self.linear || self.mipmap_linear {
            wgpu::SamplerBindingType::Filtering
        } else {
            wgpu::SamplerBindingType::NonFiltering
        }
    }

    pub(crate) fn get_or_build(&self, context: &Context) -> Arc<wgpu::Sampler> {
        let mut sampler_cache = context.ctx.caches.sampler_cache.borrow_mut();

        let address_mode = if self.clamp {
            wgpu::AddressMode::ClampToEdge
        } else {
            wgpu::AddressMode::Repeat
        };

        let filter = if self.linear {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        };

        let mipmap_filter = if self.mipmap_linear {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        };

        sampler_cache
            .get_or_insert_with(self.clone(), || {
                Arc::new(context.device().create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("mip"),
                    address_mode_u: address_mode,
                    address_mode_v: address_mode,
                    address_mode_w: address_mode,
                    mag_filter: filter,
                    min_filter: filter,
                    mipmap_filter,
                    ..Default::default()
                }))
            })
            .clone()
    }
}

/// Builds a [Sampler]
pub struct SamplerBuilder {
    clamp: bool,
    linear: bool,
    mipmap_linear: bool,
}

impl SamplerBuilder {
    pub fn new() -> Self {
        Self {
            clamp: true,
            linear: true,
            mipmap_linear: true,
        }
    }

    pub fn clamp(mut self) -> Self {
        self.clamp = true;
        self
    }

    pub fn wrap(mut self) -> Self {
        self.clamp = false;
        self
    }

    pub fn linear(mut self) -> Self {
        self.linear = true;
        self
    }
    pub fn nearest(mut self) -> Self {
        self.linear = false;
        self
    }

    pub fn mipmap_linear(mut self) -> Self {
        self.mipmap_linear = true;
        self
    }
    pub fn mipmap_nearest(mut self) -> Self {
        self.mipmap_linear = false;
        self
    }

    pub fn build(self) -> Sampler {
        Sampler {
            clamp: self.clamp,
            linear: self.linear,
            mipmap_linear: self.mipmap_linear,
        }
    }
}

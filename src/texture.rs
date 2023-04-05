use std::{hash::Hash, num::NonZeroU32, sync::Arc};

use uuid::Uuid;

use crate::{context::Context, RenderTexture};

/// A handle to a GPU texture
///
/// The equivalent to [wgpu::Texture]
#[derive(Clone, Debug)]
pub struct Texture {
    id: Uuid,
    texture: Arc<wgpu::Texture>,
    base_mip_level: u32,
    mip_level_count: Option<NonZeroU32>,
    sample_count: u32,
}

/// How to bind a [Texture] to a [BindGroup](crate::BindGroup)
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TextureBinding {
    pub(crate) texture: Texture,
    pub(crate) binding_type: wgpu::BindingType,
}

impl Texture {
    /// Create a new empty texture
    pub fn new(desc: &wgpu::TextureDescriptor, context: &Context) -> Self {
        let texture = context.device().create_texture(desc);

        Self {
            id: Uuid::new_v4(),
            texture: Arc::new(texture),
            base_mip_level: 0,
            mip_level_count: NonZeroU32::new(desc.mip_level_count),
            sample_count: desc.sample_count,
        }
    }

    /// Create a texture from pixel data
    pub fn with_data(
        desc: &wgpu::TextureDescriptor,
        data: &[u8],
        bytes_per_row: Option<NonZeroU32>,
        context: &Context,
    ) -> Self {
        let texture = context.device().create_texture(desc);

        context.queue().write_texture(
            texture.as_image_copy(),
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                // todo: derive automatically from format?
                bytes_per_row,
                rows_per_image: None,
            },
            desc.size,
        );

        Self {
            id: Uuid::new_v4(),
            texture: Arc::new(texture),
            base_mip_level: 0,
            mip_level_count: NonZeroU32::new(desc.mip_level_count),
            sample_count: desc.sample_count,
        }
    }

    pub fn size(&self) -> wgpu::Extent3d {
        self.texture.size()
    }

    pub fn dimension(&self) -> wgpu::TextureDimension {
        self.texture.dimension()
    }

    fn sample_type(&self) -> wgpu::TextureSampleType {
        match self.texture.format() {
            wgpu::TextureFormat::R8Unorm
            | wgpu::TextureFormat::R8Snorm
            | wgpu::TextureFormat::Rg8Unorm
            | wgpu::TextureFormat::Rg8Snorm
            | wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8Snorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb
            | wgpu::TextureFormat::R16Float
            | wgpu::TextureFormat::Rgba16Float
            | wgpu::TextureFormat::Rgb10a2Unorm
            | wgpu::TextureFormat::Rg11b10Float => {
                wgpu::TextureSampleType::Float { filterable: true }
            }
            wgpu::TextureFormat::R8Uint
            | wgpu::TextureFormat::Rg8Uint
            | wgpu::TextureFormat::Rgba8Uint
            | wgpu::TextureFormat::R16Uint
            | wgpu::TextureFormat::Rg16Uint
            | wgpu::TextureFormat::Rgba16Uint
            | wgpu::TextureFormat::R32Uint
            | wgpu::TextureFormat::Rg32Uint
            | wgpu::TextureFormat::Rgba32Uint => wgpu::TextureSampleType::Uint,
            wgpu::TextureFormat::R8Sint
            | wgpu::TextureFormat::Rg8Sint
            | wgpu::TextureFormat::Rgba8Sint
            | wgpu::TextureFormat::R16Sint
            | wgpu::TextureFormat::Rg16Sint
            | wgpu::TextureFormat::Rgba16Sint
            | wgpu::TextureFormat::R32Sint
            | wgpu::TextureFormat::Rg32Sint
            | wgpu::TextureFormat::Rgba32Sint => wgpu::TextureSampleType::Sint,
            _ => wgpu::TextureSampleType::Float { filterable: false },
        }
    }

    pub fn view(&self, base_mip_level: u32, mip_level_count: Option<NonZeroU32>) -> Texture {
        Self {
            id: self.id.clone(),
            texture: self.texture.clone(),
            base_mip_level,
            mip_level_count,
            sample_count: self.sample_count,
        }
    }

    pub fn as_render_texture(&self, context: &Context) -> RenderTexture {
        RenderTexture {
            view: self.get_or_build(context),
            format: self.texture.format(),
        }
    }

    /// Bind this texture for sampling. Must be passed to a [BindGroup](crate::BindGroup)
    #[must_use]
    pub fn texture_binding(&self) -> TextureBinding {
        let view_dimension = match self.texture.dimension() {
            wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
            wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        };

        TextureBinding {
            texture: self.clone(),
            binding_type: wgpu::BindingType::Texture {
                sample_type: self.sample_type(),
                view_dimension,
                multisampled: self.sample_count > 1,
            },
        }
    }

    /// Bind this texture as a storage texture. Must be passed to a [BindGroup](crate::BindGroup)
    #[must_use]
    pub fn storage_binding(&self) -> TextureBinding {
        let view_dimension = match self.texture.dimension() {
            wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
            wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        };

        TextureBinding {
            texture: self.clone(),
            binding_type: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: self.texture.format(),
                view_dimension,
            },
        }
    }

    pub(crate) fn get_or_build(&self, context: &Context) -> Arc<wgpu::TextureView> {
        let mut texture_view_cache = context.ctx.caches.texture_view_cache.borrow_mut();

        texture_view_cache
            .get_or_insert_with(self.clone(), || {
                Arc::new(self.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: None,
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: self.base_mip_level,
                    mip_level_count: self.mip_level_count,
                    base_array_layer: 0,
                    array_layer_count: None,
                }))
            })
            .clone()
    }
}

impl Hash for Texture {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.base_mip_level.hash(state);
        self.mip_level_count.hash(state);
    }
}

impl PartialEq for Texture {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.base_mip_level == other.base_mip_level
            && self.mip_level_count == other.mip_level_count
    }
}

impl Eq for Texture {}

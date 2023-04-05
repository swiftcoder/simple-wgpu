use std::sync::Arc;

/// A texture that can be used as a render pass attachment
#[derive(Clone, Debug)]
pub struct RenderTexture {
    pub(crate) view: Arc<wgpu::TextureView>,
    pub(crate) format: wgpu::TextureFormat,
}

impl RenderTexture {
    /// Create a render texture from a [wgpu::SurfaceTexture]
    ///
    /// This is primarily used to associate the swapchain image with a render pass
    pub fn from_surface_texture(surface_texture: &wgpu::SurfaceTexture) -> Self {
        Self {
            view: Arc::new(
                surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            ),
            format: surface_texture.texture.format(),
        }
    }
}

use crate::{
    command_encoder::{CommandEncoder, Pass},
    draw_call::DrawCall,
    render_texture::RenderTexture,
};

/// A color attachment for a [RenderPass]
///
/// Equivalent to [wgpu::RenderPassColorAttachment]
#[derive(Debug, Clone)]
pub struct ColorAttachment {
    pub target: RenderTexture,
    pub resolve_target: Option<RenderTexture>,
    pub ops: wgpu::Operations<wgpu::Color>,
}

/// A depth/stencil attachment for a [RenderPass]
///
/// Equivalent to [wgpu::RenderPassDepthStencilAttachment]
#[derive(Debug)]
pub struct DepthStencilAttachment {
    pub target: RenderTexture,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

/// Record a render pass
///
/// Create via [`CommandEncoder::render_pass`].
///
/// The equivalent to [wgpu::RenderPass].
pub struct RenderPass<'a> {
    label: Option<String>,
    color_attachments: Vec<ColorAttachment>,
    depth_stencil_attachment: Option<DepthStencilAttachment>,
    multisample: Option<wgpu::MultisampleState>,
    draw_calls: Vec<DrawCall>,
    frame: &'a mut CommandEncoder,
}

impl<'a> RenderPass<'a> {
    pub(crate) fn new(
        label: Option<&str>,
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        multisample: Option<wgpu::MultisampleState>,
        frame: &'a mut CommandEncoder,
    ) -> Self {
        Self {
            label: label.map(|s| s.to_string()),
            color_attachments,
            depth_stencil_attachment,
            multisample,
            draw_calls: vec![],
            frame,
        }
    }

    /// Dispatch a draw call
    pub fn draw(&mut self, draw_call: DrawCall) {
        self.draw_calls.push(draw_call);
    }
}

impl<'a> Drop for RenderPass<'a> {
    fn drop(&mut self) {
        self.frame.passes.push(Pass::Render {
            label: self.label.take(),
            color_attachments: self.color_attachments.drain(..).collect(),
            depth_stencil_attachment: self.depth_stencil_attachment.take(),
            multisample: self.multisample,
            draw_calls: self.draw_calls.drain(..).collect(),
        });
    }
}

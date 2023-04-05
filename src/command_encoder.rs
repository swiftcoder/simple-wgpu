use std::num::NonZeroU64;

use crate::{
    buffer::Buffer,
    compute_pass::ComputePass,
    context::Context,
    dispatch::Dispatch,
    draw_call::DrawCall,
    render_pass::{ColorAttachment, DepthStencilAttachment, RenderPass},
};

#[derive(Debug)]
pub(crate) enum Pass {
    Render {
        label: Option<String>,
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        multisample: Option<wgpu::MultisampleState>,
        draw_calls: Vec<DrawCall>,
    },
    Compute(Option<String>, Vec<Dispatch>),
    ClearBuffer(Buffer, u64, Option<NonZeroU64>),
    CopyBufferToBuffer {
        source: Buffer,
        source_offset: usize,
        destination: Buffer,
        destination_offset: usize,
        size: usize,
    },
}

/// Encodes a series of GPU operations
///
/// Accumulates render passes, compute passes, and GPU transfer commands.
/// No work is submitted to the GPU until the command encoder is dropped.
///
/// This is more or less the equivalent to [wgpu::CommandEncoder]
pub struct CommandEncoder {
    label: Option<String>,
    context: Context,
    pub(crate) passes: Vec<Pass>,
}

impl CommandEncoder {
    pub fn new(label: Option<&str>, context: &Context) -> Self {
        Self {
            label: label.map(|s| s.to_string()),
            context: context.clone(),
            passes: vec![],
        }
    }

    /// Begin a [ComputePass]
    pub fn compute_pass(&mut self, label: Option<&str>) -> ComputePass {
        ComputePass::new(label, self)
    }

    /// Begin a [RenderPass]
    pub fn render_pass(
        &mut self,
        label: Option<&str>,
        color_attachments: Vec<ColorAttachment>,
        depth_stencil_attachment: Option<DepthStencilAttachment>,
        multisample: Option<wgpu::MultisampleState>,
    ) -> RenderPass {
        RenderPass::new(
            label,
            color_attachments,
            depth_stencil_attachment,
            multisample,
            self,
        )
    }

    pub fn clear_buffer(&mut self, buffer: &Buffer, offset: u64, size: Option<NonZeroU64>) {
        self.passes
            .push(Pass::ClearBuffer(buffer.clone(), offset, size));
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        source: &Buffer,
        source_offset: usize,
        destination: &Buffer,
        destination_offset: usize,
        size: usize,
    ) {
        self.passes.push(Pass::CopyBufferToBuffer {
            source: source.clone(),
            source_offset,
            destination: destination.clone(),
            destination_offset,
            size,
        });
    }

    /// Consumes the frame and flushes all pending operations to the GPU
    fn submit(&mut self) {
        let mut encoder =
            self.context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: self.label.as_deref(),
                });

        for p in &self.passes {
            match p {
                Pass::Render {
                    label,
                    color_attachments,
                    depth_stencil_attachment,
                    multisample,
                    draw_calls,
                } => Self::record_render_pass(
                    label,
                    color_attachments,
                    depth_stencil_attachment,
                    multisample,
                    draw_calls,
                    &mut encoder,
                    &self.context,
                ),
                Pass::Compute(label, dispatches) => {
                    Self::record_compute_pass(label, dispatches, &mut encoder, &self.context)
                }
                Pass::ClearBuffer(buffer, offset, size) => {
                    encoder.clear_buffer(buffer.buffer(), *offset, *size)
                }
                Pass::CopyBufferToBuffer {
                    source,
                    source_offset,
                    destination,
                    destination_offset,
                    size,
                } => encoder.copy_buffer_to_buffer(
                    source.buffer(),
                    *source_offset as u64,
                    destination.buffer(),
                    *destination_offset as u64,
                    *size as u64,
                ),
            }
        }

        self.context.queue().submit(Some(encoder.finish()));

        self.context.caches().age();
    }

    fn record_compute_pass(
        label: &Option<String>,
        dispatches: &Vec<Dispatch>,
        encoder: &mut wgpu::CommandEncoder,
        context: &Context,
    ) {
        let bind_groups = dispatches
            .iter()
            .map(|dispatch| {
                dispatch
                    .bind_groups
                    .iter()
                    .map(|bind_group| bind_group.get_or_build(context))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let pipelines = dispatches
            .iter()
            .map(|dispatch| {
                dispatch
                    .pipeline
                    .get_or_build(context, &dispatch.bind_groups)
            })
            .collect::<Vec<_>>();

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: label.as_deref(),
        });

        for (i, dispatch) in dispatches.iter().enumerate() {
            for j in 0..dispatch.bind_groups.len() {
                compute_pass.set_bind_group(
                    j as u32,
                    &bind_groups[i][j],
                    &dispatch.bind_group_offsets[j],
                );
            }

            compute_pass.set_pipeline(&pipelines[i]);

            let (x, y, z) = dispatch.extent;
            compute_pass.dispatch_workgroups(x, y, z);
        }
    }

    fn record_render_pass(
        label: &Option<String>,
        color_attachments: &Vec<ColorAttachment>,
        depth_stencil_attachment: &Option<DepthStencilAttachment>,
        multisample: &Option<wgpu::MultisampleState>,
        draw_calls: &Vec<DrawCall>,
        encoder: &mut wgpu::CommandEncoder,
        context: &Context,
    ) {
        let bind_groups = draw_calls
            .iter()
            .map(|draw_call| {
                draw_call
                    .bind_groups
                    .iter()
                    .map(|bind_group| bind_group.get_or_build(context))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let color_formats = color_attachments
            .iter()
            .map(|c| c.target.format)
            .collect::<Vec<_>>();

        let pipelines = draw_calls
            .iter()
            .map(|draw_call| {
                draw_call.pipeline.get_or_build(
                    &color_formats,
                    depth_stencil_attachment.as_ref().map(|d| d.target.format),
                    multisample,
                    &draw_call.rasteriser_state,
                    &draw_call.bind_groups,
                    context,
                )
            })
            .collect::<Vec<_>>();

        let resolve_targets = color_attachments
            .iter()
            .map(|c| c.resolve_target.as_ref().map(|r| r.view.clone()))
            .collect::<Vec<_>>();

        let color_attachments = color_attachments
            .iter()
            .enumerate()
            .map(|(i, c)| {
                Some(wgpu::RenderPassColorAttachment {
                    view: &c.target.view,
                    resolve_target: resolve_targets[i].as_deref(),
                    ops: c.ops,
                })
            })
            .collect::<Vec<_>>();

        let depth_view = depth_stencil_attachment
            .as_ref()
            .map(|d| d.target.view.clone());

        let desc = wgpu::RenderPassDescriptor {
            label: label.as_deref(),
            color_attachments: &color_attachments,
            depth_stencil_attachment: depth_stencil_attachment.as_ref().map(|d| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view.as_deref().unwrap(),
                    depth_ops: d.depth_ops,
                    stencil_ops: d.stencil_ops,
                }
            }),
        };
        let mut render_pass = encoder.begin_render_pass(&desc);

        for (index, draw_call) in draw_calls.iter().enumerate() {
            for j in 0..draw_call.bind_groups.len() {
                render_pass.set_bind_group(
                    j as u32,
                    &bind_groups[index][j],
                    &draw_call.bind_group_offsets[j],
                );
            }

            render_pass.set_pipeline(&pipelines[index]);

            for (idx, buffer_slice) in draw_call.vertices.iter().enumerate() {
                render_pass.set_vertex_buffer(idx as u32, buffer_slice.get());
            }

            if let Some(buffer_slice) = &draw_call.indices {
                render_pass.set_index_buffer(buffer_slice.get(), wgpu::IndexFormat::Uint16);

                render_pass.draw_indexed(
                    draw_call.element_range.start as u32..draw_call.element_range.end as u32,
                    0,
                    draw_call.instance_range.start as u32..draw_call.instance_range.end as u32,
                );
            } else {
                render_pass.draw(
                    draw_call.element_range.start as u32..draw_call.element_range.end as u32,
                    draw_call.instance_range.start as u32..draw_call.instance_range.end as u32,
                );
            }
        }
    }
}

impl Drop for CommandEncoder {
    fn drop(&mut self) {
        self.submit();
    }
}

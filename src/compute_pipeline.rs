use std::sync::Arc;

use crate::{
    bind_group::BindGroup, context::Context, pipeline_layout::PipelineLayout, shader::EntryPoint,
};

/// A compute pipeline
///
/// Loosely equivalent to [wgpu::ComputePipeline]
#[derive(Clone, Debug)]
pub struct ComputePipeline {
    entry_point: EntryPoint,
    label: Option<String>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub(crate) struct ComputePipelineCacheKey {
    layout: PipelineLayout,
    entry_point: EntryPoint,
}

impl ComputePipeline {
    pub(crate) fn get_or_build(
        &self,
        context: &Context,
        bind_groups: &[BindGroup],
    ) -> Arc<wgpu::ComputePipeline> {
        let layout = PipelineLayout {
            bind_group_layouts: bind_groups.iter().map(|b| b.build_layout()).collect(),
        };

        let key = ComputePipelineCacheKey {
            layout: layout.clone(),
            entry_point: self.entry_point.clone(),
        };

        let mut pipeline_cache = context.ctx.caches.compute_pipeline_cache.borrow_mut();

        pipeline_cache
            .get_or_insert_with(key, || {
                let layout = layout.get_or_build(context);

                Arc::new(context.device().create_compute_pipeline(
                    &wgpu::ComputePipelineDescriptor {
                        layout: Some(&layout),
                        module: &self.entry_point.shader,
                        entry_point: &self.entry_point.entry_point,
                        label: self.label.as_deref(),
                    },
                ))
            })
            .clone()
    }
}

/// Builds a [ComputePipeline]
#[derive(Clone)]
pub struct ComputePipelineBuilder {
    entry_point: EntryPoint,
    label: Option<String>,
}

impl ComputePipelineBuilder {
    pub fn with_entry_point(entry_point: &EntryPoint) -> Self {
        Self {
            entry_point: entry_point.clone(),
            label: None,
        }
    }

    /// Set the optional debug name. This may appear in error messages and GPU profiler traces
    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn build(self) -> ComputePipeline {
        ComputePipeline {
            entry_point: self.entry_point,
            label: self.label,
        }
    }
}

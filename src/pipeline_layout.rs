use crate::{bind_group::BindGroupLayout, context::Context};

#[derive(Clone, Hash, PartialEq, Eq)]
pub(crate) struct PipelineLayout {
    pub(crate) bind_group_layouts: Vec<BindGroupLayout>,
}

impl PipelineLayout {
    pub fn get_or_build(&self, context: &Context) -> wgpu::PipelineLayout {
        let mut pipeline_layout_cache: std::cell::RefMut<
            '_,
            crate::keyed_cache::KeyedCache<PipelineLayout, wgpu::PipelineLayout>,
        > = context.caches.pipeline_layout_cache.borrow_mut();

        pipeline_layout_cache
            .get_or_insert_with(self.clone(), || {
                let bind_group_layouts = self
                    .bind_group_layouts
                    .iter()
                    .map(|layout| layout.get_or_build(context))
                    .collect::<Vec<_>>();
                let bind_group_layout_refs = bind_group_layouts
                    .iter()
                    .map(|layout| layout)
                    .collect::<Vec<_>>();

                context
                    .device()
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &bind_group_layout_refs,
                        push_constant_ranges: &[],
                    })
            })
            .clone()
    }
}

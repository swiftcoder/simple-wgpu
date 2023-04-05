use crate::{bind_group::BindGroup, compute_pipeline::ComputePipeline};

/// All of the data needed to issue a single compute operation
#[derive(Debug)]
pub struct Dispatch {
    pub bind_groups: Vec<BindGroup>,
    pub bind_group_offsets: Vec<Vec<u32>>,
    pub pipeline: ComputePipeline,
    pub extent: (u32, u32, u32),
}

use std::ops::Range;

use crate::{bind_group::BindGroup, buffer::BufferSlice, render_pipeline::RenderPipeline};

/// The set of rendering state that is convenient to vary on a per-draw basis
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RasteriserState {
    pub front_face: wgpu::FrontFace,
    pub cull_mode: Option<wgpu::Face>,
    pub depth_write: bool,
    pub depth_compare: wgpu::CompareFunction,
    pub polygon_mode: wgpu::PolygonMode,
}

impl Default for RasteriserState {
    fn default() -> Self {
        Self {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            depth_write: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            polygon_mode: wgpu::PolygonMode::Fill,
        }
    }
}

/// All of the data needed to issue a single draw call
#[derive(Debug)]
pub struct DrawCall {
    pub bind_groups: Vec<BindGroup>,
    pub bind_group_offsets: Vec<Vec<u32>>,
    pub pipeline: RenderPipeline,
    /// The vertex buffers, if any
    ///
    /// The provided buffers will be bound in order to vertex buffer slots 0..N
    pub vertices: Vec<BufferSlice>,
    /// The index buffer, if any
    ///
    /// If `indices` is `None`, the mesh data will be treated as unindexed
    pub indices: Option<BufferSlice>,
    /// The range of vertices to draw
    pub element_range: Range<usize>,
    /// The range of instances to draw
    ///
    /// You can pass `0..1` to disable instancing
    pub instance_range: Range<usize>,
    /// Additional state that is convenient to vary on a per-draw basis
    pub rasteriser_state: RasteriserState,
}

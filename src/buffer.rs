use std::{
    hash::Hash,
    num::NonZeroU64,
    ops::{Bound, Range, RangeBounds},
    sync::Arc,
};

use uuid::Uuid;
use wgpu::util::DeviceExt;

use crate::context::Context;

#[derive(Debug)]
struct BufferInternal {
    buffer: wgpu::Buffer,
    size: usize,
    usage: wgpu::BufferUsages,
}

/// A handle to a GPU buffer
///
/// The equivalent to [wgpu::Buffer]
#[derive(Clone, Debug)]
pub struct Buffer {
    id: Uuid,
    data: Arc<BufferInternal>,
}

/// How to bind a [Buffer] to a [BindGroup](crate::BindGroup)
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BufferBinding {
    pub(crate) buffer: Buffer,
    pub(crate) binding_type: wgpu::BufferBindingType,
    pub(crate) has_dynamic_offset: bool,
    pub(crate) min_binding_size: Option<NonZeroU64>,
}

impl Buffer {
    /// Create an empty buffer
    pub fn new(
        label: wgpu::Label,
        usage: wgpu::BufferUsages,
        size: usize,
        context: &Context,
    ) -> Self {
        let buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label,
            usage,
            size: size as u64,
            mapped_at_creation: false,
        });

        Self {
            id: Uuid::new_v4(),
            data: Arc::new(BufferInternal {
                buffer,
                size,
                usage,
            }),
        }
    }

    /// Create a buffer and immediately upload data to it
    pub fn with_data(
        label: wgpu::Label,
        usage: wgpu::BufferUsages,
        data: &[u8],
        context: &Context,
    ) -> Self {
        let buffer = context
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label,
                usage,
                contents: data,
            });

        Self {
            id: Uuid::new_v4(),
            data: Arc::new(BufferInternal {
                buffer,
                size: data.len(),
                usage,
            }),
        }
    }

    /// Grow the buffer to `new_size`. Does nothing if the buffer is already larger than `new_size`
    pub fn ensure_capacity(&mut self, new_size: usize, context: &Context) {
        if new_size > self.data.size {
            Arc::get_mut(&mut self.data)
                .map(|data| {
                    data.size = new_size;
                    data.buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        usage: data.usage,
                        size: new_size as u64,
                        mapped_at_creation: false,
                    });
                })
                .expect("couldn't get exclusive access to resize buffer");
        }
    }

    /// Write data to the buffer
    pub fn write(&self, data: &[u8], context: &Context) {
        context.queue().write_buffer(&self.data.buffer, 0, data);
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.data.buffer
    }

    /// Obtain a (sub) slice of the buffer
    pub fn slice<R>(&self, bounds: R) -> BufferSlice
    where
        R: RangeBounds<wgpu::BufferAddress>,
    {
        BufferSlice {
            data: self.data.clone(),
            bounds: constrain_range_to_container_len(bounds, self.data.size as u64),
        }
    }

    /// Bind this buffer as a uniform buffer. Must be passed to a [BindGroup](crate::BindGroup)
    #[must_use]
    pub fn uniform_binding(&self) -> BufferBinding {
        BufferBinding {
            buffer: self.clone(),
            binding_type: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }

    /// Bind this buffer as a storage buffer. Must be passed to a [BindGroup](crate::BindGroup)
    #[must_use]
    pub fn storage_binding(&self, read_only: bool) -> BufferBinding {
        BufferBinding {
            buffer: self.clone(),
            binding_type: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }

    /// See wgpu's [Buffer::unmap](wgpu::Buffer::unmap)
    pub fn unmap(&self) {
        self.data.buffer.unmap();
    }
}

impl Hash for Buffer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Buffer {}

/// A sub-slice of a [Buffer](Buffer)
#[derive(Debug)]
pub struct BufferSlice {
    data: Arc<BufferInternal>,
    bounds: Range<wgpu::BufferAddress>,
}

// todo: figure out how to deal with mapping sanely here
impl BufferSlice {
    /// Get the underlying wgpu [Buffer](wgpu::Buffer). You'll need this to map the contents of the buffer
    pub fn get(&self) -> wgpu::BufferSlice {
        self.data.buffer.slice(self.bounds.clone())
    }
}

fn constrain_range_to_container_len<R>(range: R, container_len: u64) -> Range<u64>
where
    R: RangeBounds<u64>,
{
    let start = match range.start_bound() {
        Bound::Included(t) => *t,
        Bound::Excluded(t) => *t + 1,
        Bound::Unbounded => 0,
    };

    let end = match range.end_bound() {
        Bound::Included(t) => *t + 1,
        Bound::Excluded(t) => *t,
        Bound::Unbounded => container_len,
    };

    start..end
}

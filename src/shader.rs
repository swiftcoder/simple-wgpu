use std::{hash::Hash, sync::Arc};

use crate::context::Context;

/// A handle to a compiled shader
///
/// The equivalent to [`wgpu::ShaderModule`]
#[derive(Clone, Debug)]
pub struct Shader {
    shader: Arc<wgpu::ShaderModule>,
}

impl Shader {
    /// Create a new shader
    ///
    /// It is generally easiest to use [wgpu::include_wgsl] to populate the `desc` argument.
    pub fn new(desc: wgpu::ShaderModuleDescriptor, context: &Context) -> Self {
        Self {
            shader: Arc::new(context.device().create_shader_module(desc)),
        }
    }

    /// Associate the shader with a specific entry point (named main function)
    pub fn entry_point(&self, entry_point: &str) -> EntryPoint {
        EntryPoint {
            shader: self.shader.clone(),
            entry_point: entry_point.to_string(),
        }
    }
}

/// A handle to a compiled shader with a specific main function
#[derive(Clone, Debug)]
pub struct EntryPoint {
    pub(crate) shader: Arc<wgpu::ShaderModule>,
    pub(crate) entry_point: String,
}

impl Eq for EntryPoint {}

impl PartialEq for EntryPoint {
    fn eq(&self, other: &Self) -> bool {
        self.shader.as_ref() as *const wgpu::ShaderModule
            == other.shader.as_ref() as *const wgpu::ShaderModule
            && self.entry_point == other.entry_point
    }
}

impl Hash for EntryPoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.shader.as_ref() as *const wgpu::ShaderModule as usize);
        self.entry_point.hash(state);
    }
}

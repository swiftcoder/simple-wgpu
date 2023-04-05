use crate::{
    command_encoder::{CommandEncoder, Pass},
    dispatch::Dispatch,
};

/// Record a compute pass
///
/// Create via [`CommandEncoder::compute_pass`].
///
/// The equivalent to [wgpu::ComputePass].
pub struct ComputePass<'a> {
    label: Option<String>,
    dispatches: Vec<Dispatch>,
    frame: &'a mut CommandEncoder,
}

impl<'a> ComputePass<'a> {
    pub(crate) fn new(label: Option<&str>, frame: &'a mut CommandEncoder) -> Self {
        Self {
            label: label.map(|s| s.to_string()),
            dispatches: vec![],
            frame,
        }
    }

    /// Dispatch a compute operation
    pub fn dispatch(&mut self, dispatch: Dispatch) {
        self.dispatches.push(dispatch)
    }
}

impl<'a> Drop for ComputePass<'a> {
    fn drop(&mut self) {
        self.frame.passes.push(Pass::Compute(
            self.label.clone(),
            self.dispatches.drain(..).collect(),
        ));
    }
}

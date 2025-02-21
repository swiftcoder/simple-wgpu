# simple-wgpu

An opinionated wrapper around [wgpu-rs](https://github.com/gfx-rs/wgpu), that aims to improve the API ergonomics.

I'm a big fan of the write-once-cross-compile-anywhere promise underlying WebGPU, but I'm less of a fan of the API itself. The WebGPU API is modelled on an earlier, pipeline-centric vision of the Vulkan API, and has inherited a degree of verbosity/inflexibility as a result. This library aims to recapture some of the simplicity of writing code for OpenGL - ideally without giving up the safety and stability guarantees of wgpu-rs.

## Goals
- Reduce the combinatorial pipeline explosion, along the lines of [VK_EXT_extended_dynamic_state](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_EXT_extended_dynamic_state.html)
- Improve DRY (Don't Repeat Yourself) by eliminating the duplicate specification in bind group/pipeline layout objects
- Make pipeline specification render target-agnostic, so the same pipeline can render to different types of target texture

### Non-goals
- Performance. I don't want this wrapper to be slow, but where necessary I will trade performance for ergonomics.
- API Stability (for now). I'm actively iterating on where the boundary between dynamic and baked state should be, as well as whether the more object-oriented wrappers make sense.
- 100% coverage of wgpu features. I'll wrap more functionality as it becomes useful to me.

## Who is it for?

Me, mostly, but if you find it useful, feel free.

## License

I've put this under the Apache license, but if you need a more permissive license, feel free to get in touch.

// ============================================================
// wireframe.wgsl — Bounding box wireframe rendering.
// Standalone shader (common.wgsl NOT prepended).
//
// Bind group 0:
//   [0] uniforms: uniform<WireframeUniform>
// ============================================================

struct WireframeUniform {
    view_proj: mat4x4<f32>,
    grid_size: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> uniforms: WireframeUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) pos: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    let scaled = pos * uniforms.grid_size;
    out.position = uniforms.view_proj * vec4<f32>(scaled, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.4, 0.4, 0.4, 0.6);
}

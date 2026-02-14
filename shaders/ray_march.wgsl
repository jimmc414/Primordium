// ============================================================
// ray_march.wgsl — Full-screen ray marching through 3D volume.
// Standalone shader (common.wgsl NOT prepended).
//
// Bind group 0:
//   [0] volume_tex: texture_3d<f32>
//   [1] tex_sampler: sampler
//   [2] camera: uniform<CameraUniform>
// ============================================================

struct CameraUniform {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,     // xyz = position, w = padding
    grid_size: f32,
    clip_axis: f32,            // -1 = no clip, 0/1/2 = X/Y/Z
    clip_position: f32,        // [0, 1] along axis
    _padding: f32,
};

@group(0) @binding(0) var volume_tex: texture_3d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Full-screen triangle: 3 vertices, no vertex buffer
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    // Generate full-screen triangle covering [-1,1] x [-1,1]
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x, -y) * 0.5 + 0.5;
    return out;
}

// Ray-AABB intersection: returns (tmin, tmax) or tmin > tmax if no hit
fn intersect_aabb(origin: vec3<f32>, inv_dir: vec3<f32>, box_min: vec3<f32>, box_max: vec3<f32>) -> vec2<f32> {
    let t0 = (box_min - origin) * inv_dir;
    let t1 = (box_max - origin) * inv_dir;
    let tmin_v = min(t0, t1);
    let tmax_v = max(t0, t1);
    let tmin = max(max(tmin_v.x, tmin_v.y), tmin_v.z);
    let tmax = min(min(tmax_v.x, tmax_v.y), tmax_v.z);
    return vec2<f32>(tmin, tmax);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let gs = camera.grid_size;

    // Reconstruct ray from inverse view-projection
    let ndc = vec4<f32>(in.uv * 2.0 - 1.0, 0.0, 1.0);
    let ndc_far = vec4<f32>(in.uv * 2.0 - 1.0, 1.0, 1.0);

    let world_near = camera.inv_view_proj * ndc;
    let world_far = camera.inv_view_proj * ndc_far;
    let ray_origin = world_near.xyz / world_near.w;
    let ray_end = world_far.xyz / world_far.w;
    let ray_dir = normalize(ray_end - ray_origin);

    // Intersect with volume AABB [0, grid_size]
    let inv_dir = 1.0 / ray_dir;
    let hit = intersect_aabb(ray_origin, inv_dir, vec3<f32>(0.0), vec3<f32>(gs));

    if hit.x > hit.y {
        // No intersection
        return vec4<f32>(0.02, 0.02, 0.04, 1.0); // dark background
    }

    let t_start = max(hit.x, 0.0);
    let t_end = hit.y;

    // March through volume
    let step_size = 0.5;
    let max_steps = 384;
    var accum = vec4<f32>(0.0);
    var t = t_start;

    for (var i = 0; i < max_steps; i = i + 1) {
        if t >= t_end || accum.a >= 0.95 {
            break;
        }

        let pos = ray_origin + ray_dir * t;
        let uvw = pos / gs;

        // Clip plane rejection
        if camera.clip_axis >= 0.0 {
            let axis = i32(camera.clip_axis);
            let clip_val = camera.clip_position;
            if axis == 0 && uvw.x > clip_val { t += step_size; continue; }
            if axis == 1 && uvw.y > clip_val { t += step_size; continue; }
            if axis == 2 && uvw.z > clip_val { t += step_size; continue; }
        }

        // Sample volume texture
        let sample = textureSampleLevel(volume_tex, tex_sampler, uvw, 0.0);

        // Front-to-back compositing
        if sample.a > 0.01 {
            let src_alpha = sample.a * (1.0 - accum.a);
            accum = vec4<f32>(
                accum.rgb + sample.rgb * src_alpha,
                accum.a + src_alpha
            );
        }

        t += step_size;
    }

    // Blend with background
    let bg = vec3<f32>(0.02, 0.02, 0.04);
    let final_rgb = accum.rgb + bg * (1.0 - accum.a);
    return vec4<f32>(final_rgb, 1.0);
}

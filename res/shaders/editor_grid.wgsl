struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct TransformationUniform {
    matrix: mat4x4<f32>,
    inv_matrix : mat4x4<f32>,
};
@group(2) @binding(0)
var<uniform> transformation: TransformationUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_position = transformation.matrix * vec4<f32>(model.position, 1.0);
    out.world_pos = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    return out;
}

const GRID_SPACING: f32 = 0.1;
const MAJOR_GRID_SPACING: f32 = 1.0;
const FADE_DISTANCE: f32 = 100.0;

const X_AXIS_COLOR: vec3<f32> = vec3<f32>(1.0, 0.0, 0.0);
const Z_AXIS_COLOR: vec3<f32> = vec3<f32>(0.0, 0.0, 1.0);
const GRID_COLOR: vec3<f32> = vec3<f32>(0.2, 0.2, 0.2);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let xz = in.world_pos.xz;

    let distance_from_origin = length(xz);
    let grid_alpha = 0.4 - smoothstep(
        20.0,
        FADE_DISTANCE,
        distance_from_origin
    );

    let uv_minor = xz / GRID_SPACING;
    let fw_minor = fwidth(uv_minor);
    let frac_minor = fract(uv_minor - 0.5) - 0.5;
    let dist_minor = min(abs(frac_minor.x), abs(frac_minor.y)) / max(fw_minor.x, fw_minor.y);
    let minor_line = 0.4 - clamp(dist_minor, 0.0, 1.0);

    let uv_major = xz / MAJOR_GRID_SPACING;
    let fw_major = fwidth(uv_major);
    let frac_major = fract(uv_major - 0.5) - 0.5;
    let dist_major = min(abs(frac_major.x), abs(frac_major.y)) / max(fw_major.x, fw_major.y);
    let major_line = 0.8 - clamp(dist_major, 0.0, 1.0);

    let line_intensity = max(minor_line * 0.5, major_line);

    let axis_threshold = 0.02;
    let x_axis_mask = 1.0 - smoothstep(0.0, axis_threshold, abs(xz.y));
    let z_axis_mask = 1.0 - smoothstep(0.0, axis_threshold, abs(xz.x));

    var final_color = GRID_COLOR * line_intensity;
    var final_alpha = grid_alpha;

    if (x_axis_mask > z_axis_mask) {
        final_color = X_AXIS_COLOR;
    } else if (z_axis_mask > x_axis_mask) {
        final_color = Z_AXIS_COLOR;
    }

    return vec4<f32>(final_color, final_alpha);
}

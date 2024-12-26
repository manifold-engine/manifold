// Currently it'a a copy of basic.wgsl, later it will be updated to be a hyperbolic shader

struct CameraUniform {
    view_proj : mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera : CameraUniform;

struct TransformationUniform {
    matrix : mat4x4<f32>,
    inv_matrix : mat4x4<f32>,
};
@group(2) @binding(0)
var<uniform> transformation : TransformationUniform;

struct VertexInput {
    @location(0) position : vec3<f32>,
    @location(1) tex_coords : vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position : vec4<f32>,
    @location(0) tex_coords : vec2<f32>,
}

@vertex
fn vs_main(model : VertexInput) -> VertexOutput {
    var out : VertexOutput;
    out.tex_coords = model.tex_coords;

    let world_position = transformation.matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_position;

    return out;
}

@group(1) @binding(0)
var t_diffuse : texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse : sampler;

@fragment
fn fs_main(in : VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

//////////////////////////////////////////////////////////////////////////////
// Uniforms
//////////////////////////////////////////////////////////////////////////////

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct TransformationUniform {
    matrix: mat4x4<f32>,
    inv_matrix: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> transformation: TransformationUniform;

struct HypershapeDescription {
    shapeType: u32,
    radius: f32,
    size: f32,
};
@group(2) @binding(0)
var<uniform> shapeDesc: HypershapeDescription;

struct WSlice {
    pos: f32,
};
@group(3) @binding(0)
var<uniform> wSlice: WSlice;

//////////////////////////////////////////////////////////////////////////////
// Constants
//////////////////////////////////////////////////////////////////////////////

const MAX_STEPS: i32 = 512;
const EPSILON: f32 = 0.001;
const MAX_DISTANCE: f32 = 100.0;

//////////////////////////////////////////////////////////////////////////////
// Vertex Stage
//////////////////////////////////////////////////////////////////////////////

struct VSOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VSOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0,  3.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0)
    );

    let pos = positions[vertexIndex];

    var out: VSOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos;
    return out;
}

//////////////////////////////////////////////////////////////////////////////
// Distance Functions for 4D Objects
//////////////////////////////////////////////////////////////////////////////

fn sdf_pentachoron(p: vec4<f32>, size: f32) -> f32 {
    let n0 = normalize(vec4<f32>( 1.0,  1.0,  1.0,  1.0));
    let n1 = normalize(vec4<f32>( 1.0, -1.0, -1.0, -1.0));
    let n2 = normalize(vec4<f32>(-1.0,  1.0, -1.0, -1.0));
    let n3 = normalize(vec4<f32>(-1.0, -1.0,  1.0, -1.0));
    let n4 = normalize(vec4<f32>(-1.0, -1.0, -1.0,  1.0));
    let d = size / 4.0;

    let dist0 = dot(n0, p) - d;
    let dist1 = dot(n1, p) - d;
    let dist2 = dot(n2, p) - d;
    let dist3 = dot(n3, p) - d;
    let dist4 = dot(n4, p) - d;

    return max(dist0, max(dist1, max(dist2, max(dist3, dist4))));
}

fn sdf_octochoron(p: vec4<f32>, size: f32) -> f32 {
    var rotatedP = p;
    let angleXY = 3.1415 / 4.0; // 45 degrees
    let angleZW = 3.1415 / 4.0; // 45 degrees

    rotatedP = rotate4D(rotatedP, angleXY, 0u, 1u);
    rotatedP = rotate4D(rotatedP, angleZW, 2u, 3u);

    let halfExtent = vec4<f32>(size);
    let q = max(abs(rotatedP) - halfExtent, vec4<f32>(0.0));
    return length(q);
}

fn sdf_4dsphere(p: vec4<f32>, radius: f32) -> f32 {
    return length(p) - radius;
}

fn sdf_4dTorus(p: vec4<f32>, R: f32, r: f32) -> f32 {
    let xyLen = length(p.xy);
    let zwLen = length(p.zw);
    let q = vec2<f32>(xyLen - R * 0.5, zwLen);
    return length(q) - r * 0.5;
}

fn distance4D(p: vec4<f32>) -> f32 {
    switch (shapeDesc.shapeType) {
        case 0u: { return sdf_pentachoron(p, shapeDesc.size); }
        case 1u: { return sdf_octochoron(p, shapeDesc.size); }
        case 2u: { return sdf_4dsphere(p, shapeDesc.radius); }
        case 3u: { return sdf_4dTorus(p, shapeDesc.radius, shapeDesc.size); }
        default: { return sdf_pentachoron(p, shapeDesc.size); }
    }
}

//////////////////////////////////////////////////////////////////////////////
// Helpers
//////////////////////////////////////////////////////////////////////////////

fn rotate4D(p: vec4<f32>, angle: f32, axis1: u32, axis2: u32) -> vec4<f32> {
    var rotated = p;
    let cosTheta = cos(angle);
    let sinTheta = sin(angle);

    let a1 = axis1;
    let a2 = axis2;
    rotated[a1] = cosTheta * p[a1] - sinTheta * p[a2];
    rotated[a2] = sinTheta * p[a1] + cosTheta * p[a2];

    return rotated;
}

fn worldToLocalPos3D(worldPos: vec3<f32>) -> vec3<f32> {
    return (transformation.inv_matrix * vec4<f32>(worldPos, 1.0)).xyz;
}

fn worldToLocalDir3D(worldDir: vec3<f32>) -> vec3<f32> {
    return normalize((transformation.inv_matrix * vec4<f32>(worldDir, 0.0)).xyz);
}

fn estimateNormal(p: vec4<f32>) -> vec4<f32> {
    let e = 0.001;
    let d0 = distance4D(vec4<f32>(p.x + e, p.y,     p.z,     p.w))
           - distance4D(vec4<f32>(p.x - e, p.y,     p.z,     p.w));
    let d1 = distance4D(vec4<f32>(p.x,     p.y + e, p.z,     p.w))
           - distance4D(vec4<f32>(p.x,     p.y - e, p.z,     p.w));
    let d2 = distance4D(vec4<f32>(p.x,     p.y,     p.z + e, p.w))
           - distance4D(vec4<f32>(p.x,     p.y,     p.z - e, p.w));
    let d3 = distance4D(vec4<f32>(p.x,     p.y,     p.z,     p.w + e))
           - distance4D(vec4<f32>(p.x,     p.y,     p.z,     p.w - e));
    return normalize(vec4<f32>(d0, d1, d2, d3));
}

//////////////////////////////////////////////////////////////////////////////
// Fragment Stage
//////////////////////////////////////////////////////////////////////////////

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
};

struct RayMarchResult {
    hit: bool,
    t: f32,
    hitPos4: vec4<f32>,
};

fn reconstructRay(inUV: vec2<f32>) -> Ray {
    let clampedCoord = clamp(inUV, vec2<f32>(-1.0), vec2<f32>(1.0));

    let nearPos4 = camera.inv_view_proj * vec4<f32>(clampedCoord, 0.0, 1.0);
    let farPos4  = camera.inv_view_proj * vec4<f32>(clampedCoord, 1.0, 1.0);

    let nearPos = nearPos4.xyz / nearPos4.w;
    let farPos  = farPos4.xyz / farPos4.w;
    let ro_world = nearPos;
    let rd_world = normalize(farPos - ro_world);
    
    return Ray(ro_world, rd_world);
}

fn transformRayToLocal(ro_world: vec3<f32>, rd_world: vec3<f32>) -> Ray {
    let ro_local = worldToLocalPos3D(ro_world);
    let rd_local = worldToLocalDir3D(rd_world);
    return Ray(ro_local, rd_local);
}

fn performRayMarch(ro_local: vec3<f32>, rd_local: vec3<f32>) -> RayMarchResult {
    var t = 0.0;
    var hit = false;
    var hitPos4 = vec4<f32>(0.0);
    
    for (var i = 0; i < MAX_STEPS; i = i + 1) {
        let p_local_3d = ro_local + rd_local * t;
        let pos4 = vec4<f32>(p_local_3d, wSlice.pos);
        
        let dist = distance4D(pos4);
        if (dist < EPSILON) {
            hit = true;
            hitPos4 = pos4;
            break;
        }
        t += dist;
        
        if (t > MAX_DISTANCE) {
            break;
        }
    }
    
    return RayMarchResult(hit, t, hitPos4);
}

fn getWorldHitPoint(ro_local: vec3<f32>, rd_local: vec3<f32>, t: f32) -> vec3<f32> {
    let hitPoint_local = ro_local + rd_local * t;
    let hitPoint_world3 = (transformation.matrix * vec4<f32>(hitPoint_local, 1.0)).xyz;
    return hitPoint_world3;
}

fn computeDepth(hitPoint_world3: vec3<f32>) -> f32 {
    let clipPos = camera.view_proj * vec4<f32>(hitPoint_world3, 1.0);
    if (clipPos.w <= 0.0) {
        discard;
    }
    let ndc = clipPos.xyz / clipPos.w;
    return ndc.z;
}

fn computeNormal(hitPos4: vec4<f32>) -> vec3<f32> {
    let N4_obj = estimateNormal(hitPos4);
    let N3_world = normalize(
        (transpose(transformation.inv_matrix) * vec4<f32>(N4_obj.xyz, 0.0)).xyz
    );
    return N3_world;
}

fn detectEdge(hitPoint_local: vec3<f32>, N3_world: vec3<f32>) -> bool {
    let e = 0.02;
    var offsets = array<vec3<f32>, 6>(
        vec3<f32>( e, 0.0, 0.0),
        vec3<f32>(-e, 0.0, 0.0),
        vec3<f32>(0.0,  e, 0.0),
        vec3<f32>(0.0, -e, 0.0),
        vec3<f32>(0.0, 0.0,  e),
        vec3<f32>(0.0, 0.0, -e)
    );
    
    var isEdge = false;
    for (var i = 0u; i < 6u; i = i + 1u) {
        let offset = offsets[i];
        let pos_offset = vec4<f32>(hitPoint_local + offset, wSlice.pos);
        let N_offset_obj = estimateNormal(pos_offset);
        
        let N_offset_world = normalize(
            (transpose(transformation.inv_matrix) * vec4<f32>(N_offset_obj.xyz, 0.0)).xyz
        );
        
        if (dot(N3_world, N_offset_world) < 0.4) {
            isEdge = true;
            break;
        }
    }
    
    return isEdge;
}

fn computeShading(N3_world: vec3<f32>, ro_world: vec3<f32>, hitPoint_world3: vec3<f32>) -> vec3<f32> {
    let lightDir = normalize(vec3<f32>(-0.3, -1.0, -0.5));
    let viewDir = normalize(ro_world - hitPoint_world3);
    
    let lambert = max(0.0, dot(N3_world, -lightDir));
    
    let halfway = normalize(-lightDir + viewDir);
    let specAngle = max(0.0, dot(N3_world, halfway));
    let shininess = 128.0;
    let specular = pow(specAngle, shininess);
    
    let rim = 1.0 - max(0.0, dot(N3_world, viewDir));
    let rimPower = 10.0;
    let rimTerm = pow(rim, rimPower);
    
    let baseColor = vec3<f32>(0.192, 0.212, 0.220);
    let ambient = 0.25;                       // ambient factor
    let diffuse = lambert * 0.75;            // diffuse factor
    let specColor = vec3<f32>(1.0, 1.0, 1.0) * specular * 0.3;
    let rimColor = vec3<f32>(1.0, 1.0, 1.0) * rimTerm * 0.15;
    let shading = baseColor * (ambient + diffuse) + specColor + rimColor;
    
    return shading;
}

fn assembleFinalColor(shading: vec3<f32>, isEdge: bool) -> vec4<f32> {
    var finalColor = shading;
    
    if (isEdge) {
        finalColor -= vec3<f32>(0.02, 0.02, 0.02);
    }
    
    finalColor = clamp(finalColor, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(finalColor, 1.0);
}

@fragment
fn fs_main(@location(0) inUV: vec2<f32>) -> FragmentOutput {
    var out: FragmentOutput;
    
    let ray = reconstructRay(inUV);
    let ro_world = ray.origin;
    let rd_world = ray.direction;
    
    let transformedRay = transformRayToLocal(ro_world, rd_world);
    let ro_local = transformedRay.origin;
    let rd_local = transformedRay.direction;
    let marchResult = performRayMarch(ro_local, rd_local);
    if (!marchResult.hit) {
        discard;
    }
    
    let hitPoint_world3 = getWorldHitPoint(ro_local, rd_local, marchResult.t);
    out.depth = computeDepth(hitPoint_world3);

    let N3_world = computeNormal(marchResult.hitPos4);
    let hitPoint_local = ro_local + rd_local * marchResult.t;
    let isEdge = detectEdge(hitPoint_local, N3_world);
    let shading = computeShading(N3_world, ro_world, hitPoint_world3);
    out.color = assembleFinalColor(shading, isEdge);
    
    return out;
}


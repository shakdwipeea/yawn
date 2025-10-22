struct UniformData {
    mouse_move: vec2<f32>,
    mouse_click: vec2<f32>,
    resolution: vec2<f32>,
    time: f32,
    _padding0: f32,
    camera_position: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uni: UniformData;
@group(1) @binding(0) var<uniform> view_proj: mat4x4<f32>;

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_col0: vec4<f32>,
    @location(4) model_col1: vec4<f32>,
    @location(5) model_col2: vec4<f32>,
    @location(6) model_col3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>
}


@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4<f32>(
        in.model_col0,
        in.model_col1,
        in.model_col2,
        in.model_col3,
    );
    let world_position = model * vec4<f32>(in.pos, 1.0);
    out.clip_position = view_proj * world_position;
    out.world_pos = world_position.xyz;
    out.normal = normalize(in.normal);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_direction = normalize(vec3<f32>(0.35, 1.0, 0.45));
    let light_color = vec3<f32>(1.0, 0.95, 0.85);
    let base_color = vec3<f32>(0.2, 0.2, 0.2);

    let normal = normalize(in.normal);
    let view_dir = normalize(uni.camera_position.xyz - in.world_pos);

    let diffuse_strength = max(dot(normal, light_direction), 0.0);
    let ambient = 0.15;

    var specular = 0.0;
    if diffuse_strength > 0.0 {
        let halfway_dir = normalize(light_direction + view_dir);
        specular = pow(max(dot(normal, halfway_dir), 0.0), 32.0);
    }

    let lighting = min(base_color * (ambient + diffuse_strength) + light_color * specular, vec3<f32>(1.0));
    let x = select(0.0, 0.3, distance(in.clip_position.xy, uni.mouse_move) < 25.0);
    let y = select(0.0, 0.3, distance(in.clip_position.xy, uni.mouse_click) < 25.0);
    return vec4<f32>(lighting + x - y, 1.0);
}

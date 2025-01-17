struct Particle {
    model: mat4x4<f32>,
    color: vec4<f32>,
    vel_mass: vec4<f32>, 
    scale: f32,
    lifetime: f32, // lifetime == -1. is decayed
};

struct Emitter {
    delta_sec: f32,
    elapsed_sec: f32,
    spawn_from: f32,
    spawn_until: f32,
    box_x: f32,
    box_y: f32,
    box_z: f32,
    box_width: f32,
    box_height: f32,
    box_depth: f32,
    box_yaw: f32,
    box_pitch: f32,
    box_roll: f32,
    diffusion_width: f32,
    diffusion_depth: f32,
    padding: f32,
    particle_model: mat4x4<f32>,
    particle_color_r: f32,
    particle_color_g: f32,
    particle_color_b: f32,
    particle_color_a: f32,
    particle_speed_min: f32,
    particle_speed_max: f32,
    particle_size_min: f32,
    particle_size_max: f32,
    particle_friction_coefficient: f32,
    material_mass: f32,
    particle_lifetime: f32,
    padding_2: f32,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    position: vec3<f32>,
    bloom_treshold: vec3<f32>,
    tonemap: u32,
};

struct GaussianBlur {
    br_treshold: f32,
    radius: i32,
    sigma: f32,
    intensity: f32,
}

struct FxIO {
    in_idx: u32,
    out_idx: u32,
    in_downscale: u32,
    out_downscale: u32,
    in_size_x: u32,
    in_size_y: u32,
    out_size_x: u32,
    out_size_y: u32,
}

const PI: f32 = 3.141592653589;

fn is_decayed(em: Emitter, par: Particle) -> bool {
    return em.particle_lifetime < par.lifetime;
}

fn random(input: f32, elapsed_sec: f32) -> f32 {
    let value = vec2<f32>(input, elapsed_sec);
    return fract(sin(dot(value, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn gen_abs_range(unique: f32, value: f32, elapsed_sec: f32) -> f32 {
    return abs(random(unique, elapsed_sec)) * value;
}

fn gen_dyn_range(unique: f32, value: f32, elapsed_sec: f32) -> f32 {
    return sin(random(unique, elapsed_sec) * 60.) * value;
}

fn yaw_matrix(yaw: f32) -> mat3x3<f32> {
    let s = sin(yaw);
    let c = cos(yaw);

    return mat3x3<f32>(
        vec3<f32>(c, 0., -s),
        vec3<f32>(0., 1., 0.),
        vec3<f32>(s, 0., c),
    );
}

fn pitch_matrix(pitch: f32) -> mat3x3<f32> {
    let s = sin(pitch);
    let c = cos(pitch);

    return mat3x3<f32>(
        vec3<f32>(c, s, 0.),
        vec3<f32>(-s, c, 0.),
        vec3<f32>(0., 0., 1.),
    );
}

fn roll_matrix(roll: f32) -> mat3x3<f32> {
    let s = sin(roll);
    let c = cos(roll);

    return mat3x3<f32>(
        vec3<f32>(1., 0., 0.),
        vec3<f32>(0., c, s),
        vec3<f32>(0., -s, c),
    );
}

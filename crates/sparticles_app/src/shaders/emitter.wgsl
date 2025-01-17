@group(0) @binding(0) var<storage, read> particles_src : array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_dst : array<Particle>;
@group(0) @binding(2) var<uniform> em: Emitter; 

fn create_velocity(input_random: f32, speed_random: f32) -> vec3<f32> {
    let diff_width = gen_dyn_range(input_random * 0.12, em.diffusion_width, em.elapsed_sec) / 2.;
    let diff_depth = gen_dyn_range(input_random * 0.45, em.diffusion_depth, em.elapsed_sec) / 2.;

    return vec3<f32>(0., speed_random, 0.) * yaw_matrix(em.box_yaw) * pitch_matrix(em.box_pitch + diff_width) * roll_matrix(em.box_roll + diff_depth);
}

fn create_particle_position(input_random: f32) -> vec3<f32> {
    let half_width = em.box_width / 2.0;
    let half_height = em.box_height / 2.0;
    let half_depth = em.box_depth / 2.0;

    let random_width = random(input_random * 1.6, em.elapsed_sec);
    let random_height = random(input_random * 0.42, em.elapsed_sec);
    let random_depth = random(input_random / 0.11, em.elapsed_sec);

    let unrotated_x = random_width * em.box_width - half_width;
    let unrotated_y = random_height * em.box_height - half_height;
    let unrotated_z = random_depth * em.box_depth - half_depth;

    let local_pos = vec3<f32>(unrotated_x, unrotated_y, unrotated_z);

    let local_rot = local_pos * yaw_matrix(em.box_yaw) * pitch_matrix(em.box_pitch) * roll_matrix(em.box_roll);

    return vec3<f32>(em.box_x, em.box_y, em.box_z) + local_rot;
}

fn spawn_particle(index: u32) {
    var particle = particles_src[index];
    let input_random = f32(index);

    let particle_color = vec4<f32>(
        em.particle_color_r,
        em.particle_color_g,
        em.particle_color_b,
        em.particle_color_a,
    );

    let size_delta = em.particle_size_max - em.particle_size_min;
    let size_random = gen_abs_range(input_random + 100., size_delta, em.elapsed_sec);
    let size = em.particle_size_min + size_random;

    let speed_delta = em.particle_speed_max - em.particle_speed_min;
    let speed_random = gen_abs_range(input_random + 40., speed_delta, em.elapsed_sec);
    let particle_speed = em.particle_speed_min + speed_random;
    let position = create_particle_position(input_random);
    let velocity = create_velocity(input_random, particle_speed);

    particle.scale = size;
    particle.color = particle_color;
    particle.vel_mass = vec4<f32>(velocity, em.material_mass * size);
    particle.lifetime = 0.;
    particle.model = em.particle_model;
    particle.model.w = vec4(position, 1.0);

    particles_dst[index] = particle;
}

@compute
@workgroup_size(128)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let particle_len = arrayLength(&particles_src);
    let index = global_invocation_id.x;

    if particle_len <= index {
        return;
    }

    if u32(em.spawn_from) <= index && index < u32(em.spawn_until) {
        spawn_particle(index);
        return;
    }

    var particle = particles_src[index];

    if is_decayed(em, particle) {
        particles_dst[index].lifetime = particle.lifetime;
        return;
    }

    particle.lifetime += em.delta_sec;

    let new_vel = particle.vel_mass.xyz * em.particle_friction_coefficient;
    particle.vel_mass = vec4<f32>(new_vel, particle.vel_mass.w);

    let pos = particle.model.w;
    let new_pos = pos.xyz + new_vel * em.delta_sec;

    particle.model.w = vec4(new_pos, 1.);

    particles_dst[index] = particle;
}


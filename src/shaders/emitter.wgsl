@group(0) @binding(0) var<storage, read> particles_src : array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_dst : array<Particle>;
@group(0) @binding(2) var<uniform> em: Emitter; 

fn is_decayed(par: Particle) -> bool {
    return em.particle_lifetime < par.lifetime;
}

fn pitch_matrix() -> mat3x3<f32> {
    let s = sin(em.box_pitch);
    let c = cos(em.box_pitch);

    return mat3x3<f32>(
        vec3<f32>(c, s, 0.),
        vec3<f32>(-s, c, 0.),
        vec3<f32>(0., 0., 1.),
    );
}

fn roll_matrix() -> mat3x3<f32> {
    let s = sin(em.box_roll);
    let c = cos(em.box_roll);

    return mat3x3<f32>(
        vec3<f32>(1., 0., 0.),
        vec3<f32>(0., c, s),
        vec3<f32>(0., -s, c),
    );
}

fn yaw_matrix() -> mat3x3<f32> {
    let s = sin(em.box_yaw);
    let c = cos(em.box_yaw);

    return mat3x3<f32>(
        vec3<f32>(c, 0., -s),
        vec3<f32>(0., 1., 0.),
        vec3<f32>(s, 0., c),
    );
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
    let local = local_pos * roll_matrix() * yaw_matrix() * pitch_matrix();
    
    return vec3<f32>(em.box_x, em.box_y, em.box_z) + local;
}

fn spawn_particle(index: u32) {
    var particle = particles_src[index];

    let input_random = f32(index);
    let particle_position = create_particle_position(input_random);

    let particle_color = vec4<f32>(
        em.particle_color_r,
        em.particle_color_g,
        em.particle_color_b,
        em.particle_color_a,
    );

    let velocity = vec3<f32>(
        em.particle_velocity_x,
        em.particle_velocity_y,
        em.particle_velocity_z,
    );

    particle.position = particle_position;
    particle.velocity = velocity;
    particle.color = particle_color;
    particle.size = em.particle_size;
    particle.lifetime = 0.;
    particle.mass = em.particle_mass;

    particles_dst[index] = particle;
}

@compute
@workgroup_size(128)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let particle_len = arrayLength(&particles_src);
    let index = global_invocation_id.x;

    if (particle_len <= index) {
        return;
    }

    if (u32(em.spawn_from) <= index && index < u32(em.spawn_until)) {
        spawn_particle(index);
        return;
    } 

    var particle = particles_src[index];
    particle.lifetime += em.delta_sec;

    if (is_decayed(particle)) {
        if (particle.lifetime != -1.) {
            particle.lifetime = -1.;
            particles_dst[index] = particle;
        }
        return;
    }

    particle.velocity *= em.particle_friction_coefficient;
    particle.position += particle.velocity * em.delta_sec;

    particles_dst[index] = particle;
}

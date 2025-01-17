use crate::model::clock::Clock;
use crate::model::{EmitterState, GfxState, LifeCycle};
use crate::shaders::ShaderOptions;
use crate::traits::*;
use crate::util::persistence::DynamicExport;
use crate::util::ListAction;
use egui_wgpu::wgpu;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::any::Any;
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GravityUniform {
    pub life_cycle: LifeCycle,
    pub gravitational_force: f32,
    pub dead_zone: f32,
    pub mass: f32,
    pub should_animate: bool,
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub current_pos: Vec3,
}

impl Default for GravityUniform {
    fn default() -> Self {
        Self {
            life_cycle: LifeCycle {
                from_sec: 0.,
                until_sec: 6.,
                lifetime_sec: 12.,
            },
            gravitational_force: 0.01,
            dead_zone: 4.,
            mass: 1_000_000.,
            start_pos: [-25., 8., 0.].into(),
            current_pos: [-25., 8., 0.].into(),
            end_pos: [25., 8., 0.].into(),
            should_animate: false,
        }
    }
}

pub struct GravityUniformOptions {
    /// In newton
    pub gravitational_force: f32,
    /// Use to exclude extreme gravitational pulls, e.g. 20.
    pub dead_zone: f32,
    pub mass: f32,
    pub life_cycle: LifeCycle,
    pub start_pos: Vec3,
    pub end_pos: Vec3,
}

impl GravityUniform {
    pub fn new(props: GravityUniformOptions) -> Self {
        Self {
            gravitational_force: props.gravitational_force,
            dead_zone: props.dead_zone,
            mass: props.mass,
            life_cycle: props.life_cycle,
            start_pos: props.start_pos,
            end_pos: props.end_pos,
            current_pos: props.start_pos,
            should_animate: false,
        }
    }

    fn create_buffer_content(&self) -> [f32; 6] {
        [
            self.gravitational_force,
            self.dead_zone,
            self.mass,
            self.current_pos.x,
            self.current_pos.y,
            self.current_pos.z,
        ]
    }
}

#[derive(Clone, Copy)]
pub struct RegisterGravityAnimation;

impl RegisterGravityAnimation {
    /// Will append animation to emitter
    pub fn append(uniform: GravityUniform, emitter: &mut EmitterState, gfx_state: &GfxState) {
        let anim = Box::new(GravityAnimation::new(uniform, emitter, gfx_state));

        emitter.push_particle_animation(anim);
    }
}

impl RegisterParticleAnimation for RegisterGravityAnimation {
    fn create_default(
        &self,
        gfx_state: &GfxState,
        emitter: &EmitterState,
    ) -> Box<dyn ParticleAnimation> {
        Box::new(GravityAnimation::new(
            GravityUniform::default(),
            emitter,
            gfx_state,
        ))
    }

    fn tag(&self) -> &'static str {
        "gravity"
    }

    fn import(
        &self,
        gfx_state: &GfxState,
        emitter: &EmitterState,
        value: serde_json::Value,
    ) -> Box<dyn ParticleAnimation> {
        let uniform = serde_json::from_value(value).unwrap();
        Box::new(GravityAnimation::new(uniform, emitter, gfx_state))
    }
}

pub struct GravityAnimation {
    pub pipeline: wgpu::ComputePipeline,
    pub uniform: GravityUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub selected_action: ListAction,
    pub enabled: bool,
}

impl HandleAction for GravityAnimation {
    fn selected_action(&mut self) -> &mut ListAction {
        &mut self.selected_action
    }

    fn export(&self) -> DynamicExport {
        let animation = serde_json::to_value(self.uniform).unwrap();

        DynamicExport {
            tag: RegisterGravityAnimation.tag().to_owned(),
            data: animation,
        }
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

impl ParticleAnimation for GravityAnimation {
    fn compute<'a>(
        &'a self,
        emitter: &'a EmitterState,
        clock: &Clock,
        compute_pass: &mut wgpu::ComputePass<'a>,
    ) {
        if !self.uniform.should_animate {
            return;
        }

        let nr = clock.get_bindgroup_nr();

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &emitter.bgs[nr], &[]);
        compute_pass.set_bind_group(1, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(emitter.dispatch_x_count, 1, 1);
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn update(&mut self, clock: &Clock, gfx_state: &GfxState) {
        let queue = &gfx_state.queue;
        let uniform = &mut self.uniform;
        let life_cycle = &mut uniform.life_cycle;
        let current_sec = life_cycle.get_current_sec(clock);

        uniform.should_animate = life_cycle.shoud_animate(current_sec);

        if uniform.should_animate {
            let fraction = life_cycle.get_fraction(current_sec);
            uniform.current_pos = uniform.start_pos.lerp(uniform.end_pos, fraction);
            let buffer_content = uniform.create_buffer_content();

            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&buffer_content));
        }
    }

    fn recreate(&self, gfx_state: &GfxState, emitter: &EmitterState) -> Box<dyn ParticleAnimation> {
        Box::new(Self::new(self.uniform, emitter, gfx_state))
    }
}

impl GravityAnimation {
    fn new(uniform: GravityUniform, emitter: &EmitterState, gfx_state: &GfxState) -> Self {
        let device = &gfx_state.device;

        let shader = gfx_state.create_shader_builtin(ShaderOptions {
            if_directives: &[],
            files: &["gravity_anim.wgsl"],
            label: "Gravity animation",
        });

        let buffer_content = uniform.create_buffer_content();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gravitational buffer"),
            contents: bytemuck::cast_slice(&buffer_content),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let animation_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Uniform data
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(buffer_content.len() as u64 * 4),
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &animation_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Gravity animation bind group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gravity animation layout"),
            bind_group_layouts: &[&emitter.bg_layout, &animation_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Gravity animation pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        Self {
            pipeline,
            uniform,
            buffer,
            bind_group,
            selected_action: ListAction::None,
            enabled: true,
        }
    }
}

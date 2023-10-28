use super::{
    post_process::{CreateFxOptions, FxIOUniform, PingPongState},
    FxState,
};
use crate::{traits::CustomShader, util::UniformContext};
use egui_wgpu::wgpu;
use encase::ShaderType;
use serde::{Deserialize, Serialize};

pub struct Downscale {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    io_uniform: FxIOUniform,
}

#[derive(ShaderType, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct DownscaleUniform {
    pub downscale: f32,
}

pub struct DownscaleSettings {
    pub ds_uniform: DownscaleUniform,
    pub io_uniform: FxIOUniform,
}

impl Downscale {
    pub fn compute<'a>(
        &'a self,
        ping_pong: &mut PingPongState,
        fx_state: &'a FxState,
        c_pass: &mut wgpu::ComputePass<'a>,
    ) {
        let (count_x, count_y) = fx_state.count_in(&self.io_uniform);

        c_pass.set_pipeline(&self.pipeline);
        c_pass.set_bind_group(0, fx_state.bind_group(ping_pong), &[]);
        c_pass.set_bind_group(1, &self.bind_group, &[]);
        c_pass.dispatch_workgroups(count_x, count_y, 1);

        ping_pong.swap();
    }

    pub fn new2(options: &CreateFxOptions, settings: DownscaleSettings) -> Self {
        let CreateFxOptions {
            gfx_state,
            fx_state,
        } = options;

        let device = &gfx_state.device;
        let shader = device.create_shader("fx/downscale.wgsl", "Downscale");

        let io_ctx = UniformContext::from_uniform(&settings.io_uniform, device, "IO");
        let ds_ctx = UniformContext::from_uniform(&settings.ds_uniform, device, "Downscale");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Downscale layout"),
            bind_group_layouts: &[
                &fx_state.bind_group_layout,
                &io_ctx.bg_layout,
                &ds_ctx.bg_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Downscale pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "downscale",
        });

        Self {
            pipeline,
            bind_group: io_ctx.bg,
            io_uniform: settings.io_uniform,
        }
    }

    pub fn new(options: &CreateFxOptions, io_uniform: FxIOUniform) -> Self {
        let CreateFxOptions {
            gfx_state,
            fx_state,
        } = options;

        let device = &gfx_state.device;
        let shader = device.create_shader("fx/downscale.wgsl", "Downscale");

        let io_ctx = UniformContext::from_uniform(&io_uniform, device, "Downscale");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Downscale layout"),
            bind_group_layouts: &[&fx_state.bind_group_layout, &io_ctx.bg_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Downscale pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "downscale",
        });

        Self {
            pipeline,
            bind_group: io_ctx.bg,
            io_uniform,
        }
    }
}

use super::{
    post_process::{CreateFxOptions, FxIOUniform, PingPongState},
    FxState,
};
use crate::{traits::CustomShader, util::UniformContext};
use egui_wgpu::wgpu;

pub struct Blend {
    additive_pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    io_uniform: FxIOUniform,
}

impl Blend {
    pub fn compute_additive<'a>(
        &'a self,
        ping_pong: &mut PingPongState,
        fx_state: &'a FxState,
        c_pass: &mut wgpu::ComputePass<'a>,
    ) {
        let (count_x, count_y) = fx_state.count_out(&self.io_uniform);

        c_pass.set_pipeline(&self.additive_pipeline);
        c_pass.set_bind_group(0, fx_state.bind_group(ping_pong), &[]);
        c_pass.set_bind_group(1, &self.bind_group, &[]);
        c_pass.dispatch_workgroups(count_x, count_y, 1);

        ping_pong.swap(&self.io_uniform);
    }

    pub fn new(options: &CreateFxOptions, io_uniform: FxIOUniform) -> Self {
        let CreateFxOptions {
            gfx_state,
            fx_state,
        } = options;

        let device = &gfx_state.device;
        let blend_shader = device.create_shader("fx/blend.wgsl", "Blend");

        let blend_ctx = UniformContext::from_uniform(&io_uniform, device, "Blend");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blend layout"),
            bind_group_layouts: &[&fx_state.bind_group_layout, &blend_ctx.bg_layout],
            push_constant_ranges: &[],
        });

        // TODO multiple entry points for different types of blend
        let additive_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Blend pipeline"),
            layout: Some(&pipeline_layout),
            module: &blend_shader,
            entry_point: "additive",
        });

        Self {
            additive_pipeline,
            bind_group: blend_ctx.bg,
            io_uniform,
        }
    }
}

use std::sync::Arc;

use super::FxIOSwapCtx;
use super::FxIOUniform;
use super::FxOptions;
use super::FxState;
use crate::model::gfx_state::Profiler;
use crate::model::GfxState;
use crate::shaders::ShaderOptions;
use async_std::sync::RwLock;
use async_std::task;
use egui_wgpu::wgpu;

pub struct BlurPass {
    pub blur_pipeline_x: wgpu::ComputePipeline,
    pub blur_pipeline_y: wgpu::ComputePipeline,
    pub split_pipeline: wgpu::ComputePipeline,

    io_ctx: FxIOSwapCtx,
}

#[derive(Debug)]
pub struct BlurPassSettings<'a> {
    pub blur_layout: &'a wgpu::BindGroupLayout,
    pub io_idx: (u32, u32),
    pub downscale: f32,
}

impl BlurPass {
    /// Computes horizontal vertical gaussian blur
    pub fn compute_gaussian<'a>(
        &'a self,
        fx_state: &'a FxState,
        gfx: &Arc<RwLock<GfxState>>,
        blur_bg: &'a wgpu::BindGroup,
        c_pass: &mut wgpu::ComputePass<'a>,
    ) {
        task::block_on(Profiler::begin_scope(gfx, "Gaussian", c_pass));

        let (count_x, count_y) = fx_state.count_out(&self.io_ctx.uniforms[0]);

        c_pass.set_pipeline(&self.blur_pipeline_x);
        c_pass.set_bind_group(0, &fx_state.bg, &[]);
        c_pass.set_bind_group(1, &self.io_ctx.bgs[0], &[]);
        c_pass.set_bind_group(2, blur_bg, &[]);
        c_pass.dispatch_workgroups(count_x, count_y, 1);

        c_pass.set_pipeline(&self.blur_pipeline_y);
        c_pass.set_bind_group(0, &fx_state.bg, &[]);
        c_pass.set_bind_group(1, &self.io_ctx.bgs[1], &[]);
        c_pass.set_bind_group(2, blur_bg, &[]);
        c_pass.dispatch_workgroups(count_x, count_y, 1);

        task::block_on(Profiler::end_scope(gfx, c_pass));
    }

    pub fn split<'a>(
        &'a self,
        fx_state: &'a FxState,
        gfx: &Arc<RwLock<GfxState>>,
        blur_bg: &'a wgpu::BindGroup,
        c_pass: &mut wgpu::ComputePass<'a>,
    ) {
        task::block_on(Profiler::begin_scope(gfx, "Split", c_pass));

        c_pass.set_pipeline(&self.split_pipeline);
        c_pass.set_bind_group(0, &fx_state.bg, &[]);
        c_pass.set_bind_group(1, &self.io_ctx.bgs[0], &[]);
        c_pass.set_bind_group(2, blur_bg, &[]);
        c_pass.dispatch_workgroups(fx_state.count_x, fx_state.count_y, 1);

        task::block_on(Profiler::end_scope(gfx, c_pass));
    }

    pub fn resize(&mut self, options: &FxOptions) {
        self.io_ctx.resize(options);
    }

    pub fn new(options: &FxOptions, settings: BlurPassSettings) -> Self {
        let FxOptions {
            gfx: gfx_state,
            fx_state,
        } = options;

        let device = &gfx_state.device;

        let BlurPassSettings {
            blur_layout,
            io_idx: (in_idx, out_idx),
            downscale,
        } = settings;

        let blur_shader = gfx_state.create_shader_builtin(ShaderOptions {
            label: "Gaussian blur",
            files: &["fx/gaussian_blur.wgsl"],
            if_directives: &[],
        });

        let io_ping = FxIOUniform::asymetric_scaled(options.fx_state, in_idx, out_idx, downscale);
        let io_pong = FxIOUniform::asymetric_scaled(options.fx_state, out_idx, in_idx, downscale);
        let io_ctx = FxIOSwapCtx::new([io_ping, io_pong], device, "IO Swap blur");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Split layout"),
            bind_group_layouts: &[&fx_state.bg_layout, &io_ctx.bg_layout, &blur_layout],
            push_constant_ranges: &[],
        });

        let new_pipeline = |entry_point: &str| -> wgpu::ComputePipeline {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Gaussian blur pipeline"),
                layout: Some(&pipeline_layout),
                module: &blur_shader,
                entry_point,
            })
        };

        let blur_pipeline_x = new_pipeline("apply_blur_x");
        let blur_pipeline_y = new_pipeline("apply_blur_y");
        let split_pipeline = new_pipeline("split_bloom");

        Self {
            blur_pipeline_x,
            blur_pipeline_y,
            split_pipeline,
            io_ctx,
        }
    }
}

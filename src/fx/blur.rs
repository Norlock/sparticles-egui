use super::post_process::FxState;
use super::post_process::FxStateOptions;
use crate::traits::*;
use crate::GfxState;
use egui_wgpu::wgpu::{self, util::DeviceExt};
use egui_winit::egui::Slider;
use egui_winit::egui::Ui;
use encase::{ShaderType, UniformBuffer};
use std::num::NonZeroU64;

pub struct Blur {
    blur_pipelines: Vec<wgpu::ComputePipeline>,
    split_pipeline: wgpu::ComputePipeline,

    blur_bind_group: wgpu::BindGroup,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub blur: BlurUniform,
    pub blur_buffer: wgpu::Buffer,

    fx_state: FxState,
    passes: usize,
}

#[derive(Debug, ShaderType)]
pub struct BlurUniform {
    /// 0.10 - 0.15 is reasonable
    pub brightness_threshold: f32,
    /// 2.2 - 2.6 is reasonable
    pub gamma: f32,
    /// Kernel size (8 default) too high or too low slows down performance
    /// Lower is more precise
    pub kernel_size: u32,

    // How far should the blur reach (in relation with kernel size)
    pub radius: u32,
    //pub depth_add: f32,
    //pub depth_mul: f32,
}

impl BlurUniform {
    pub fn new() -> Self {
        Self {
            brightness_threshold: 0.2,
            gamma: 2.2,
            kernel_size: 16,
            radius: 16,
        }
    }

    pub fn create_buffer_content(&self) -> Vec<u8> {
        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&self).unwrap();
        buffer.into_inner()
    }
}

impl PostFx for Blur {
    fn compute<'a>(
        &'a self,
        fx_inputs: Vec<&'a wgpu::BindGroup>,
        c_pass: &mut wgpu::ComputePass<'a>,
    ) {
        let output = &self.fx_state;
        let input = fx_inputs[0];

        // Splits parts to fx tex
        c_pass.set_pipeline(&self.split_pipeline);
        c_pass.set_bind_group(0, input, &[]);
        c_pass.set_bind_group(1, &output.bind_group(1), &[]);
        c_pass.set_bind_group(2, &self.blur_bind_group, &[]);
        c_pass.dispatch_workgroups(output.count_x, output.count_y, 1);

        // Smoothen downscaled texture
        for i in 0..self.passes {
            c_pass.set_pipeline(&self.blur_pipelines[i % 2]);
            c_pass.set_bind_group(0, input, &[]);
            c_pass.set_bind_group(1, &output.bind_group(i), &[]);
            c_pass.set_bind_group(2, &self.blur_bind_group, &[]);
            c_pass.dispatch_workgroups(output.count_x, output.count_y, 1);
        }
    }

    fn resize(&mut self, gfx_state: &GfxState) {
        let dims = Self::tex_dimensions(&gfx_state.surface_config, self.blur.kernel_size);
        self.fx_state.resize(dims, gfx_state);
    }

    fn fx_state(&self) -> &FxState {
        &self.fx_state
    }

    fn output(&self) -> &wgpu::BindGroup {
        self.fx_state.bind_group(self.passes % 2)
    }

    fn create_ui(&mut self, ui: &mut Ui, gfx_state: &GfxState) {
        let queue = &gfx_state.queue;

        ui.label("Gaussian blur");
        ui.add(
            Slider::new(&mut self.blur.brightness_threshold, 0.0..=1.0)
                .text("Brightness threshold"),
        );
        ui.add(Slider::new(&mut self.blur.kernel_size, 4..=32).text("Kernel size"));
        ui.add(Slider::new(&mut self.blur.radius, 4..=16).text("Blur radius"));
        ui.add(
            Slider::new(&mut self.passes, 2..=100)
                .step_by(2.)
                .text("Amount of passes"),
        );

        queue.write_buffer(&self.blur_buffer, 0, &self.blur.create_buffer_content());
    }
}

impl Blur {
    fn tex_dimensions(config: &wgpu::SurfaceConfiguration, kernel_size: u32) -> [u32; 2] {
        let fx_dim = config.fx_dimensions();
        let tex_width = (fx_dim[0] as f32 / kernel_size as f32).ceil() as u32;
        let tex_height = (fx_dim[1] as f32 / kernel_size as f32).ceil() as u32;

        [tex_width, tex_height]
    }

    pub fn new(gfx_state: &GfxState, depth_view: &wgpu::TextureView, shader_entry: &str) -> Self {
        let device = &gfx_state.device;
        let config = &gfx_state.surface_config;

        let blur = BlurUniform::new();
        let buffer_content = blur.create_buffer_content();
        let min_binding_size = NonZeroU64::new(buffer_content.len() as u64);

        let blur_shader = device.create_shader("fx/gaussian_blur.wgsl", "Gaussian blur");

        let blur_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Blur uniform"),
            contents: &buffer_content,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let passes = 8;

        let fx_state = FxState::new(FxStateOptions {
            label: "Blur".to_string(),
            tex_dimensions: Self::tex_dimensions(config, blur.kernel_size),
            gfx_state,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blur uniform layout"),
            entries: &[
                // Globals
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size,
                    },
                    count: None,
                },
                // Depth
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let blur_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blur uniform bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: blur_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Split layout"),
            bind_group_layouts: &[
                &fx_state.bind_group_layout, // input
                &fx_state.bind_group_layout, // output
                &bind_group_layout,          // globals + depth
            ],
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

        let blur_pipelines = vec![new_pipeline("blur_x"), new_pipeline("blur_y")];
        let split_pipeline = new_pipeline(shader_entry);

        Self {
            blur_pipelines,
            bind_group_layout,
            blur_bind_group,
            blur_buffer,
            blur,
            fx_state,
            split_pipeline,
            passes,
        }
    }
}

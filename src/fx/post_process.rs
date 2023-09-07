use std::num::NonZeroU64;

use crate::traits::*;
use egui_wgpu::wgpu::{self, util::DeviceExt};
use encase::{ShaderType, UniformBuffer};

use crate::model::GfxState;

use super::{blend::BlendCompute, Blend, BlendType, Bloom};

pub struct PostProcessState {
    pub frame_state: FrameState,
    fx_state: FxState,
    post_fx: Vec<Box<dyn PostFxChain>>,
    frame_group_layout: wgpu::BindGroupLayout,
    initialize_pipeline: wgpu::ComputePipeline,
    finalize_pipeline: wgpu::RenderPipeline,
    blend: Blend,
    uniform: OffsetUniform,
    offset_buffer: wgpu::Buffer,
}

pub struct FrameState {
    pub depth_view: wgpu::TextureView,
    pub frame_view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

pub struct FxChainOutput<'a> {
    pub blend: BlendType,
    pub bind_group: &'a wgpu::BindGroup,
}

#[derive(ShaderType, Clone)]
pub struct OffsetUniform {
    offset: i32,
}

impl OffsetUniform {
    fn new(config: &wgpu::SurfaceConfiguration) -> Self {
        Self {
            offset: config.fx_offset() as i32,
        }
    }

    fn buffer_content(&self) -> Vec<u8> {
        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&self).unwrap();
        buffer.into_inner()
    }
}

pub const WORK_GROUP_SIZE: [f32; 2] = [8., 8.];

impl PostProcessState {
    pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

    fn render_output(&self) -> &wgpu::BindGroup {
        let nr = self.post_fx.iter().filter(|fx| fx.enabled()).count();

        self.fx_state.bind_group(nr)
    }

    pub fn resize(&mut self, gfx_state: &GfxState) {
        let config = &gfx_state.surface_config;
        let queue = &gfx_state.queue;

        self.uniform = OffsetUniform::new(config);
        queue.write_buffer(&self.offset_buffer, 0, &self.uniform.buffer_content());

        self.frame_state =
            FrameState::new(gfx_state, &self.frame_group_layout, &self.offset_buffer);
        self.fx_state.resize(config.fx_dimensions(), gfx_state);

        for pfx in self.post_fx.iter_mut() {
            pfx.resize(&gfx_state);
        }
    }

    pub fn blend<'a>(
        &'a self,
        input: FxChainOutput<'a>,
        output: &'a wgpu::BindGroup,
        c_pass: &mut wgpu::ComputePass<'a>,
    ) {
        let compute = BlendCompute {
            input: input.bind_group,
            output,
            count_x: self.fx_state.count_x,
            count_y: self.fx_state.count_y,
        };

        match input.blend {
            BlendType::ADDITIVE => self.blend.add(compute, c_pass),
            BlendType::BLEND => {
                todo!("todo")
            }
            BlendType::REPLACE => {
                todo!("todo")
            }
        }
    }

    pub fn compute(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut c_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Post process pipeline"),
        });

        c_pass.set_pipeline(&self.initialize_pipeline);
        c_pass.set_bind_group(0, &self.fx_state.bind_group(1), &[]);
        c_pass.set_bind_group(1, &self.frame_state.bind_group, &[]);
        c_pass.dispatch_workgroups(self.fx_state.count_x, self.fx_state.count_y, 1);

        for (i, pfx) in self.post_fx.iter().filter(|fx| fx.enabled()).enumerate() {
            let frame = self.fx_state.bind_group(i);
            let fx = pfx.compute(frame, &mut c_pass);

            self.blend(fx, frame, &mut c_pass);
        }
    }

    pub fn render<'a>(&'a self, r_pass: &mut wgpu::RenderPass<'a>) {
        r_pass.set_pipeline(&self.finalize_pipeline);
        r_pass.set_bind_group(0, self.render_output(), &[]);
        r_pass.set_bind_group(1, &self.frame_state.bind_group, &[]);
        r_pass.draw(0..3, 0..1);
    }

    pub fn create_fx_layout(
        device: &wgpu::Device,
        offset: &OffsetUniform,
    ) -> wgpu::BindGroupLayout {
        let min_binding_size = NonZeroU64::new(offset.buffer_content().len() as u64);

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Frame layout"),
            entries: &[
                // Frame
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        multisampled: false,
                    },
                    count: None,
                },
                // Offset uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn new(gfx_state: &GfxState) -> Self {
        let device = &gfx_state.device;
        let config = &gfx_state.surface_config;

        let initialize_shader = device.create_shader("fx/initialize.wgsl", "Init post fx");
        let finalize_shader = device.create_shader("fx/finalize.wgsl", "Finalize post fx");

        let uniform = OffsetUniform::new(config);
        let buffer_content = uniform.buffer_content();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Offset"),
            contents: &buffer_content,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let frame_group_layout = Self::create_fx_layout(&device, &uniform);
        let frame_state = FrameState::new(gfx_state, &frame_group_layout, &buffer);

        let fx_state = FxState::new(FxStateOptions {
            label: "Post process start".to_string(),
            tex_dimensions: config.fx_dimensions(),
            gfx_state,
        });

        let fx_group_layout = &fx_state.bind_group_layout;

        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Init layout"),
            bind_group_layouts: &[&fx_group_layout, &frame_group_layout],
            push_constant_ranges: &[],
        });

        let initialize_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Init pipeline"),
                layout: Some(&compute_layout),
                module: &initialize_shader,
                entry_point: "init",
            });

        let render_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Post fx render"),
            bind_group_layouts: &[&fx_group_layout, &frame_group_layout],
            push_constant_ranges: &[],
        });

        let finalize_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Finalize pipeline"),
            layout: Some(&render_layout),
            vertex: wgpu::VertexState {
                module: &finalize_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &finalize_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let blend = Blend::new(gfx_state, &fx_state);

        Self {
            frame_state,
            fx_state,
            post_fx: vec![],
            frame_group_layout,
            initialize_pipeline,
            finalize_pipeline,
            blend,
            offset_buffer: buffer,
            uniform,
        }
        .append_fx(gfx_state)
    }

    fn append_fx(mut self, gfx_state: &GfxState) -> Self {
        let bloom = Bloom::new(gfx_state, &self.frame_state.depth_view);

        self.post_fx.push(Box::new(bloom));

        return self;
    }
}

impl FrameState {
    pub fn new(
        gfx_state: &GfxState,
        bind_group_layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
    ) -> Self {
        let device = &gfx_state.device;

        let frame_view = gfx_state.create_frame_view();
        let depth_view = gfx_state.create_depth_view();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&frame_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            frame_view,
            depth_view,
            bind_group,
        }
    }
}

pub struct FxState {
    bind_groups: Vec<wgpu::BindGroup>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub count_x: u32,
    pub count_y: u32,
    pub label: String,
}

pub struct FxStateOptions<'a> {
    /// For debugging purposes
    pub label: String,
    pub tex_dimensions: Dimensions,
    pub gfx_state: &'a GfxState,
}

pub type Dimensions = [u32; 2];

impl FxState {
    pub fn bind_group(&self, idx: usize) -> &wgpu::BindGroup {
        &self.bind_groups[idx % 2]
    }

    fn create_bind_groups(
        dimensions: Dimensions,
        layout: &wgpu::BindGroupLayout,
        gfx_state: &GfxState,
    ) -> Vec<wgpu::BindGroup> {
        let device = &gfx_state.device;

        let mut bind_groups = Vec::new();
        let fx_view_1 = gfx_state.create_fx_view(dimensions);
        let fx_view_2 = gfx_state.create_fx_view(dimensions);
        let fx_views = vec![fx_view_1, fx_view_2];

        for i in 0..2 {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&fx_views[i % 2]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&fx_views[(i + 1) % 2]),
                    },
                ],
            });

            bind_groups.push(bg);
        }

        return bind_groups;
    }

    fn get_dispatch_counts(dimensions: Dimensions) -> [u32; 2] {
        let count_x = (dimensions[0] as f32 / WORK_GROUP_SIZE[0]).ceil() as u32;
        let count_y = (dimensions[1] as f32 / WORK_GROUP_SIZE[1]).ceil() as u32;

        return [count_x, count_y];
    }

    pub fn resize(&mut self, dimensions: Dimensions, gfx_state: &GfxState) {
        let counts = Self::get_dispatch_counts(dimensions);

        self.bind_groups = Self::create_bind_groups(dimensions, &self.bind_group_layout, gfx_state);
        self.count_x = counts[0];
        self.count_y = counts[1];
    }

    pub fn new(options: FxStateOptions) -> Self {
        let FxStateOptions {
            label,
            tex_dimensions,
            gfx_state,
        } = options;

        let device = &gfx_state.device;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bloom textures layout"),
            entries: &[
                // FX Write
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        view_dimension: wgpu::TextureViewDimension::D2,
                        format: PostProcessState::TEXTURE_FORMAT,
                        access: wgpu::StorageTextureAccess::WriteOnly,
                    },
                    count: None,
                },
                // FX Read
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let bind_groups = Self::create_bind_groups(tex_dimensions, &bind_group_layout, gfx_state);
        let counts = Self::get_dispatch_counts(tex_dimensions);

        Self {
            label,
            bind_groups,
            bind_group_layout,
            count_x: counts[0],
            count_y: counts[1],
        }
    }
}

impl FxDimensions for wgpu::SurfaceConfiguration {
    fn fx_dimensions(&self) -> Dimensions {
        let expand = self.fx_offset() * 2;

        [self.width + expand, self.height + expand]
    }

    fn fx_offset(&self) -> u32 {
        (self.width / 60).max(32)
    }
}
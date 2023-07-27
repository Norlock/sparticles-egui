use egui_wgpu_backend::wgpu::{self, util::DeviceExt};
use glam::*;

use super::gfx_state;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4 {
    x_axis: Vec4::new(1.0, 0.0, 0.0, 0.0),
    y_axis: Vec4::new(0.0, 1.0, 0.0, 0.0),
    z_axis: Vec4::new(0.0, 0.0, 0.5, 0.0),
    w_axis: Vec4::new(0.0, 0.0, 0.5, 1.0),
};

pub struct Camera {
    position: glam::Vec3,    // Camera position
    focus_point: glam::Vec3, // Where does the camera look at?
    up: glam::Vec3,          // What way is up
    fov: f32,                // Field of view (frustum vertical degrees)
    aspect: f32,             // Make sure x/y stays in aspect
    near: f32,               // What is too close to show
    far: f32,                // What is too far to show
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Camera {
    pub fn new(gfx_state: &gfx_state::GfxState) -> Self {
        let device = &gfx_state.device;
        let surface_config = &gfx_state.surface_config;

        let position = glam::Vec3::new(0., 0., 10.);
        let focus_point = glam::Vec3::new(0., 0., 0.);

        let near = 0.1;
        let far = 100.0;
        let fov = (45.0f32).to_radians();
        let up = glam::Vec3::Y;

        let aspect = surface_config.width as f32 / surface_config.height as f32;

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            label: Some("Camera buffer"),
            contents: bytemuck::cast_slice(&[0.0; 16]),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            up,
            fov,
            far,
            near,
            position,
            buffer,
            bind_group_layout,
            bind_group,
            focus_point,
            aspect,
        }
    }

    fn create_view_projection_matrix(&self) -> [f32; 16] {
        let view = glam::Mat4::look_at_rh(self.position, self.focus_point, self.up);
        let proj = glam::Mat4::perspective_lh(self.fov, self.aspect, self.near, self.far);

        (OPENGL_TO_WGPU_MATRIX * proj * view).to_cols_array()
    }
}

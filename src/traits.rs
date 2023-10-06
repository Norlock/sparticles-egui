use crate::{
    fx::post_process::{FxPersistenceType, FxState, FxView},
    model::{Clock, EmitterState, EmitterUniform, GfxState, State},
};
use egui_wgpu::wgpu;
use egui_winit::egui::Ui;
use std::{
    num::NonZeroU64,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub trait FromRGB {
    fn from_rgb(r: u8, g: u8, b: u8) -> Self;
}

pub trait FromRGBA {
    fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self;
}

pub trait CustomShader {
    fn create_shader(&self, shader_str_raw: &str, label: &str) -> wgpu::ShaderModule;
}

pub trait CreateGui {
    fn create_gui(&self, app_state: &mut State);
}

pub trait ToVecF32 {
    fn to_vec_f32(&self) -> Vec<f32>;
}

pub trait CreateAspect {
    fn aspect(&self) -> f32;
}

pub trait RegisterParticleAnimation {
    fn tag(&self) -> &str;

    fn create_default(
        &self,
        gfx_state: &GfxState,
        emitter: &EmitterState,
    ) -> Box<dyn ParticleAnimation>;

    fn dyn_clone(&self) -> Box<dyn RegisterParticleAnimation>;
}

impl PartialEq for dyn RegisterParticleAnimation {
    fn eq(&self, other: &Self) -> bool {
        self.tag() == other.tag()
    }
}

pub trait ParticleAnimation {
    fn update(&mut self, clock: &Clock, gfx_state: &GfxState);

    fn compute<'a>(
        &'a self,
        spawner: &'a EmitterState,
        clock: &Clock,
        compute_pass: &mut wgpu::ComputePass<'a>,
    );

    fn recreate(
        self: Box<Self>,
        gfx_state: &GfxState,
        spawner: &EmitterState,
    ) -> Box<dyn ParticleAnimation>;

    fn create_gui(&mut self, ui: &mut Ui);
}

pub trait EmitterAnimation {
    fn animate(&mut self, emitter: &mut EmitterUniform, clock: &Clock);
    fn create_gui(&mut self, ui: &mut Ui);
}

pub trait CalculateBufferSize {
    fn cal_buffer_size(&self) -> Option<NonZeroU64>;
}

pub trait HandleAngles {
    fn to_degrees(&self) -> Self;
    fn to_radians(&self) -> Self;
}

pub trait PostFx {
    fn compute<'a>(
        &'a self,
        fx_inputs: Vec<&'a Rc<wgpu::BindGroup>>,
        data: &mut wgpu::ComputePass<'a>,
    );

    fn resize(&mut self, gfx_state: &GfxState);
    fn output(&self) -> &Rc<wgpu::BindGroup>;
    fn create_ui(&mut self, ui: &mut Ui, gfx_state: &GfxState);
}

pub trait PostFxChain {
    fn compute<'a>(&'a self, input: &'a Rc<wgpu::BindGroup>, c_pass: &mut wgpu::ComputePass<'a>);

    fn resize(&mut self, gfx_state: &GfxState, fx_state: &FxState);
    fn create_ui(&mut self, ui: &mut Ui, gfx_state: &GfxState);

    fn add_views(&self, fx_views: &mut Vec<FxView>, idx: usize);
    fn export(&self) -> FxPersistenceType;

    fn enabled(&self) -> bool;
    fn delete(&self) -> bool;
}

pub trait CreateFxView {
    fn default_view(&self) -> wgpu::TextureView;
}

pub trait FxDimensions {
    fn fx_dimensions(&self) -> [u32; 2];
    fn fx_offset(&self) -> u32;
}

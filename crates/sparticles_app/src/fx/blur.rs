use std::sync::Arc;

use super::blur_pass::BlurPass;
use super::blur_pass::BlurPassSettings;
use super::FxOptions;
use super::FxState;
use crate::model::Camera;
use crate::model::GfxState;
use crate::traits::*;
use crate::util::DynamicExport;
use crate::util::ListAction;
use crate::util::UniformContext;
use async_std::sync::RwLock;
use egui_wgpu::wgpu;
use encase::ShaderType;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum BlurType {
    Gaussian,
    Box,
    Sharpen,
}

pub enum BlurEvent {
    UpdateUniform,
}

pub struct BlurFx {
    pub blur_uniform: BlurUniform,
    pub blur_ctx: UniformContext,
    pub blur_type: BlurType,
    pub blur_pass: BlurPass,

    pub update_uniform: Option<BlurEvent>,

    pub selected_action: ListAction,
    pub enabled: bool,
}

#[derive(ShaderType, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BlurUniform {
    pub brightness_threshold: f32,

    // How far to look
    pub radius: i32,
    pub sigma: f32,
    pub intensity: f32,
}

pub struct RegisterBlurFx;

// Create default is used as single fx
impl RegisterPostFx for RegisterBlurFx {
    fn tag(&self) -> &'static str {
        "blur"
    }

    fn create_default(&self, options: &FxOptions) -> Box<dyn PostFx> {
        let settings = BlurSettings {
            blur_uniform: BlurUniform::default(),
            blur_type: BlurType::Gaussian,
        };

        Box::new(BlurFx::new(options, settings))
    }

    fn import(&self, options: &FxOptions, value: serde_json::Value) -> Box<dyn PostFx> {
        let settings = serde_json::from_value(value).expect("Can't parse blur");

        Box::new(BlurFx::new(options, settings))
    }
}

impl Default for BlurUniform {
    fn default() -> Self {
        Self {
            brightness_threshold: 0.6,
            radius: 4,
            sigma: 1.3,
            intensity: 1.00, // betere naam verzinnen
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct BlurSettings {
    pub blur_uniform: BlurUniform,
    pub blur_type: BlurType,
}

impl PostFx for BlurFx {
    fn resize(&mut self, options: &FxOptions) {
        self.blur_pass.resize(options);
    }

    fn update(&mut self, gfx_state: &GfxState, _camera: &mut Camera) {
        if self.update_uniform.take().is_some() {
            let queue = &gfx_state.queue;
            let buffer_content = self.blur_uniform.buffer_content();
            queue.write_buffer(&self.blur_ctx.buf, 0, &buffer_content);
        }
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn compute<'a>(
        &'a self,
        fx_state: &'a FxState,
        gfx: &Arc<RwLock<GfxState>>,
        c_pass: &mut wgpu::ComputePass<'a>,
    ) {
        let bp = &self.blur_pass;

        if self.blur_type == BlurType::Gaussian {
            bp.compute_gaussian(fx_state, gfx, &self.blur_ctx.bg, c_pass);
        }
    }
}

impl HandleAction for BlurFx {
    fn selected_action(&mut self) -> &mut ListAction {
        &mut self.selected_action
    }

    fn export(&self) -> DynamicExport {
        let settings = BlurSettings {
            blur_uniform: self.blur_uniform,
            blur_type: self.blur_type,
        };

        DynamicExport {
            tag: RegisterBlurFx.tag().to_string(),
            data: serde_json::to_value(settings).expect("Can't create export for blur fx"),
        }
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

impl BlurFx {
    pub fn new(options: &FxOptions, blur_settings: BlurSettings) -> Self {
        let FxOptions { gfx: gfx_state, .. } = options;

        let device = &gfx_state.device;

        let BlurSettings {
            blur_uniform,
            blur_type,
        } = blur_settings;

        let blur_ctx = UniformContext::from_uniform(&blur_uniform, device, "Blur");

        let blur_pass = BlurPass::new(
            options,
            BlurPassSettings {
                blur_layout: &blur_ctx.bg_layout,
                io_idx: (0, 2),
                downscale: 1.,
            },
        );

        Self {
            blur_ctx,
            blur_uniform,
            blur_type,
            blur_pass,

            update_uniform: None,
            enabled: true,
            selected_action: ListAction::None,
        }
    }
}

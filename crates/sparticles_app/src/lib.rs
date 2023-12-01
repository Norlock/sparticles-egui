use egui_wgpu;
use egui_winit;
use egui_winit::winit;
use init::AppVisitor;
use model::{Events, GfxState, State};
use winit::event::Event::*;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{self, WindowId};

pub use egui_wgpu::wgpu;
pub use glam;
pub use wgpu_profiler as profiler;

pub mod gui {
    pub use egui_wgpu::*;
    pub use egui_winit::*;
}

pub mod animations;
pub mod fx;
pub mod init;
pub mod loader;
pub mod model;
pub mod shaders;
pub mod texture;
pub mod traits;
pub mod util;

pub fn start(mut app_visitor: impl AppVisitor + 'static) {
    env_logger::init();

    let event_loop = EventLoop::new();

    let window = window::WindowBuilder::new()
        .with_decorations(true)
        .with_transparent(false)
        //.with_resizable(false)
        //.with_max_inner_size(PhysicalSize::new(1920., 1080.))
        .with_title("Sparticles")
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(&mut app_visitor, window);
    let mut shift_pressed = false;
    let mut events = Events::default();

    event_loop.run(move |event, _, control_flow| {
        let gfx = &mut state.gfx;
        let do_exec = |window_id: WindowId| window_id == gfx.window_id();

        match event {
            RedrawRequested(window_id) if do_exec(window_id) => {
                state.update(&events);
                events = GfxState::render(&mut state, &mut app_visitor);
            }
            MainEventsCleared => {
                gfx.request_redraw();
            }
            WindowEvent { event, window_id } if do_exec(window_id) => {
                let response = gfx.handle_event(&event);

                match event {
                    winit::event::WindowEvent::Resized(size) => {
                        state.resize(size);
                    }
                    winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(*new_inner_size);
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    winit::event::WindowEvent::KeyboardInput { input, .. } => {
                        if !response.consumed {
                            state.process_events(input, shift_pressed);
                        }
                    }
                    winit::event::WindowEvent::ModifiersChanged(modifier) => {
                        shift_pressed = modifier.shift()
                    }
                    _ => {}
                }
            }
            _ => (),
        }
    });
}
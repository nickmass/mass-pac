use glium::glutin::config::ConfigTemplateBuilder;
use glium::winit;
use winit::keyboard::PhysicalKey;

use super::audio::Audio;
use super::filters::Filter;
use super::gfx::{Gfx, GfxBackBuffer, GliumContext};
use super::input::InputMap;

use crate::pacman::UserInput;

pub enum UserEvent {
    Frame,
}

#[derive(Debug, Copy, Clone)]
pub enum EmulatorInput {
    System(UserInput),
}

impl From<UserInput> for EmulatorInput {
    fn from(value: UserInput) -> Self {
        EmulatorInput::System(value)
    }
}

pub struct App<const ROT_90: bool, F, A> {
    audio: A,
    gfx: Gfx<ROT_90, F>,
    window: winit::window::Window,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<std::sync::mpsc::Sender<EmulatorInput>>,
    back_buffer: GfxBackBuffer,
    pause: bool,
}

impl<const ROT_90: bool, F: Filter<GliumContext>, A: Audio> App<ROT_90, F, A> {
    pub fn new(filter: F, audio: A) -> Self {
        let event_loop = winit::event_loop::EventLoop::with_user_event()
            .build()
            .unwrap();

        let dims = if ROT_90 {
            let (height, width) = filter.dimensions();
            (width, height)
        } else {
            filter.dimensions()
        };

        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_config_template_builder(
                ConfigTemplateBuilder::new().with_swap_interval(None, None),
            )
            .with_vsync(false)
            .with_inner_size(dims.0 * 2, dims.1 * 2)
            .with_title("Mass Pac")
            .build(&event_loop);

        let proxy = event_loop.create_proxy();
        let back_buffer = GfxBackBuffer::new(proxy);
        let gfx = Gfx::new(display, back_buffer.clone(), filter);

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        window.set_cursor_visible(false);

        Self {
            audio,
            window,
            gfx,
            back_buffer,
            event_loop: Some(event_loop),
            input: InputMap::new(),
            input_tx: None,
            pause: false,
        }
    }

    pub fn back_buffer(&self) -> GfxBackBuffer {
        self.back_buffer.clone()
    }

    pub fn system_io(&mut self) -> SystemInputs {
        let (tx, rx) = std::sync::mpsc::channel();
        self.input_tx = Some(tx);

        SystemInputs { rx }
    }

    fn send_inputs(&self) {
        if let Some(tx) = self.input_tx.as_ref() {
            let p1 = self.input.controller();

            if self.input.reset() {
                let _ = tx.send(UserInput::Reset.into());
            }

            let _ = tx.send(UserInput::Player(p1).into());
        }
    }

    pub fn run(mut self) -> ! {
        let Some(event_loop) = self.event_loop.take() else {
            panic!("no event loop created");
        };

        self.audio.play();

        let Err(err) = event_loop.run_app(&mut self) else {
            std::process::exit(0)
        };

        panic!("{:?}", err)
    }
}

impl<const ROT_90: bool, F: Filter<GliumContext>, A: Audio>
    winit::application::ApplicationHandler<UserEvent> for App<ROT_90, F, A>
{
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::Resized(size) => {
                self.gfx.resize(size.into());
                self.window.request_redraw();
            }
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if event.state.is_pressed() {
                        self.input.press(key);
                    } else {
                        self.input.release(key);
                    }

                    if self.input.pause() {
                        self.pause = !self.pause;
                        if self.pause {
                            self.audio.pause();
                        } else {
                            self.audio.play();
                        }
                    }
                }
            }
            winit::event::WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer: _,
            } => {
                self.window.request_redraw();
            }
            winit::event::WindowEvent::RedrawRequested => {
                if self.window.is_visible() != Some(false) {
                    self.gfx.render();
                }
            }
            _ => (),
        }
        self.send_inputs();
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Frame => {
                self.gfx.swap();
                self.window.request_redraw();
            }
        }
    }
}

pub struct SystemInputs {
    rx: std::sync::mpsc::Receiver<EmulatorInput>,
}

impl SystemInputs {
    pub fn try_inputs(&mut self) -> impl Iterator<Item = EmulatorInput> + '_ {
        self.rx.try_iter()
    }
}

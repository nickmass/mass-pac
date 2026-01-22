use std::time::{Duration, Instant};

use glium::glutin::config::ConfigTemplateBuilder;
use glium::winit;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::Window;

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

pub struct App<F, A> {
    audio: A,
    gfx: Gfx<F>,
    window: Window,
    event_loop: Option<EventLoop<UserEvent>>,
    input: InputMap,
    input_tx: Option<std::sync::mpsc::Sender<EmulatorInput>>,
    back_buffer: GfxBackBuffer,
    pause: bool,
    mouse_show_time: Instant,
    cursor_visible: bool,
}

impl<F: Filter<GliumContext>, A: Audio> App<F, A> {
    pub fn new(filter: F, audio: A) -> Self {
        let event_loop = EventLoop::with_user_event().build().unwrap();

        // rotate dimensions to adjust for game rendering at 90 degrees
        let (width, height) = {
            let (height, width) = filter.dimensions();
            (width, height)
        };

        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_config_template_builder(
                ConfigTemplateBuilder::new().with_swap_interval(None, None),
            )
            .with_vsync(false)
            .with_inner_size(width * 2, height * 2)
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
            mouse_show_time: Instant::now(),
            cursor_visible: false,
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

    fn update_cursor(&mut self, mouse_moved: bool) {
        if mouse_moved && !self.cursor_visible {
            self.window.set_cursor_visible(true);
            self.mouse_show_time = Instant::now();
            self.cursor_visible = true;
        } else if !mouse_moved
            && self.cursor_visible
            && self.mouse_show_time.elapsed() > Duration::from_secs(1)
        {
            self.window.set_cursor_visible(false);
            self.cursor_visible = false;
        }
    }
}

impl<F: Filter<GliumContext>, A: Audio> winit::application::ApplicationHandler<UserEvent>
    for App<F, A>
{
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.update_cursor(false);

        match event {
            WindowEvent::Resized(size) => {
                self.gfx.resize(size.into());
                self.window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
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
            WindowEvent::ScaleFactorChanged { .. } => {
                self.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if self.window.is_visible() != Some(false) {
                    self.gfx.render();
                }
            }
            WindowEvent::CursorMoved { .. } => {
                self.update_cursor(true);
            }
            _ => (),
        }
        self.send_inputs();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
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

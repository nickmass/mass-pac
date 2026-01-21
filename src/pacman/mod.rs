use std::cell::Cell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, ContextBuilder, LocalWake, LocalWaker, Waker};

mod input;
mod mem;
mod rom;
mod sound;
mod video;
mod z80;

use input::Input;
pub use input::{DipSettings, Player, UserInput, dip};
use mem::Memory;
pub use rom::{Rom, RomBuilder};
use sound::{Samples, Sound};
use video::Video;
use z80::{Cpu, CpuPinInputs, CpuPinOutputs};

const CPU_CLOCK: u64 = 3072000;

pub struct System {
    cpu: Ticker<CpuTickState>,
    memory: Memory,
    video: Video,
    sound: Sound,
    input: Input,
    cpu_input: CpuPinInputs,
    rom: Rom,
}

impl System {
    pub fn new(rom: Rom, dip: DipSettings) -> Self {
        let cpu = Cpu::new();

        Self {
            cpu: Ticker::new(CpuTickState::default(), cpu.run()),
            memory: Memory::new(),
            video: Video::new(&rom),
            sound: Sound::new(),
            input: Input::new(dip),
            cpu_input: CpuPinInputs::default(),
            rom,
        }
    }

    pub fn handle_input(&mut self, input: UserInput) {
        self.input.handle_input(input);
    }

    pub fn run(&mut self, samples: u32) {
        let samples = samples as usize;
        while !self.sound.samples_full(samples) {
            let output = self.cpu.tick(self.cpu_input);

            let data = match output {
                CpuPinOutputs::Read(address) => {
                    let data = self.rom.read(address, 0xff);
                    let data = self.memory.read(address, data);
                    let data = self.input.read(address, data);
                    Some(data)
                }
                CpuPinOutputs::Write(address, value) => {
                    self.rom.write(address, value);
                    self.memory.write(address, value);
                    self.video.write(address, value);
                    self.sound.write(address, value);
                    None
                }
                CpuPinOutputs::IoRead(_address) => None,
                CpuPinOutputs::IoWrite(address, value) => {
                    self.video.io_write(address, value);
                    None
                }
                CpuPinOutputs::InterruptAck => Some(self.video.interrupt_ack()),
                CpuPinOutputs::Idle => None,
            };

            self.sound.tick(&self.rom);
            self.video.tick(&self.rom, &self.memory);

            self.cpu_input = CpuPinInputs {
                data: data.unwrap_or(0xff),
                int_req: self.video.interrupt_req(),
                reset: self.input.reset(),
                ..Default::default()
            };
        }
    }

    pub fn frame(&self) -> u32 {
        self.video.frame()
    }

    pub fn screen(&self) -> &[u8] {
        &self.video.screen()
    }

    pub fn audio_clock(&self) -> f64 {
        self.sound.audio_clock()
    }

    pub fn samples(&mut self) -> Samples<'_> {
        self.sound.samples()
    }
}

struct Ticker<T> {
    future: Pin<Box<dyn Future<Output = ()>>>,
    waker: Rc<T>,
}

impl<T: TickState> Ticker<T> {
    fn new<F: Future<Output = ()> + 'static>(state: T, future: F) -> Self {
        Self {
            future: Box::pin(future),
            waker: Rc::new(state),
        }
    }

    fn tick(&mut self, input: T::Input) -> T::Output {
        self.waker.set_input(input);
        let local_waker = LocalWaker::from(self.waker.clone());
        let mut ctx = ContextBuilder::from_waker(Waker::noop())
            .local_waker(&local_waker)
            .build();
        let _ = self.future.as_mut().poll(&mut ctx);
        let state = T::from_context(&ctx);
        state.output()
    }
}

trait TickState: LocalWake + 'static {
    type Input;
    type Output;
    fn set_input(&self, input: Self::Input);
    fn output(&self) -> Self::Output;
    fn from_context<'a>(cx: &'a Context<'_>) -> &'a Self;
}

#[derive(Default)]
struct CpuTickState {
    input: Cell<CpuPinInputs>,
    output: Cell<CpuPinOutputs>,
}

impl CpuTickState {
    fn set_input(&self, input: CpuPinInputs) {
        self.input.set(input);
    }

    fn input(&self) -> CpuPinInputs {
        self.input.get()
    }

    fn set_output(&self, output: CpuPinOutputs) {
        self.output.set(output);
    }

    fn output(&self) -> CpuPinOutputs {
        self.output.get()
    }
}

impl TickState for CpuTickState {
    type Input = CpuPinInputs;

    type Output = CpuPinOutputs;

    fn set_input(&self, input: Self::Input) {
        self.set_input(input);
    }

    fn output(&self) -> Self::Output {
        self.output()
    }

    fn from_context<'a>(cx: &'a Context<'_>) -> &'a Self {
        unsafe { &*(cx.local_waker().data() as *const Self) }
    }
}

impl LocalWake for CpuTickState {
    fn wake(self: Rc<Self>) {}
}

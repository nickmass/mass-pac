#![feature(local_waker)]

use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, ContextBuilder, LocalWake, LocalWaker, Waker};

use save_states::SaveState;

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
use serde::{Deserialize, Serialize};
use sound::{Samples, Sound};
use video::Video;
use z80::{Cpu, CpuPinInputs, CpuPinOutputs};

const CPU_CLOCK: u64 = 3072000;

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveData {
    cpu: z80::CpuData,
    memory: mem::MemoryData,
    video: video::VideoData,
    sound: sound::SoundData,
    input: input::InputData,
    cpu_input: CpuPinInputs,
    rom: rom::RomData,
}

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
                CpuPinOutputs::Break => {
                    break;
                }
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

    pub fn save_state(&mut self) -> SaveData {
        self.cpu.request_save_state();
        self.run(self.audio_clock() as u32);
        let cpu = self
            .cpu
            .save_data()
            .expect("cpu returned without setting save data");

        SaveData {
            cpu,
            memory: self.memory.save_state(),
            video: self.video.save_state(),
            sound: self.sound.save_state(),
            input: self.input.save_state(),
            cpu_input: self.cpu_input.clone(),
            rom: self.rom.save_state(),
        }
    }

    pub fn restore_state(&mut self, state: SaveData) {
        self.cpu.request_break();
        self.run(self.audio_clock() as u32);
        self.cpu.set_save_data(state.cpu);
        self.memory.restore_state(&state.memory);
        self.video.restore_state(&state.video);
        self.sound.restore_state(&state.sound);
        self.input.restore_state(&state.input);
        self.cpu_input = state.cpu_input.clone();
        self.rom.restore_state(&state.rom);
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

    fn tick<I: Into<T::Input>>(&mut self, input: I) -> T::Output {
        let input = input.into();
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

impl Ticker<CpuTickState> {
    fn request_break(&self) {
        self.waker.set_request(TickRequest::Break);
    }

    fn request_save_state(&self) {
        self.waker.set_request(TickRequest::SaveState);
    }

    fn set_save_data(&self, data: z80::CpuData) {
        self.waker.set_request(TickRequest::RestoreState(data));
    }

    fn save_data(&self) -> Option<z80::CpuData> {
        self.waker.take_save_data()
    }
}

trait TickState: LocalWake + 'static {
    type Input;
    type Output;
    fn set_input(&self, input: Self::Input);
    fn output(&self) -> Self::Output;
    fn from_context<'a>(cx: &'a Context<'_>) -> &'a Self;
}

enum TickRequest {
    Break,
    SaveState,
    RestoreState(z80::CpuData),
}

#[derive(Default)]
struct CpuTickState {
    input: Cell<CpuPinInputs>,
    output: Cell<CpuPinOutputs>,
    save_data: RefCell<Option<z80::CpuData>>,
    request: RefCell<Option<TickRequest>>,
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

    fn set_request(&self, req: TickRequest) {
        self.request.replace(Some(req));
    }

    fn take_request(&self) -> Option<TickRequest> {
        self.request.take()
    }

    fn set_save_data(&self, data: z80::CpuData) {
        self.save_data.replace(Some(data));
    }

    fn take_save_data(&self) -> Option<z80::CpuData> {
        self.save_data.replace(None)
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

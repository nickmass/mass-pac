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
use z80::{Cpu, CpuPinInputs, CpuPinOutputs, CpuTickInput};

const CPU_CLOCK: u64 = 3072000;

pub use SystemData as SaveData;

#[derive(SaveState)]
pub struct System {
    #[save(nested)]
    cpu: Ticker<CpuTickState>,
    #[save(nested)]
    memory: Memory,
    #[save(nested)]
    video: Video,
    #[save(nested)]
    sound: Sound,
    #[save(nested)]
    input: Input,
    cpu_input: CpuPinInputs,
    #[save(nested)]
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

    pub fn save_state(&self) -> SaveData {
        <Self as SaveState>::save_state(self)
    }

    pub fn restore_state(&mut self, state: &SaveData) {
        <Self as SaveState>::restore_state(self, state);
    }
}

struct Ticker<T> {
    future: RefCell<Pin<Box<dyn Future<Output = ()>>>>,
    waker: Rc<T>,
}

impl<T: TickState> Ticker<T> {
    fn new<F: Future<Output = ()> + 'static>(state: T, future: F) -> Self {
        Self {
            future: RefCell::new(Box::pin(future)),
            waker: Rc::new(state),
        }
    }

    fn tick<I: Into<T::Input>>(&self, input: I) -> T::Output {
        let input = input.into();
        self.waker.set_input(input);
        let local_waker = LocalWaker::from(self.waker.clone());
        let mut ctx = ContextBuilder::from_waker(Waker::noop())
            .local_waker(&local_waker)
            .build();
        let mut future = self.future.borrow_mut();
        let _ = future.as_mut().poll(&mut ctx);
        let state = T::from_context(&ctx);
        state.output()
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct CpuTickerData {
    cpu: z80::CpuData,
    input: CpuTickInput,
    output: CpuPinOutputs,
}

impl SaveState for Ticker<CpuTickState> {
    type Data = CpuTickerData;

    fn save_state(&self) -> Self::Data {
        let _ = self.tick(CpuTickInput::SaveState);
        let cpu = self.waker.save_data().expect("no CpuData stored");
        CpuTickerData {
            cpu,
            input: self.waker.save_input.get(),
            output: self.waker.save_output.get(),
        }
    }

    fn restore_state(&mut self, state: &Self::Data) {
        self.waker.set_checkpoint(false, state.output);
        while !self.waker.checkpoint() {
            let _ = self.tick(CpuTickInput::Tick(CpuPinInputs::default()));
        }
        self.waker.set_save_data(state.cpu.clone());
        let _ = self.tick(CpuTickInput::RestoreState(state.output));
        self.waker.set_input(state.input);
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
    input: Cell<CpuTickInput>,
    output: Cell<CpuPinOutputs>,
    checkpoint: Cell<bool>,
    checkpoint_input: Cell<CpuTickInput>,
    checkpoint_output: Cell<CpuPinOutputs>,
    save_input: Cell<CpuTickInput>,
    save_output: Cell<CpuPinOutputs>,
    save_data: RefCell<Option<z80::CpuData>>,
}

impl CpuTickState {
    fn set_input(&self, input: CpuTickInput) {
        self.input.set(input);
    }

    fn input(&self) -> CpuTickInput {
        self.input.get()
    }

    fn set_output(&self, output: CpuPinOutputs) {
        self.output.set(output);
    }

    fn output(&self) -> CpuPinOutputs {
        self.output.get()
    }

    fn set_save_data(&self, data: z80::CpuData) {
        *self.save_data.borrow_mut() = Some(data);
        self.save_input.set(self.checkpoint_input.get());
        self.save_output.set(self.checkpoint_output.get());
    }

    fn save_data(&self) -> Option<z80::CpuData> {
        self.save_data.borrow().clone()
    }

    fn set_checkpoint(&self, checkpoint: bool, output: CpuPinOutputs) {
        self.checkpoint.set(checkpoint);
        if checkpoint {
            self.checkpoint_input.set(self.input());
            self.checkpoint_output.set(output);
        }
    }

    fn checkpoint(&self) -> bool {
        self.checkpoint.get()
    }
}

impl TickState for CpuTickState {
    type Input = CpuTickInput;

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

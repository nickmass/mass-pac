use std::time::Duration;

use super::audio::SamplesSender;
use blip_buf::BlipBuf;

use super::app::{EmulatorInput, SystemInputs};
use super::gfx::GfxBackBuffer;

use crate::pacman::{DipSettings, Rom, System, dip};

pub struct Runner {
    machine: System,
    back_buffer: GfxBackBuffer,
    inputs: Option<SystemInputs>,
    samples_tx: SamplesSender,
    blip: BlipBuf,
    blip_delta: i32,
    frame: Option<u32>,
}

impl Runner {
    pub fn new(
        rom: Rom,
        inputs: SystemInputs,
        back_buffer: GfxBackBuffer,
        samples_tx: SamplesSender,
        sample_rate: u32,
    ) -> Self {
        let mut dip = DipSettings::default();
        dip.cost = dip::Cost::OneCreditPerCoin;
        dip.bonus = dip::BonusLife::Score10000;
        let machine = System::new(rom, dip);
        let mut blip = BlipBuf::new(sample_rate / 20);
        blip.set_rates(machine.audio_clock(), sample_rate as f64);

        Self {
            machine,
            back_buffer,
            inputs: Some(inputs),
            samples_tx,
            blip,
            blip_delta: 0,
            frame: None,
        }
    }

    pub fn run(mut self) {
        let Some(mut inputs) = self.inputs.take() else {
            panic!("inputs taken");
        };

        loop {
            for input in inputs.try_inputs() {
                match input {
                    EmulatorInput::System(input) => self.machine.handle_input(input),
                }
            }

            if let Some(samples) = self
                .samples_tx
                .wait_for_wants_samples(Duration::from_millis(1))
            {
                self.step(samples as u32);
            }
        }
    }

    fn step(&mut self, samples: u32) {
        self.machine.run(samples * 2);

        self.update_audio();
        let frame = self.machine.frame();
        if self.frame != Some(frame) {
            self.frame = Some(frame);
            self.update_frame();
        }
    }

    fn update_audio(&mut self) {
        let mut count = 0;
        for (i, &v) in self.machine.samples().iter().enumerate() {
            self.blip.add_delta(i as u32, v as i32 - self.blip_delta);
            self.blip_delta = v as i32;
            count += 1;
        }
        self.blip.end_frame(count as u32);
        self.samples_tx.add_samples_from_blip(&mut self.blip);
    }

    fn update_frame(&mut self) {
        self.back_buffer.update(|frame| {
            frame.copy_from_slice(self.machine.screen());
        });
    }
}

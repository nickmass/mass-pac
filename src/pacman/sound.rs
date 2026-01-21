use super::CPU_CLOCK;
use super::rom::BaseRom;

struct WavData;

impl WavData {
    fn sample(rom: &BaseRom, waveform: u8, offset: u8) -> i16 {
        let address = (waveform as u16 & 0x7) << 5 | (offset as u16);
        rom.snd(address) as i16 - 7
    }
}

const AUDIO_CLOCK: u64 = 96000;
const PERIOD: u64 = CPU_CLOCK / AUDIO_CLOCK;

pub struct Sound {
    counter: u64,
    samples: Vec<i16>,
    regs: [u8; 0x20],
    chan_0: ChannelState<0>,
    chan_1: ChannelState<1>,
    chan_2: ChannelState<2>,
    enabled: bool,
    sample: i16,
}

impl Sound {
    pub fn new() -> Self {
        Self {
            counter: PERIOD,
            samples: Vec::new(),
            regs: [0; _],
            chan_0: ChannelState::new(),
            chan_1: ChannelState::new(),
            chan_2: ChannelState::new(),
            enabled: false,
            sample: 0,
        }
    }

    pub fn audio_clock(&self) -> f64 {
        AUDIO_CLOCK as f64
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let address = address & 0x7fff;
        if address == 0x5001 {
            self.enabled = value & 1 != 0;
        } else if address >= 0x5040 && address < 0x5060 {
            self.regs[address as usize - 0x5040] = value;
        }
    }

    pub fn tick(&mut self, rom: &BaseRom) {
        self.counter -= 1;
        if self.counter != 0 {
            return;
        }

        self.counter = PERIOD;

        if self.enabled {
            let mut sample = 0;

            let mut chan_0 = self.chan_0.channel(&self.regs);
            let mut chan_1 = self.chan_1.channel(&self.regs);
            let mut chan_2 = self.chan_2.channel(&self.regs);

            chan_0.tick(rom);
            chan_1.tick(rom);
            chan_2.tick(rom);

            sample += chan_0.sample();
            sample += chan_1.sample();
            sample += chan_2.sample();

            self.sample = sample << 6;
        }

        self.samples.push(self.sample)
    }

    pub fn samples_full(&self, samples: usize) -> bool {
        self.samples.len() >= samples
    }

    pub fn samples(&mut self) -> Samples<'_> {
        Samples(&mut self.samples)
    }
}

pub struct Samples<'a>(&'a mut Vec<i16>);

impl<'a> std::ops::Deref for Samples<'a> {
    type Target = [i16];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> std::ops::Drop for Samples<'a> {
    fn drop(&mut self) {
        self.0.clear();
    }
}

struct ChannelState<const ID: usize> {
    accumulator: u32,
    sample: i16,
}

impl<const ID: usize> ChannelState<ID> {
    fn new() -> Self {
        Self {
            accumulator: 0,
            sample: 0,
        }
    }

    fn channel<'a>(&'a mut self, regs: &'a [u8; 0x20]) -> Channel<'a, ID> {
        Channel { inner: self, regs }
    }
}

struct Channel<'a, const ID: usize> {
    inner: &'a mut ChannelState<ID>,
    regs: &'a [u8; 0x20],
}

impl<'a, const ID: usize> Channel<'a, ID> {
    fn base() -> usize {
        ID * 5
    }

    fn volume(&self) -> u8 {
        self.regs[Self::base() + 0x15] & 0x0f
    }

    fn waveform(&self) -> u8 {
        self.regs[Self::base() + 0x05] & 0x07
    }

    fn freq_nibble(&self, nibble: usize) -> u32 {
        self.regs[Self::base() + nibble + 0x10] as u32 & 0x0f
    }

    fn freq(&self) -> u32 {
        let mut freq = self.freq_nibble(4);
        freq <<= 4;
        freq |= self.freq_nibble(3);
        freq <<= 4;
        freq |= self.freq_nibble(2);
        freq <<= 4;
        freq |= self.freq_nibble(1);
        freq <<= 4;
        if ID == 0 {
            freq |= self.freq_nibble(0);
        }

        freq
    }

    fn tick(&mut self, rom: &BaseRom) {
        let freq = self.freq();

        if freq == 0 {
            return;
        }

        let offset = ((self.inner.accumulator >> 15) & 0x1f) as u8;
        self.inner.accumulator += freq;
        self.inner.accumulator &= 0xfffff;

        self.inner.sample = WavData::sample(&rom, self.waveform(), offset);
    }

    fn sample(&self) -> i16 {
        self.inner.sample * self.volume() as i16
    }
}

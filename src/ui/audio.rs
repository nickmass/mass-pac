use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use direct_ring_buffer::{Consumer, Producer, create_ring_buffer};

mod pipewire;
pub use pipewire::PipewireAudio;

mod null;
pub use null::Null;

pub trait Audio {
    fn sample_rate(&self) -> u32;
    fn play(&mut self);
    fn pause(&mut self);
}

fn samples_channel(
    capacity: usize,
    buffer_len: usize,
    buffer_depth: usize,
) -> (SamplesSender, SamplesReceiver<i16>) {
    let (tx, rx) = create_ring_buffer(capacity);

    let buffer_len = Arc::new(AtomicUsize::new(buffer_len));
    let notify = SamplesNotify::new();

    let rx = SamplesReceiver {
        rx,
        last_sample: 0,
        notify: notify.clone(),
    };

    let tx = SamplesSender {
        tx,
        capacity,
        buffer_len,
        notify,
        buffer_depth,
    };

    (tx, rx)
}

struct SamplesReceiver<T> {
    rx: Consumer<T>,
    last_sample: T,
    notify: SamplesNotify,
}

impl<T> SamplesReceiver<T> {
    fn notify(&self) {
        self.notify.notify();
    }
}

impl<T: Copy> Iterator for SamplesReceiver<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if let Some(sample) = self.rx.read_element() {
            self.last_sample = sample;
            Some(sample)
        } else {
            Some(self.last_sample)
        }
    }
}

pub struct SamplesSender {
    tx: Producer<i16>,
    capacity: usize,
    buffer_len: Arc<AtomicUsize>,
    buffer_depth: usize,
    notify: SamplesNotify,
}

impl SamplesSender {
    pub fn add_samples_from_blip(&mut self, blip: &mut blip_buf::BlipBuf) {
        let avail = blip.samples_avail() as usize;

        if avail == 0 {
            return;
        }

        let written = self.tx.write_slices(
            |buf, _offset| {
                let count = blip.read_samples(buf, false) as usize;
                count
            },
            Some(avail),
        );

        if written < avail {
            blip.clear();
        }
    }

    pub fn wants_samples(&self) -> Option<usize> {
        let used = self.capacity - self.tx.available();
        let buffer_required = self.buffer_len.load(Ordering::Relaxed) * self.buffer_depth;

        (used < buffer_required).then_some(buffer_required.saturating_sub(used))
    }

    pub fn wait_for_wants_samples(&self, duration: std::time::Duration) -> Option<usize> {
        if let Some(samples) = self.wants_samples() {
            return Some(samples);
        }

        self.notify.wait_timeout(duration);
        self.wants_samples()
    }
}

#[derive(Clone)]
struct SamplesNotify {
    pair: Arc<(Mutex<()>, Condvar)>,
}

impl SamplesNotify {
    fn new() -> Self {
        Self {
            pair: Arc::new((Mutex::new(()), Condvar::new())),
        }
    }

    fn notify(&self) {
        let (_lock, cvar) = &*self.pair;
        cvar.notify_all()
    }

    fn wait_timeout(&self, duration: std::time::Duration) {
        let (lock, cvar) = &*self.pair;
        let guard = lock.lock().unwrap();
        let _ = cvar.wait_timeout(guard, duration).unwrap();
    }
}

macro_rules! impl_audio_devices {
    {$($(#[$attr:meta])? $variant:ident => $struct:ident;)*} => {
        pub enum AudioDevices {
            $(
                $(#[$attr])?
                $variant($struct)
            ),*
        }

        impl Audio for AudioDevices {
            fn sample_rate(&self) -> u32 {
                match self {
                    $($(#[$attr])? Self::$variant(a) => a.sample_rate()),*
                }
            }

            fn play(&mut self) {
                match self {
                    $($(#[$attr])? Self::$variant(a) => a.play()),*
                }
            }

            fn pause(&mut self) {
                match self {
                    $($(#[$attr])? Self::$variant(a) => a.pause()),*
                }
            }
        }

        $(
            $(#[$attr])?
            impl From<$struct> for AudioDevices {
                fn from(value: $struct) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

impl_audio_devices! {
    Pipewire => PipewireAudio;
    Null => Null;
}

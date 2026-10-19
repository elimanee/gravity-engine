//! Where generated sound goes: rodio on the desktop, the browser's WebAudio
//! in the web version (through `web/gravity_web.js`). Sounds are given as
//! interleaved samples; music (looping or not) can be faded and stopped.

/// A sound that can be faded or stopped while it plays (music).
pub struct Voice(platform::Voice);

impl Voice {
    pub fn set_volume(&self, volume: f32) {
        self.0.set_volume(volume);
    }

    pub fn stop(self) {
        self.0.stop();
    }
}

pub struct Output(platform::Output);

impl Output {
    /// The default output, or None without a sound device.
    pub fn new() -> Option<Self> {
        platform::Output::new().map(Output)
    }

    /// Play `samples` (`channels` interleaved at `rate` Hz) once.
    pub fn play(&self, samples: Vec<f32>, channels: u16, rate: u32) {
        self.0.play(samples, channels, rate);
    }

    /// Play `samples` over and over at `volume` until the voice is stopped.
    pub fn play_loop(&self, samples: Vec<f32>, channels: u16, rate: u32, volume: f32) -> Option<Voice> {
        self.0.play_voice(samples, channels, rate, volume, true).map(Voice)
    }

    /// Play `samples` once at `volume`, keeping a hold on it to stop it early.
    pub fn play_voice(&self, samples: Vec<f32>, channels: u16, rate: u32, volume: f32) -> Option<Voice> {
        self.0.play_voice(samples, channels, rate, volume, false).map(Voice)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use rodio::buffer::SamplesBuffer;
    use rodio::{OutputStream, OutputStreamHandle, Sink, Source};

    pub struct Output {
        _stream: OutputStream,
        handle: OutputStreamHandle,
    }

    impl Output {
        pub fn new() -> Option<Self> {
            let (stream, handle) = OutputStream::try_default().ok()?;
            Some(Output { _stream: stream, handle })
        }

        pub fn play(&self, samples: Vec<f32>, channels: u16, rate: u32) {
            let _ = self.handle.play_raw(SamplesBuffer::new(channels, rate, samples));
        }

        pub fn play_voice(
            &self,
            samples: Vec<f32>,
            channels: u16,
            rate: u32,
            volume: f32,
            looped: bool,
        ) -> Option<Voice> {
            let sink = Sink::try_new(&self.handle).ok()?;
            sink.set_volume(volume);
            let buffer = SamplesBuffer::new(channels, rate, samples);
            if looped {
                sink.append(buffer.repeat_infinite());
            } else {
                sink.append(buffer);
            }
            Some(Voice(sink))
        }
    }

    pub struct Voice(Sink);

    impl Voice {
        pub fn set_volume(&self, volume: f32) {
            self.0.set_volume(volume);
        }

        pub fn stop(self) {
            self.0.stop();
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    #[link(wasm_import_module = "env")]
    extern "C" {
        /// Start a buffer of `len` samples at `ptr`; returns its id.
        fn ge_audio_play(ptr: *const f32, len: u32, channels: u32, rate: u32, volume: f32, looped: u32) -> u32;
        fn ge_audio_volume(id: u32, volume: f32);
        fn ge_audio_stop(id: u32);
    }

    pub struct Output;

    impl Output {
        pub fn new() -> Option<Self> {
            Some(Output)
        }

        pub fn play(&self, samples: Vec<f32>, channels: u16, rate: u32) {
            // SAFETY: the browser copies the samples before this returns.
            unsafe { ge_audio_play(samples.as_ptr(), samples.len() as u32, channels as u32, rate, 1.0, 0) };
        }

        pub fn play_voice(
            &self,
            samples: Vec<f32>,
            channels: u16,
            rate: u32,
            volume: f32,
            looped: bool,
        ) -> Option<Voice> {
            let (ptr, len, looped) = (samples.as_ptr(), samples.len() as u32, looped as u32);
            // SAFETY: as above.
            let id = unsafe { ge_audio_play(ptr, len, channels as u32, rate, volume, looped) };
            Some(Voice(id))
        }
    }

    pub struct Voice(u32);

    impl Voice {
        pub fn set_volume(&self, volume: f32) {
            // SAFETY: plain values; unknown ids are ignored.
            unsafe { ge_audio_volume(self.0, volume) };
        }

        pub fn stop(self) {
            // SAFETY: as above.
            unsafe { ge_audio_stop(self.0) };
        }
    }
}

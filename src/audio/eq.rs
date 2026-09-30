//! Winamp-style 10-band equalizer: a pre-amplifier and one peaking filter
//! (RBJ biquad) per band, applied to the music as it plays. The settings
//! live in [`EqParams`], shared with the audio threads through atomics.

use rodio::Source;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Band centre frequencies (Hz), as on Winamp's equalizer.
pub const FREQS: [f32; 10] = [60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 14000.0, 16000.0];
/// Gains range over ±12 dB.
pub const MAX_DB: f32 = 12.0;
const Q: f32 = 1.2;

#[derive(Default)]
pub struct EqParams {
    enabled: AtomicBool,
    /// Preamp then the ten bands, in dB (f32 bits).
    gains: [AtomicU32; 11],
    version: AtomicU32,
}

impl EqParams {
    /// Update the settings; the filters pick them up on their next block.
    pub fn set(&self, enabled: bool, preamp: f32, bands: &[f32; 10]) {
        let mut changed = self.enabled.swap(enabled, Ordering::Relaxed) != enabled;
        for (slot, g) in self.gains.iter().zip(std::iter::once(&preamp).chain(bands)) {
            let bits = g.clamp(-MAX_DB, MAX_DB).to_bits();
            changed |= slot.swap(bits, Ordering::Relaxed) != bits;
        }
        if changed {
            self.version.fetch_add(1, Ordering::Release);
        }
    }

    fn snapshot(&self) -> (bool, f32, [f32; 10]) {
        let g = |i: usize| f32::from_bits(self.gains[i].load(Ordering::Relaxed));
        (self.enabled.load(Ordering::Relaxed), g(0), std::array::from_fn(|i| g(i + 1)))
    }
}

#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Biquad {
    /// Peaking filter of `gain_db` at `freq`, for `rate` Hz audio.
    fn peaking(freq: f32, gain_db: f32, rate: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = std::f32::consts::TAU * freq / rate;
        let alpha = w0.sin() / (2.0 * Q);
        let cos = w0.cos();
        let a0 = 1.0 + alpha / a;
        Biquad {
            b0: (1.0 + alpha * a) / a0,
            b1: -2.0 * cos / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }
}

/// Filter memory (transposed direct form II).
#[derive(Clone, Copy, Default)]
struct State {
    z1: f32,
    z2: f32,
}

impl State {
    fn run(&mut self, f: &Biquad, x: f32) -> f32 {
        let y = f.b0 * x + self.z1;
        self.z1 = f.b1 * x - f.a1 * y + self.z2;
        self.z2 = f.b2 * x - f.a2 * y;
        y
    }
}

/// The filters for one stream of audio.
pub struct Equalizer {
    params: Arc<EqParams>,
    version: u32,
    rate: u32,
    on: bool,
    preamp: f32,
    /// Bands that do something (gain ≠ 0 and below Nyquist).
    filters: Vec<Biquad>,
    /// Per channel, per filter.
    state: Vec<Vec<State>>,
}

impl Equalizer {
    pub fn new(params: Arc<EqParams>) -> Self {
        Equalizer { params, version: u32::MAX, rate: 0, on: false, preamp: 1.0, filters: vec![], state: vec![] }
    }

    /// Re-read the settings when they (or the format) changed.
    fn refresh(&mut self, channels: usize, rate: u32) {
        let version = self.params.version.load(Ordering::Acquire);
        if version == self.version && rate == self.rate && self.state.len() == channels {
            return;
        }
        let (on, preamp, bands) = self.params.snapshot();
        self.version = version;
        self.rate = rate;
        self.on = on;
        self.preamp = 10f32.powf(preamp / 20.0);
        let nyquist = rate as f32 / 2.0;
        self.filters = FREQS
            .iter()
            .zip(bands)
            .filter(|(f, g)| g.abs() > 0.05 && **f < nyquist * 0.95)
            .map(|(f, g)| Biquad::peaking(*f, g, rate as f32))
            .collect();
        // Keep the memory when only the gains moved, so changes do not click.
        let n = self.filters.len();
        self.state.resize(channels, vec![]);
        for s in &mut self.state {
            s.resize(n, State::default());
        }
    }

    /// Filter one sample of channel `ch`.
    #[inline]
    fn sample(&mut self, x: f32, ch: usize) -> f32 {
        if !self.on {
            return x;
        }
        let mut y = x * self.preamp;
        let state = &mut self.state[ch];
        for (s, f) in state.iter_mut().zip(&self.filters) {
            y = s.run(f, y);
        }
        y.clamp(-1.0, 1.0)
    }

    /// Filter interleaved samples in place.
    pub fn process(&mut self, buf: &mut [f32], channels: usize, rate: u32) {
        let channels = channels.max(1);
        self.refresh(channels, rate);
        if !self.on {
            return;
        }
        for frame in buf.chunks_mut(channels) {
            for (ch, s) in frame.iter_mut().enumerate() {
                *s = self.sample(*s, ch);
            }
        }
    }
}

/// A rodio source played through an [`Equalizer`].
pub struct EqSource<S> {
    inner: S,
    eq: Equalizer,
    /// Channel of the next sample.
    ch: usize,
    /// Samples until the settings are checked again.
    countdown: usize,
}

impl<S: Source<Item = f32>> EqSource<S> {
    pub fn new(inner: S, params: Arc<EqParams>) -> Self {
        EqSource { inner, eq: Equalizer::new(params), ch: 0, countdown: 0 }
    }
}

impl<S: Source<Item = f32>> Iterator for EqSource<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let x = self.inner.next()?;
        let channels = self.inner.channels().max(1) as usize;
        if self.countdown == 0 && self.ch == 0 {
            self.eq.refresh(channels, self.inner.sample_rate());
            self.countdown = 1024;
        }
        self.countdown = self.countdown.saturating_sub(1);
        let ch = self.ch.min(channels - 1);
        let y = self.eq.sample(x, ch);
        self.ch = (self.ch + 1) % channels;
        Some(y)
    }
}

impl<S: Source<Item = f32>> Source for EqSource<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.ch = 0;
        self.inner.try_seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RMS of a sine at `freq` after the equalizer.
    fn response(params: &Arc<EqParams>, freq: f32) -> f32 {
        let rate = 44_100;
        let mut eq = Equalizer::new(params.clone());
        let mut buf: Vec<f32> =
            (0..rate / 2).map(|i| 0.25 * (std::f32::consts::TAU * freq * i as f32 / rate as f32).sin()).collect();
        eq.process(&mut buf, 1, rate);
        let tail = &buf[buf.len() / 2..];
        (tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32).sqrt()
    }

    #[test]
    fn bands_boost_and_cut_their_frequencies() {
        let flat = 0.25 / 2f32.sqrt();
        let p = Arc::new(EqParams::default());
        let mut bands = [0.0; 10];
        bands[0] = 12.0; // 60 Hz up
        bands[7] = -12.0; // 12 kHz down
        p.set(true, 0.0, &bands);
        let db = |r: f32| 20.0 * (r / flat).log10();
        assert!((db(response(&p, 60.0)) - 12.0).abs() < 1.0, "60 Hz boosted by ~12 dB");
        assert!((db(response(&p, 12_000.0)) + 12.0).abs() < 1.5, "12 kHz cut by ~12 dB");
        assert!(db(response(&p, 1000.0)).abs() < 1.0, "1 kHz untouched");
        // Switched off: bit-exact pass-through.
        p.set(false, 0.0, &bands);
        assert!((response(&p, 60.0) - flat).abs() < 1e-3);
    }

    #[test]
    fn preamp_scales_and_output_stays_in_range() {
        let p = Arc::new(EqParams::default());
        p.set(true, 12.0, &[12.0; 10]);
        let mut eq = Equalizer::new(p);
        let mut buf = vec![0.9f32, -0.9, 0.9, -0.9];
        eq.process(&mut buf, 2, 44_100);
        assert!(buf.iter().all(|x| x.abs() <= 1.0));
    }
}

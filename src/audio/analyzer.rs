//! Audio analysis for the visualizer: a shared tap that playback threads feed
//! with the samples they actually send to the sound card, and an analyzer that
//! turns the latest samples into smoothed frequency bands, a waveform and beats.

use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// FFT window length (samples).
pub const FFT_SIZE: usize = 2048;
/// Number of logarithmic frequency bands produced by the analyzer.
pub const BANDS: usize = 48;
/// Number of waveform points produced by the analyzer.
pub const WAVE_POINTS: usize = 256;
/// Samples older than this are considered silence (paused / stopped playback).
const STALE_MS: u64 = 150;

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Ring of the most recent mono samples, written by the audio threads.
pub struct Tap {
    samples: Mutex<VecDeque<f32>>,
    sample_rate: AtomicU32,
    last_push_ms: AtomicU64,
}

impl Default for Tap {
    fn default() -> Self {
        Tap {
            samples: Mutex::new(VecDeque::with_capacity(FFT_SIZE * 2)),
            sample_rate: AtomicU32::new(48_000),
            last_push_ms: AtomicU64::new(0),
        }
    }
}

impl Tap {
    /// Append interleaved samples (mixed down to mono). Never blocks: if the
    /// buffer is busy the chunk is simply dropped, which is fine for a visualizer
    /// and keeps real-time audio callbacks safe.
    pub fn push_interleaved(&self, data: &[f32], channels: usize, sample_rate: u32) {
        let channels = channels.max(1);
        let Ok(mut buf) = self.samples.try_lock() else { return };
        for frame in data.chunks(channels) {
            buf.push_back(frame.iter().sum::<f32>() / frame.len() as f32);
        }
        let excess = buf.len().saturating_sub(FFT_SIZE * 2);
        buf.drain(..excess);
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
        self.last_push_ms.store(now_ms(), Ordering::Relaxed);
    }

    /// Copy the most recent `out.len()` samples into `out` (zero-padded at the
    /// front). Returns the sample rate, or `None` when the audio is silent/stale.
    fn latest(&self, out: &mut [f32]) -> Option<u32> {
        if now_ms().saturating_sub(self.last_push_ms.load(Ordering::Relaxed)) > STALE_MS {
            return None;
        }
        let buf = self.samples.lock().ok()?;
        out.fill(0.0);
        let n = buf.len().min(out.len());
        let start = out.len() - n;
        for (dst, src) in out[start..].iter_mut().zip(buf.iter().skip(buf.len() - n)) {
            *dst = *src;
        }
        Some(self.sample_rate.load(Ordering::Relaxed))
    }
}

/// In-place iterative radix-2 FFT. `re.len()` must be a power of two.
pub fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two() && im.len() == n);
    // Bit reversal.
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
}

/// Smoothed analysis results, updated once per frame.
pub struct Analyzer {
    /// Band levels in 0‥1 (log-spaced 30 Hz – 16 kHz).
    pub bands: [f32; BANDS],
    /// Falling peak markers for each band.
    pub peaks: [f32; BANDS],
    /// Waveform in -1‥1.
    pub wave: [f32; WAVE_POINTS],
    /// Overall loudness 0‥1.
    pub level: f32,
    /// Low-frequency energy 0‥1.
    pub bass: f32,
    /// Decays from 1 to 0 after each detected beat.
    pub beat: f32,
    /// True on the frame a beat was detected.
    pub beat_now: bool,
    /// Whether audio was flowing on the last update.
    pub active: bool,
    window: Vec<f32>,
    samples: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    bass_avg: f32,
    since_beat: f32,
}

impl Default for Analyzer {
    fn default() -> Self {
        let window = (0..FFT_SIZE).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos()).collect();
        Analyzer {
            bands: [0.0; BANDS],
            peaks: [0.0; BANDS],
            wave: [0.0; WAVE_POINTS],
            level: 0.0,
            bass: 0.0,
            beat: 0.0,
            beat_now: false,
            active: false,
            window,
            samples: vec![0.0; FFT_SIZE],
            re: vec![0.0; FFT_SIZE],
            im: vec![0.0; FFT_SIZE],
            bass_avg: 0.0,
            since_beat: 1.0,
        }
    }
}

/// Edges of the log-spaced bands in Hz.
fn band_edges() -> [f32; BANDS + 1] {
    let (lo, hi) = (30.0f32, 16_000.0f32);
    let mut e = [0.0; BANDS + 1];
    for (i, v) in e.iter_mut().enumerate() {
        *v = lo * (hi / lo).powf(i as f32 / BANDS as f32);
    }
    e
}

impl Analyzer {
    /// Analyse the latest samples of `tap`. `gain` scales the sensitivity.
    pub fn update(&mut self, tap: &Tap, dt: f32, gain: f32) {
        let rate = tap.latest(&mut self.samples);
        self.active = rate.is_some();
        let mut target = [0.0f32; BANDS];
        let mut level = 0.0;
        let mut bass = 0.0;

        if let Some(rate) = rate {
            // Waveform: the most recent WAVE_POINTS samples, lightly decimated.
            let step = 2;
            let start = FFT_SIZE - WAVE_POINTS * step;
            for (i, w) in self.wave.iter_mut().enumerate() {
                *w = (self.samples[start + i * step] * gain).clamp(-1.0, 1.0);
            }
            level = (self.samples.iter().map(|s| s * s).sum::<f32>() / FFT_SIZE as f32).sqrt() * gain * 2.5;

            for i in 0..FFT_SIZE {
                self.re[i] = self.samples[i] * self.window[i];
                self.im[i] = 0.0;
            }
            fft(&mut self.re, &mut self.im);
            let bin_hz = rate as f32 / FFT_SIZE as f32;
            let edges = band_edges();
            for (b, t) in target.iter_mut().enumerate() {
                let lo = ((edges[b] / bin_hz) as usize).max(1);
                let hi = ((edges[b + 1] / bin_hz) as usize).max(lo + 1).min(FFT_SIZE / 2);
                let mut peak = 0.0f32;
                for k in lo..hi {
                    peak = peak.max((self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt());
                }
                // Normalise: a full-scale sine gives ~FFT_SIZE/4 with a Hann window.
                let mag = peak / (FFT_SIZE as f32 / 4.0) * gain;
                // Tilt up the highs, which naturally carry less energy.
                let tilt = 1.0 + b as f32 / BANDS as f32 * 1.5;
                let db = 20.0 * (mag * tilt).max(1e-6).log10();
                *t = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            }
            bass = target[..BANDS / 8].iter().sum::<f32>() / (BANDS / 8) as f32;
        } else {
            for w in self.wave.iter_mut() {
                *w *= 0.8;
            }
        }

        // Fast attack, slow release.
        let rise = 1.0 - (-dt * 30.0).exp();
        let fall = 1.0 - (-dt * 6.0).exp();
        for ((band, peak), &goal) in self.bands.iter_mut().zip(self.peaks.iter_mut()).zip(target.iter()) {
            let k = if goal > *band { rise } else { fall };
            *band += (goal - *band) * k;
            *peak = if *band >= *peak { *band } else { (*peak - dt * 0.5).max(0.0) };
        }
        self.level += (level.min(1.0) - self.level) * if level > self.level { rise } else { fall };

        // Beat: bass well above its recent average, with a short cooldown.
        self.bass += (bass - self.bass) * rise;
        self.bass_avg += (self.bass - self.bass_avg) * (1.0 - (-dt * 1.5).exp());
        self.since_beat += dt;
        self.beat_now = self.active && self.bass > 0.35 && self.bass > self.bass_avg * 1.25 && self.since_beat > 0.25;
        if self.beat_now {
            self.since_beat = 0.0;
            self.beat = 1.0;
        }
        self.beat = (self.beat - dt * 3.0).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_finds_a_pure_tone() {
        let n = 1024;
        let mut re: Vec<f32> = (0..n).map(|i| (2.0 * PI * 64.0 * i as f32 / n as f32).sin()).collect();
        let mut im = vec![0.0; n];
        fft(&mut re, &mut im);
        let mags: Vec<f32> = (0..n / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt()).collect();
        let peak = mags.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert_eq!(peak, 64);
    }

    fn feed_tone(tap: &Tap, hz: f32, rate: u32) {
        let samples: Vec<f32> =
            (0..FFT_SIZE * 2).map(|i| (2.0 * PI * hz * i as f32 / rate as f32).sin() * 0.5).collect();
        tap.push_interleaved(&samples, 1, rate);
    }

    #[test]
    fn analyzer_puts_energy_in_the_right_band() {
        let tap = Tap::default();
        let mut a = Analyzer::default();
        for hz in [80.0f32, 5000.0] {
            feed_tone(&tap, hz, 48_000);
            for _ in 0..30 {
                a.update(&tap, 1.0 / 60.0, 1.0);
            }
            let loudest = a.bands.iter().enumerate().max_by(|x, y| x.1.total_cmp(y.1)).unwrap().0;
            let edges = band_edges();
            assert!(edges[loudest] <= hz * 1.2 && edges[loudest + 1] >= hz * 0.8, "{hz} Hz landed in band {loudest}");
            assert!(a.active && a.level > 0.1);
        }
    }

    #[test]
    fn waveform_follows_the_signal() {
        let tap = Tap::default();
        let mut a = Analyzer::default();
        feed_tone(&tap, 220.0, 48_000);
        a.update(&tap, 1.0 / 60.0, 1.0);
        let peak = a.wave.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.3, "waveform peak {peak}");
    }

    #[test]
    fn stereo_is_mixed_down_and_silence_decays() {
        let tap = Tap::default();
        tap.push_interleaved(&[1.0, -1.0, 0.5, 0.5], 2, 44_100);
        let mut out = [9.0f32; 4];
        assert_eq!(tap.latest(&mut out), Some(44_100));
        assert_eq!(out, [0.0, 0.0, 0.0, 0.5]);

        let empty = Tap::default();
        let mut a = Analyzer { bands: [1.0; BANDS], ..Default::default() };
        for _ in 0..120 {
            a.update(&empty, 1.0 / 60.0, 1.0);
        }
        assert!(!a.active);
        assert!(a.bands.iter().all(|b| *b < 0.01));
    }
}

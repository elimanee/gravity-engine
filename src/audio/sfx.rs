//! Procedural sound effects: every sound is synthesised on the fly (a few
//! damped partials and filtered noise), so there are no sample files, and
//! each hit sounds a little different.

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle};

const RATE: u32 = 44_100;
/// Sounds started in the last `WINDOW` seconds are counted against `MAX_VOICES`.
const WINDOW: f64 = 0.12;
const MAX_VOICES: usize = 6;

/// What made the sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sound {
    /// Two objects hitting. `size` is the larger one's size (px), `hard`
    /// 0 (soft, dull) ‥ 1 (bouncy, ringing).
    Hit { size: f32, hard: f32 },
    /// Something breaking.
    Shatter,
    /// Bomb tool.
    Boom,
    /// A shape spawned.
    Pop,
    /// A link or glue joint made.
    Snap,
    /// Grains poured.
    Pour,
    /// The knife slicing something.
    Slice,
    /// Something catching fire.
    Ignite,
    /// Wood crackling in a fire.
    Crackle,
    /// A fire put out by water.
    Hiss,
    /// A thruster's roar (played over and over while it works).
    Thrust,
    /// A cannon shot.
    Shot,
    /// Thunder after a lightning strike.
    Thunder,
    /// Challenge solved.
    Win,
}

pub struct Sfx {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    started: Vec<f64>,
    seed: u32,
}

impl Sfx {
    pub fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Sfx { _stream: stream, handle, started: vec![], seed: 0x9e37_79b9 })
    }

    /// Play `sound` at `volume` (0‥1), panned by `pan` (-1 left ‥ 1 right),
    /// with its pitch scaled by `pitch` (slow motion plays lower).
    pub fn play(&mut self, now: f64, sound: Sound, volume: f32, pan: f32, pitch: f32) {
        self.started.retain(|t| now - *t < WINDOW);
        let urgent = matches!(sound, Sound::Win | Sound::Boom | Sound::Shatter | Sound::Thunder);
        if volume < 0.01 || (!urgent && self.started.len() >= MAX_VOICES) {
            return;
        }
        self.started.push(now);
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let mono = synth(sound, pitch, self.seed);
        let buf = SamplesBuffer::new(2, RATE, stereo(&mono, volume, pan));
        let _ = self.handle.play_raw(buf);
    }
}

/// Equal-power pan, gain and a soft limiter.
fn stereo(mono: &[f32], volume: f32, pan: f32) -> Vec<f32> {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    let (l, r) = (a.cos() * volume, a.sin() * volume);
    mono.iter().flat_map(|&s| [(s * l).tanh(), (s * r).tanh()]).collect()
}

/// Tiny deterministic noise source.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() * 0.5 + 0.5) * (hi - lo)
    }
}

/// A damped sine: frequency (Hz), amplitude, decay time (s), start (s).
struct Partial(f32, f32, f32, f32);

fn add_partials(out: &mut [f32], partials: &[Partial]) {
    let dt = 1.0 / RATE as f32;
    for &Partial(f, amp, decay, start) in partials {
        let first = (start * RATE as f32) as usize;
        let len = ((decay * 6.0) * RATE as f32) as usize;
        for (k, s) in out.iter_mut().skip(first).take(len).enumerate() {
            let t = k as f32 * dt;
            *s += (std::f32::consts::TAU * f * t).sin() * amp * (-t / decay).exp();
        }
    }
}

/// Noise through a one-pole low-pass whose cutoff (Hz) follows `cutoff(t)`,
/// shaped by `env(t)`.
fn add_noise(out: &mut [f32], noise: &mut Noise, cutoff: impl Fn(f32) -> f32, env: impl Fn(f32) -> f32) {
    let dt = 1.0 / RATE as f32;
    let mut y = 0.0;
    for (k, s) in out.iter_mut().enumerate() {
        let t = k as f32 * dt;
        let a = 1.0 - (-std::f32::consts::TAU * cutoff(t) * dt).exp();
        y += (noise.next() - y) * a;
        *s += y * env(t);
    }
}

pub fn synth(sound: Sound, pitch: f32, seed: u32) -> Vec<f32> {
    let mut n = Noise(seed | 1);
    let secs = |s: f32| vec![0.0; (s * RATE as f32) as usize];
    let p = pitch.clamp(0.2, 2.0);
    match sound {
        Sound::Hit { size, hard } => {
            let hard = hard.clamp(0.0, 1.0);
            // Bigger objects ring lower; each hit is detuned a little.
            let f0 = (5200.0 / size.max(8.0).sqrt()).clamp(90.0, 1400.0) * n.range(0.9, 1.1) * p;
            let ring = 0.03 + hard * 0.16;
            let mut out = secs(ring * 5.0 + 0.03);
            add_partials(
                &mut out,
                &[
                    Partial(f0, 0.55, ring, 0.0),
                    Partial(f0 * 2.76, 0.3 * hard + 0.05, ring * 0.5, 0.0),
                    Partial(f0 * 5.40, 0.18 * hard, ring * 0.3, 0.0),
                ],
            );
            // The knock itself: a burst of darker noise for soft hits.
            let click = 900.0 + hard * 5000.0;
            add_noise(&mut out, &mut n, |_| click * p, |t| 0.9 * (-t / 0.006).exp());
            out
        }
        Sound::Shatter => {
            let mut out = secs(0.9);
            add_noise(&mut out, &mut n, |_| 7000.0, |t| 0.7 * (-t / 0.04).exp());
            let mut partials = vec![];
            for _ in 0..14 {
                let f = n.range(2200.0, 7500.0) * p;
                partials.push(Partial(f, n.range(0.05, 0.14), n.range(0.05, 0.18), n.range(0.0, 0.25)));
            }
            add_partials(&mut out, &partials);
            out
        }
        Sound::Boom => {
            let mut out = secs(1.4);
            add_noise(
                &mut out,
                &mut n,
                |t| (1400.0 * (-t / 0.12).exp() + 90.0) * p,
                |t| 1.8 * (t / 0.004).min(1.0) * (-t / 0.35).exp(),
            );
            // A falling sub thump.
            let dt = 1.0 / RATE as f32;
            let mut phase = 0.0;
            for (k, s) in out.iter_mut().enumerate() {
                let t = k as f32 * dt;
                phase += std::f32::consts::TAU * (35.0 + 70.0 * (-t / 0.08).exp()) * p * dt;
                *s += phase.sin() * 0.9 * (-t / 0.3).exp();
            }
            out
        }
        Sound::Pop => {
            let mut out = secs(0.09);
            let dt = 1.0 / RATE as f32;
            let base = n.range(380.0, 520.0) * p;
            let mut phase = 0.0;
            for (k, s) in out.iter_mut().enumerate() {
                let t = k as f32 * dt;
                phase += std::f32::consts::TAU * base * (1.0 + t * 25.0) * dt;
                *s = phase.sin() * 0.35 * (t / 0.002).min(1.0) * (-t / 0.025).exp();
            }
            out
        }
        Sound::Snap => {
            let mut out = secs(0.12);
            add_partials(&mut out, &[Partial(1650.0 * p, 0.25, 0.012, 0.0), Partial(2480.0 * p, 0.15, 0.02, 0.018)]);
            add_noise(&mut out, &mut n, |_| 4000.0, |t| 0.4 * (-t / 0.003).exp());
            out
        }
        Sound::Pour => {
            let mut out = secs(0.18);
            add_noise(
                &mut out,
                &mut n,
                |_| 2500.0 * p,
                |t| 0.18 * (t / 0.03).min(1.0) * ((0.18 - t) / 0.06).clamp(0.0, 1.0),
            );
            out
        }
        Sound::Slice => {
            // A quick swish: noise sweeping down from very high.
            let mut out = secs(0.22);
            add_noise(
                &mut out,
                &mut n,
                |t| (9000.0 * (-t / 0.05).exp() + 900.0) * p,
                |t| 0.9 * (t / 0.01).min(1.0) * (-t / 0.05).exp(),
            );
            out
        }
        Sound::Ignite => {
            // A soft whoosh: low noise swelling then dying away.
            let mut out = secs(0.5);
            add_noise(
                &mut out,
                &mut n,
                |t| (600.0 + 2200.0 * (t / 0.12).min(1.0)) * p,
                |t| 0.9 * (t / 0.08).min(1.0) * (-t / 0.12).exp(),
            );
            out
        }
        Sound::Crackle => {
            // A few sharp pops at random moments.
            let mut out = secs(0.3);
            let pops = 3 + (n.next().abs() * 4.0) as usize;
            for _ in 0..pops {
                let at = n.range(0.0, 0.2);
                let f = n.range(1800.0, 4200.0) * p;
                add_partials(&mut out, &[Partial(f, n.range(0.12, 0.3), 0.004, at)]);
            }
            add_noise(&mut out, &mut n, |_| 3000.0, |t| 0.25 * (-t / 0.01).exp());
            out
        }
        Sound::Hiss => {
            let mut out = secs(0.7);
            add_noise(&mut out, &mut n, |_| 9000.0 * p, |t| 0.55 * (t / 0.02).min(1.0) * (-t / 0.14).exp());
            out
        }
        Sound::Thrust => {
            let mut out = secs(0.3);
            add_noise(
                &mut out,
                &mut n,
                |_| 700.0 * p,
                |t| 0.6 * (t / 0.03).min(1.0) * ((0.3 - t) / 0.08).clamp(0.0, 1.0),
            );
            out
        }
        Sound::Shot => {
            let mut out = secs(0.45);
            add_noise(&mut out, &mut n, |t| (3000.0 * (-t / 0.03).exp() + 200.0) * p, |t| 1.1 * (-t / 0.06).exp());
            let dt = 1.0 / RATE as f32;
            let mut phase = 0.0;
            for (k, s) in out.iter_mut().enumerate() {
                let t = k as f32 * dt;
                phase += std::f32::consts::TAU * (60.0 + 120.0 * (-t / 0.04).exp()) * p * dt;
                *s += phase.sin() * 0.7 * (-t / 0.08).exp();
            }
            out
        }
        Sound::Thunder => {
            // A sharp crack, then a long low rumble rolling in waves.
            let mut out = secs(1.9);
            add_noise(
                &mut out,
                &mut n,
                |t| (5000.0 * (-t / 0.03).exp() + 120.0) * p,
                |t| {
                    let roll = 0.75 + 0.25 * (t * 9.0).sin();
                    1.3 * (t / 0.003).min(1.0)
                        * ((-t / 0.05).exp() * 0.8 + 0.6 * (-t / 0.55).exp() * roll)
                        * ((1.9 - t) / 0.4).clamp(0.0, 1.0)
                },
            );
            out
        }
        Sound::Win => {
            let mut out = secs(1.6);
            // C major arpeggio of soft bells.
            for (i, semis) in [0.0f32, 4.0, 7.0, 12.0].iter().enumerate() {
                let f = 523.25 * 2f32.powf(semis / 12.0) * p;
                let at = i as f32 * 0.09;
                add_partials(
                    &mut out,
                    &[Partial(f, 0.28, 0.45, at), Partial(f * 2.0, 0.09, 0.25, at), Partial(f * 3.0, 0.04, 0.12, at)],
                );
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Sound; 15] = [
        Sound::Hit { size: 80.0, hard: 0.2 },
        Sound::Hit { size: 20.0, hard: 1.0 },
        Sound::Shatter,
        Sound::Boom,
        Sound::Pop,
        Sound::Snap,
        Sound::Pour,
        Sound::Slice,
        Sound::Ignite,
        Sound::Crackle,
        Sound::Hiss,
        Sound::Thrust,
        Sound::Shot,
        Sound::Thunder,
        Sound::Win,
    ];

    #[test]
    fn sounds_are_short_audible_and_finite() {
        for s in ALL {
            let v = synth(s, 1.0, 7);
            assert!(!v.is_empty() && v.len() < RATE as usize * 2, "{s:?} length {}", v.len());
            assert!(v.iter().all(|x| x.is_finite()), "{s:?}");
            let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.05, "{s:?} is silent ({peak})");
            // Fades out instead of clicking off.
            let tail = v[v.len() - 32..].iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(tail < 0.05, "{s:?} ends abruptly ({tail})");
        }
    }

    #[test]
    fn stereo_output_is_limited() {
        let v = stereo(&synth(Sound::Boom, 1.0, 3), 1.0, -1.0);
        assert!(v.iter().all(|x| x.abs() <= 1.0));
        // Hard left: the right channel is (nearly) silent.
        let right = v.iter().skip(1).step_by(2).fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(right < 1e-3);
    }
}

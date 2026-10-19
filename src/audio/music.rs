//! Sonic's music, made here: little chiptune songs (two pulse voices, a
//! triangle bass and noise drums, like the 8-bit consoles) composed from a
//! seed and a style for each zone, the boss, invincibility, Super Sonic and
//! the special stages, and short jingles. Nothing comes from the games.
//!
//! Songs are 16 bars (an A and a B section over a chord progression) and
//! loop seamlessly; jingles play once.

pub const RATE: u32 = 22_050;

/// A looping song.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Song {
    Meadow,
    Twilight,
    Snowfield,
    Lagoon,
    Storm,
    Harbour,
    Boss,
    Invincible,
    Super,
    Special,
}

/// A tune played once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Jingle {
    ActClear,
    OneUp,
    GameOver,
    Emerald,
    /// The countdown when Sonic is running out of air (it speeds up).
    Drowning,
}

impl Jingle {
    /// Whether the song stops for it (rather than coming back after it).
    pub fn ends_song(self) -> bool {
        matches!(self, Jingle::ActClear | Jingle::GameOver)
    }
}

const MAJOR: &[i32] = &[0, 2, 4, 5, 7, 9, 11];
const MINOR: &[i32] = &[0, 2, 3, 5, 7, 8, 10];
const DORIAN: &[i32] = &[0, 2, 3, 5, 7, 9, 10];
const MIXOLYDIAN: &[i32] = &[0, 2, 4, 5, 7, 9, 10];

#[derive(Clone, Copy, PartialEq)]
enum Bass {
    /// Eighth notes on the root.
    Pulse,
    /// Root and octave, alternating eighths.
    Octaves,
    /// Quarter notes walking up the chord.
    Walking,
}

#[derive(Clone, Copy, PartialEq)]
enum Drums {
    Rock,
    Driving,
    Shuffle,
    Soft,
}

struct Style {
    seed: u64,
    bpm: f32,
    /// MIDI note of the key's tonic (around middle C).
    root: i32,
    scale: &'static [i32],
    /// Scale degrees (0 = tonic) of the chord of each bar of a section.
    chords: [usize; 8],
    /// Chords of the B section.
    chords_b: [usize; 8],
    duty: f32,
    bass: Bass,
    drums: Drums,
    /// Second voice: an arpeggio (else held chord tones).
    arp: bool,
    /// Lead echo (0: none).
    echo: f32,
}

fn style(song: Song) -> Style {
    match song {
        Song::Meadow => Style {
            seed: 11,
            bpm: 144.0,
            root: 62,
            scale: MAJOR,
            chords: [0, 0, 3, 4, 5, 3, 1, 4],
            chords_b: [3, 4, 2, 5, 3, 4, 0, 4],
            duty: 0.25,
            bass: Bass::Octaves,
            drums: Drums::Rock,
            arp: true,
            echo: 0.18,
        },
        Song::Twilight => Style {
            seed: 23,
            bpm: 108.0,
            root: 57,
            scale: MINOR,
            chords: [0, 5, 2, 6, 0, 5, 3, 4],
            chords_b: [5, 6, 0, 0, 3, 2, 3, 4],
            duty: 0.5,
            bass: Bass::Walking,
            drums: Drums::Soft,
            arp: false,
            echo: 0.32,
        },
        Song::Snowfield => Style {
            seed: 37,
            bpm: 120.0,
            root: 65,
            scale: MAJOR,
            chords: [0, 4, 5, 3, 0, 4, 3, 4],
            chords_b: [5, 2, 3, 0, 5, 2, 3, 4],
            duty: 0.125,
            bass: Bass::Pulse,
            drums: Drums::Soft,
            arp: true,
            echo: 0.3,
        },
        Song::Lagoon => Style {
            seed: 41,
            bpm: 100.0,
            root: 60,
            scale: DORIAN,
            chords: [0, 3, 0, 3, 6, 3, 1, 4],
            chords_b: [3, 3, 0, 0, 6, 6, 1, 4],
            duty: 0.5,
            bass: Bass::Walking,
            drums: Drums::Shuffle,
            arp: false,
            echo: 0.25,
        },
        Song::Storm => Style {
            seed: 53,
            bpm: 164.0,
            root: 59,
            scale: MINOR,
            chords: [0, 0, 5, 6, 0, 0, 3, 4],
            chords_b: [5, 6, 0, 0, 5, 6, 3, 4],
            duty: 0.25,
            bass: Bass::Pulse,
            drums: Drums::Driving,
            arp: true,
            echo: 0.12,
        },
        Song::Harbour => Style {
            seed: 67,
            bpm: 132.0,
            root: 63,
            scale: MIXOLYDIAN,
            chords: [0, 6, 3, 0, 0, 6, 3, 4],
            chords_b: [3, 3, 0, 0, 5, 6, 1, 4],
            duty: 0.25,
            bass: Bass::Octaves,
            drums: Drums::Shuffle,
            arp: true,
            echo: 0.2,
        },
        Song::Boss => Style {
            seed: 71,
            bpm: 172.0,
            root: 55,
            scale: MINOR,
            chords: [0, 0, 1, 1, 0, 0, 5, 4],
            chords_b: [5, 5, 6, 6, 3, 3, 4, 4],
            duty: 0.5,
            bass: Bass::Pulse,
            drums: Drums::Driving,
            arp: true,
            echo: 0.0,
        },
        Song::Invincible => Style {
            seed: 83,
            bpm: 184.0,
            root: 64,
            scale: MAJOR,
            chords: [0, 4, 5, 4, 3, 0, 1, 4],
            chords_b: [3, 4, 0, 5, 3, 4, 0, 4],
            duty: 0.25,
            bass: Bass::Octaves,
            drums: Drums::Driving,
            arp: true,
            echo: 0.0,
        },
        Song::Super => Style {
            seed: 97,
            bpm: 176.0,
            root: 66,
            scale: MIXOLYDIAN,
            chords: [0, 6, 0, 6, 3, 4, 0, 4],
            chords_b: [3, 4, 5, 6, 3, 4, 6, 4],
            duty: 0.125,
            bass: Bass::Octaves,
            drums: Drums::Rock,
            arp: true,
            echo: 0.15,
        },
        Song::Special => Style {
            seed: 101,
            bpm: 126.0,
            root: 61,
            scale: MAJOR,
            chords: [0, 2, 3, 4, 0, 2, 3, 4],
            chords_b: [5, 3, 0, 4, 5, 3, 1, 4],
            duty: 0.125,
            bass: Bass::Walking,
            drums: Drums::Soft,
            arp: true,
            echo: 0.35,
        },
    }
}

/// Small seeded random numbers, so a song is always the same.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn chance(&mut self, p: f32) -> bool {
        (self.next() % 1000) as f32 / 1000.0 < p
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Voice {
    Lead,
    Harmony,
    Bass,
}

/// A note: who plays it, when (beats), how long (beats), its MIDI pitch
/// and loudness.
#[derive(Clone, Copy)]
struct Note {
    voice: Voice,
    at: f32,
    len: f32,
    pitch: i32,
    vol: f32,
}

#[derive(Clone, Copy)]
enum Hit {
    Kick,
    Snare,
    Hat,
}

/// The pitch of scale degree `d` (any integer, octaves wrap) from `root`.
fn degree(scale: &[i32], root: i32, d: i32) -> i32 {
    let n = scale.len() as i32;
    root + scale[d.rem_euclid(n) as usize] + 12 * d.div_euclid(n)
}

/// Rhythms of a two-bar phrase: (start, length) in sixteenths.
const RHYTHMS: &[&[(u8, u8)]] = &[
    &[(0, 3), (3, 1), (4, 2), (6, 2), (8, 4), (12, 4), (16, 3), (19, 1), (20, 4), (24, 8)],
    &[(0, 2), (2, 2), (4, 4), (8, 2), (10, 2), (12, 4), (16, 6), (22, 2), (24, 4), (28, 4)],
    &[(0, 4), (4, 2), (6, 2), (8, 6), (14, 2), (16, 2), (18, 2), (20, 2), (22, 2), (24, 8)],
    &[(2, 2), (4, 2), (6, 4), (10, 2), (12, 4), (18, 2), (20, 2), (22, 2), (24, 6), (30, 2)],
    &[(0, 6), (6, 2), (8, 2), (10, 2), (12, 4), (16, 4), (20, 4), (24, 4), (28, 4)],
];

/// A motif: a rhythm and the steps (in scale degrees) between its notes.
struct Motif {
    rhythm: &'static [(u8, u8)],
    steps: Vec<i32>,
}

fn motif(rng: &mut Rng) -> Motif {
    let rhythm = RHYTHMS[rng.below(RHYTHMS.len())];
    let steps = (0..rhythm.len())
        .map(|_| match rng.below(10) {
            0..=2 => 1,
            3..=5 => -1,
            6 => 2,
            7 => -2,
            8 => 0,
            _ => 3,
        })
        .collect();
    Motif { rhythm, steps }
}

/// Play `m` over two bars from bar `bar` (chords `chords`), starting near
/// degree `from`; strong beats land on chord tones. Returns the last degree.
#[allow(clippy::too_many_arguments)]
fn realise(
    notes: &mut Vec<Note>,
    m: &Motif,
    st: &Style,
    bar: usize,
    chords: &[usize; 8],
    from: i32,
    end_on_tonic: bool,
) -> i32 {
    let mut d = from;
    let last = m.rhythm.len() - 1;
    for (k, (&(start, len), &step)) in m.rhythm.iter().zip(&m.steps).enumerate() {
        let local_bar = bar + start as usize / 16;
        let chord = chords[local_bar % 8] as i32;
        d += step;
        let strong = start % 4 == 0;
        if strong {
            // The nearest chord tone (root, third or fifth of the bar's chord).
            let tones = [chord, chord + 2, chord + 4];
            d = (-1..=2).flat_map(|oct| tones.map(|t| t + 7 * oct)).min_by_key(|t| (t - d).abs()).unwrap_or(d);
        }
        if k == last && end_on_tonic {
            d = if d > 3 { 7 } else { 0 };
        }
        // Keep the tune in its range.
        d = d.clamp(-2, 11);
        let pitch = degree(st.scale, st.root, d);
        let at = (bar * 16 + start as usize) as f32 / 4.0;
        notes.push(Note { voice: Voice::Lead, at, len: len as f32 / 4.0 * 0.92, pitch, vol: 1.0 });
    }
    d
}

/// The notes and drum hits of a 16-bar song.
fn compose(st: &Style) -> (Vec<Note>, Vec<(f32, Hit)>) {
    let mut rng = Rng(st.seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut notes = vec![];
    // Tune: A = m1 m2 m1 m3, B = m4 m5 m4 m6.
    let ms: Vec<Motif> = (0..6).map(|_| motif(&mut rng)).collect();
    let mut d = 4;
    for (section, chords) in [(0usize, &st.chords), (8, &st.chords_b)] {
        let base = if section == 0 { 0 } else { 3 };
        for (k, which) in [0, 1, 0, 2].into_iter().enumerate() {
            let bar = section + k * 2;
            d = realise(&mut notes, &ms[base + which], st, bar, chords, d, k == 3);
        }
    }
    // Harmony and bass follow the chords.
    for bar in 0..16 {
        let chord = if bar < 8 { st.chords[bar] } else { st.chords_b[bar - 8] } as i32;
        let b0 = bar as f32 * 4.0;
        let tone = |k: i32| degree(st.scale, st.root - 12, chord + k);
        if st.arp {
            let pattern = [0, 2, 4, 7, 4, 2, 0, 2];
            for s in 0..16 {
                notes.push(Note {
                    voice: Voice::Harmony,
                    at: b0 + s as f32 / 4.0,
                    len: 0.2,
                    pitch: tone(pattern[s % 8]),
                    vol: 0.7,
                });
            }
        } else {
            for half in 0..2 {
                for k in [2, 4] {
                    notes.push(Note {
                        voice: Voice::Harmony,
                        at: b0 + half as f32 * 2.0,
                        len: 1.9,
                        pitch: tone(k),
                        vol: 0.55,
                    });
                }
            }
        }
        let root = degree(st.scale, st.root - 24, chord);
        match st.bass {
            Bass::Pulse => {
                for e in 0..8 {
                    notes.push(Note { voice: Voice::Bass, at: b0 + e as f32 * 0.5, len: 0.42, pitch: root, vol: 1.0 });
                }
            }
            Bass::Octaves => {
                for e in 0..8 {
                    let pitch = if e % 2 == 0 { root } else { root + 12 };
                    notes.push(Note { voice: Voice::Bass, at: b0 + e as f32 * 0.5, len: 0.42, pitch, vol: 1.0 });
                }
            }
            Bass::Walking => {
                for (q, k) in [0, 2, 4, 5].into_iter().enumerate() {
                    let pitch = degree(st.scale, st.root - 24, chord + k);
                    notes.push(Note { voice: Voice::Bass, at: b0 + q as f32, len: 0.9, pitch, vol: 1.0 });
                }
            }
        }
    }
    // Drums, with a fill at the end of each section.
    let mut hits = vec![];
    for bar in 0..16 {
        let b0 = bar as f32 * 4.0;
        let fill = bar % 8 == 7;
        for s in 0..16 {
            let at = b0 + s as f32 / 4.0;
            if fill && s >= 12 {
                hits.push((at, Hit::Snare));
                continue;
            }
            let (kick, snare, hat) = match st.drums {
                Drums::Rock => (s == 0 || s == 8 || s == 10, s == 4 || s == 12, s % 2 == 0),
                Drums::Driving => (s % 4 == 0, s == 4 || s == 12, true),
                Drums::Shuffle => (s == 0 || s == 7 || s == 10, s == 4 || s == 12, s % 4 == 0 || s % 4 == 3),
                Drums::Soft => (s == 0 || s == 8, s == 12 && rng.chance(0.8), s % 4 == 2),
            };
            if kick {
                hits.push((at, Hit::Kick));
            }
            if snare {
                hits.push((at, Hit::Snare));
            }
            if hat {
                hits.push((at, Hit::Hat));
            }
        }
    }
    (notes, hits)
}

fn freq(pitch: i32) -> f32 {
    440.0 * 2f32.powf((pitch - 69) as f32 / 12.0)
}

/// Smooths the jump of a pulse wave at phase `t` (0‥1) moving `dt` per
/// sample (PolyBLEP), so high notes do not alias into a hiss.
fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// A band-limited pulse wave of `duty` at phase `p`.
fn pulse(p: f32, dt: f32, duty: f32) -> f32 {
    let t = p.fract();
    let naive = if t < duty { 1.0 } else { -1.0 };
    naive + blep(t, dt) - blep((t + 1.0 - duty).fract(), dt)
}

/// Add the notes and hits, `beat` seconds long, into `out` (which wraps
/// round, so a song loops without a gap).
fn render(out: &mut [f32], notes: &[Note], hits: &[(f32, Hit)], beat: f32, st_duty: f32, echo: f32, wrap: bool) {
    let n = out.len();
    let mut lead = vec![0.0f32; n];
    let rate = RATE as f32;
    let put = |buf: &mut [f32], i: usize, v: f32| {
        if wrap {
            buf[i % n] += v;
        } else if i < n {
            buf[i] += v;
        }
    };
    for note in notes {
        let start = (note.at * beat * rate) as usize;
        let dur = note.len * beat;
        let release = 0.04;
        let len = ((dur + release) * rate) as usize;
        let f = freq(note.pitch);
        let mut phase = 0.0f32;
        for k in 0..len {
            let t = k as f32 / rate;
            // Envelope: quick attack, a little decay, release after the note.
            let env = (t / 0.004).min(1.0)
                * (0.65 + 0.35 * (-t / 0.09).exp())
                * if t > dur { (1.0 - (t - dur) / release).max(0.0) } else { 1.0 };
            let (gain, value) = match note.voice {
                Voice::Lead => {
                    let vib = if t > 0.15 { (std::f32::consts::TAU * 5.5 * t).sin() * 0.004 } else { 0.0 };
                    let dt = f * (1.0 + vib) / rate;
                    phase += dt;
                    (0.16, pulse(phase, dt, st_duty))
                }
                Voice::Harmony => {
                    phase += f / rate;
                    (0.07, pulse(phase, f / rate, 0.5))
                }
                Voice::Bass => {
                    phase += f / rate;
                    // A stepped triangle, like the consoles' bass channel.
                    let tri = 4.0 * (phase.fract() - 0.5).abs() - 1.0;
                    (0.30, (tri * 8.0).round() / 8.0)
                }
            };
            let v = value * env * gain * note.vol;
            if note.voice == Voice::Lead {
                put(&mut lead, start + k, v);
            } else {
                put(out, start + k, v);
            }
        }
    }
    // The lead's echo.
    if echo > 0.0 {
        let delay = (beat * 0.75 * rate) as usize;
        let len = lead.len();
        for _ in 0..2 {
            for i in (0..len).rev() {
                let src = if i >= delay {
                    Some(i - delay)
                } else if wrap {
                    Some(i + len - delay)
                } else {
                    None
                };
                if let Some(j) = src {
                    lead[i] += lead[j] * echo * 0.5;
                }
            }
        }
    }
    for (o, l) in out.iter_mut().zip(&lead) {
        *o += l;
    }
    // Drums.
    let mut noise = 0x1234_5678u32;
    let mut white = move || {
        noise ^= noise << 13;
        noise ^= noise >> 17;
        noise ^= noise << 5;
        (noise as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    for &(at, hit) in hits {
        let start = (at * beat * rate) as usize;
        let (secs, gain) = match hit {
            Hit::Kick => (0.16, 0.42),
            Hit::Snare => (0.13, 0.2),
            Hit::Hat => (0.03, 0.035),
        };
        let mut phase = 0.0f32;
        let mut last = 0.0;
        for k in 0..(secs * rate) as usize {
            let t = k as f32 / rate;
            let v = match hit {
                Hit::Kick => {
                    phase += (45.0 + 110.0 * (-t / 0.03).exp()) / rate;
                    (std::f32::consts::TAU * phase).sin() * (-t / 0.07).exp()
                }
                Hit::Snare => {
                    phase += 190.0 / rate;
                    white() * (-t / 0.05).exp() * 0.8 + (std::f32::consts::TAU * phase).sin() * (-t / 0.03).exp() * 0.5
                }
                Hit::Hat => {
                    // High-passed noise.
                    let w = white();
                    let hp = w - last;
                    last = w;
                    hp * (-t / 0.012).exp()
                }
            };
            put(out, start + k, v * gain);
        }
    }
    // Take the edge off the top (one-pole low-pass at about 7 kHz), then a
    // gentle limiter.
    let a = 1.0 - (-std::f32::consts::TAU * 7000.0 / rate).exp();
    let mut y = 0.0;
    if wrap {
        // Start from where the end leaves the filter, so the loop is seamless.
        for &s in out.iter() {
            y += (s - y) * a;
        }
    }
    for s in out.iter_mut() {
        y += (*s - y) * a;
        *s = y;
    }
    for s in out.iter_mut() {
        *s = (*s * 1.1).tanh() * 0.8;
    }
}

/// A song, `RATE` Hz mono, made to be looped.
pub fn song(song: Song) -> Vec<f32> {
    let st = style(song);
    let (notes, hits) = compose(&st);
    let beat = 60.0 / st.bpm;
    let len = (16.0 * 4.0 * beat * RATE as f32) as usize;
    let mut out = vec![0.0; len];
    render(&mut out, &notes, &hits, beat, st.duty, st.echo, true);
    out
}

/// A jingle, `RATE` Hz mono, played once.
pub fn jingle(j: Jingle) -> Vec<f32> {
    let lead = |at: f32, len: f32, pitch: i32| Note { voice: Voice::Lead, at, len, pitch, vol: 1.0 };
    let harm = |at: f32, len: f32, pitch: i32| Note { voice: Voice::Harmony, at, len, pitch, vol: 0.8 };
    let bass = |at: f32, len: f32, pitch: i32| Note { voice: Voice::Bass, at, len, pitch, vol: 1.0 };
    let (bpm, notes, hits, beats): (f32, Vec<Note>, Vec<(f32, Hit)>, f32) = match j {
        Jingle::ActClear => {
            let mut n = vec![];
            for (k, p) in [62, 66, 69, 74, 78, 81].into_iter().enumerate() {
                n.push(lead(k as f32 * 0.25, 0.22, p));
            }
            n.extend([lead(1.5, 0.45, 79), lead(2.0, 0.45, 81), lead(2.5, 2.4, 86)]);
            n.extend([harm(1.5, 0.45, 71), harm(2.0, 0.45, 73), harm(2.5, 2.4, 78)]);
            n.extend([bass(0.0, 1.4, 38), bass(1.5, 0.9, 43), bass(2.5, 2.4, 38)]);
            let hits =
                vec![(0.0, Hit::Kick), (1.5, Hit::Snare), (2.0, Hit::Snare), (2.5, Hit::Kick), (2.5, Hit::Snare)];
            (150.0, n, hits, 5.5)
        }
        Jingle::OneUp => {
            let mut n = vec![];
            for (at, len, p) in
                [(0.0, 0.4, 72), (0.5, 0.2, 76), (0.75, 0.2, 79), (1.0, 0.4, 84), (1.5, 0.2, 79), (1.75, 1.2, 84)]
            {
                n.push(lead(at, len, p));
                n.push(harm(at, len, p - 5));
            }
            n.extend([bass(0.0, 0.9, 48), bass(1.0, 0.9, 43), bass(2.0, 0.9, 48)]);
            (168.0, n, vec![(0.0, Hit::Kick), (1.0, Hit::Kick), (1.75, Hit::Snare)], 3.2)
        }
        Jingle::GameOver => {
            let mut n = vec![];
            for (k, p) in [69, 67, 64, 62, 60, 57].into_iter().enumerate() {
                n.push(lead(k as f32 * 0.5, 0.45, p));
            }
            n.push(lead(3.0, 2.0, 57));
            n.extend([harm(3.0, 2.0, 60), harm(3.0, 2.0, 64)]);
            n.extend([bass(0.0, 1.9, 45), bass(2.0, 0.9, 41), bass(3.0, 2.0, 33)]);
            (96.0, n, vec![(3.0, Hit::Kick)], 5.5)
        }
        Jingle::Emerald => {
            let mut n = vec![];
            for (k, p) in [72, 76, 79, 83, 84, 88, 91, 95].into_iter().enumerate() {
                n.push(lead(k as f32 * 0.25, 0.4, p));
            }
            n.extend([lead(2.0, 2.4, 96), harm(2.0, 2.4, 88), harm(2.0, 2.4, 91)]);
            n.extend([bass(0.0, 1.9, 48), bass(2.0, 2.4, 36)]);
            (132.0, n, vec![(2.0, Hit::Kick), (2.0, Hit::Hat)], 5.0)
        }
        Jingle::Drowning => {
            // Twelve seconds of a rising ostinato that hurries along.
            let mut n = vec![];
            let mut at = 0.0;
            let mut k = 0;
            while at < 22.0 {
                let p = 57 + (k / 4) % 12;
                n.push(lead(at, 0.2, p + if k % 2 == 0 { 0 } else { 12 }));
                n.push(bass(at, 0.2, p - 24));
                let step = (0.5 - at / 60.0).max(0.25);
                at += step;
                k += 1;
            }
            (110.0, n, vec![], 22.0)
        }
    };
    let beat = 60.0 / bpm;
    let len = (beats * beat * RATE as f32) as usize;
    let mut out = vec![0.0; len];
    render(&mut out, &notes, &hits, beat, 0.25, 0.15, false);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Song; 10] = [
        Song::Meadow,
        Song::Twilight,
        Song::Snowfield,
        Song::Lagoon,
        Song::Storm,
        Song::Harbour,
        Song::Boss,
        Song::Invincible,
        Song::Super,
        Song::Special,
    ];

    #[test]
    fn songs_are_audible_bounded_and_always_the_same() {
        for s in ALL {
            let a = song(s);
            let secs = a.len() as f32 / RATE as f32;
            assert!((15.0..60.0).contains(&secs), "{s:?} lasts {secs} s");
            assert!(a.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
            let rms = (a.iter().map(|v| v * v).sum::<f32>() / a.len() as f32).sqrt();
            assert!(rms > 0.03, "{s:?} is too quiet: {rms}");
            assert_eq!(a, song(s), "{s:?} changes between renders");
        }
        assert_ne!(song(Song::Meadow), song(Song::Storm));
    }

    #[test]
    fn songs_loop_without_a_click() {
        for s in ALL {
            let a = song(s);
            let jump = (a[0] - a[a.len() - 1]).abs();
            assert!(jump < 0.3, "{s:?} jumps {jump} where it loops");
        }
    }

    #[test]
    fn jingles_end_and_say_whether_the_song_stops() {
        for j in [Jingle::ActClear, Jingle::OneUp, Jingle::GameOver, Jingle::Emerald, Jingle::Drowning] {
            let a = jingle(j);
            assert!(a.iter().any(|v| v.abs() > 0.05), "{j:?} is silent");
            assert!(a.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        }
        let secs = jingle(Jingle::Drowning).len() as f32 / RATE as f32;
        assert!((secs - 12.0).abs() < 1.0, "the countdown lasts {secs} s");
        assert!(Jingle::ActClear.ends_song() && !Jingle::OneUp.ends_song());
    }

    /// Write every song and jingle as a WAV file into `$GE_MUSIC_DIR`,
    /// to listen to them: `GE_MUSIC_DIR=/tmp/m cargo test write_music -- --ignored`.
    #[test]
    #[ignore]
    fn write_music() {
        let dir = std::path::PathBuf::from(std::env::var("GE_MUSIC_DIR").expect("set GE_MUSIC_DIR"));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = |name: &str, samples: &[f32]| {
            let data: Vec<u8> = samples.iter().flat_map(|s| ((s * 32767.0) as i16).to_le_bytes()).collect();
            let mut f = Vec::new();
            f.extend(b"RIFF");
            f.extend((36 + data.len() as u32).to_le_bytes());
            f.extend(b"WAVEfmt ");
            f.extend(16u32.to_le_bytes());
            f.extend(1u16.to_le_bytes());
            f.extend(1u16.to_le_bytes());
            f.extend(RATE.to_le_bytes());
            f.extend((RATE * 2).to_le_bytes());
            f.extend(2u16.to_le_bytes());
            f.extend(16u16.to_le_bytes());
            f.extend(b"data");
            f.extend((data.len() as u32).to_le_bytes());
            f.extend(data);
            std::fs::write(dir.join(format!("{name}.wav")), f).unwrap();
        };
        for s in ALL {
            wav(&format!("{s:?}").to_lowercase(), &song(s));
        }
        for j in [Jingle::ActClear, Jingle::OneUp, Jingle::GameOver, Jingle::Emerald, Jingle::Drowning] {
            wav(&format!("jingle-{j:?}").to_lowercase(), &jingle(j));
        }
    }
}

//! Tracker module player (.mod / .xm / .it / .s3m / …).
//! Decoding runs on a background thread via libopenmpt; PCM floats are
//! streamed to cpal through a lock-free ring buffer.

use super::analyzer::Tap;
use super::eq::{EqParams, Equalizer};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};

enum Cmd {
    Load {
        data: Vec<u8>,
        name: String,
    },
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    /// Jump to this many seconds.
    Seek(f64),
    /// Move this many orders (patterns) forward or back.
    Order(i32),
}

#[derive(Clone, Default)]
pub struct TrackerInfo {
    pub loaded: bool,
    pub playing: bool,
    pub title: String,
    /// "XM", "IT", "MOD", "S3M", …
    pub fmt: String,
    pub name: String,
    pub order: i32,
    pub orders: i32,
    /// Seconds played and song length.
    pub position: f64,
    pub duration: f64,
    /// Played to the end (modules play once; the playlist moves on).
    pub ended: bool,
}

pub struct TrackerPlayer {
    tx: SyncSender<Cmd>,
    info: Arc<Mutex<TrackerInfo>>,
    _thread: std::thread::JoinHandle<()>,
}

impl TrackerPlayer {
    pub fn start(tap: Arc<Tap>, eq: Arc<EqParams>) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Cmd>(4);
        let info = Arc::new(Mutex::new(TrackerInfo::default()));
        let info2 = Arc::clone(&info);
        let thread = std::thread::spawn(move || audio_thread(rx, info2, tap, eq));
        Self { tx, info, _thread: thread }
    }

    pub fn load(&self, path: &str) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let name = crate::config::file_name_of(path);
        self.tx.send(Cmd::Load { data, name }).map_err(|_| "audio thread stopped".to_string())
    }

    pub fn toggle_pause(&self) {
        let cmd = if self.info().playing { Cmd::Pause } else { Cmd::Resume };
        self.tx.send(cmd).ok();
    }

    pub fn stop(&self) {
        self.tx.send(Cmd::Stop).ok();
    }

    pub fn seek(&self, seconds: f64) {
        self.tx.try_send(Cmd::Seek(seconds)).ok();
    }

    pub fn jump_orders(&self, delta: i32) {
        self.tx.try_send(Cmd::Order(delta)).ok();
    }

    pub fn set_volume(&self, vol: f32) {
        self.tx.try_send(Cmd::SetVolume(vol)).ok();
    }

    pub fn info(&self) -> TrackerInfo {
        self.info.lock().map(|i| i.clone()).unwrap_or_default()
    }
}

fn audio_thread(rx: Receiver<Cmd>, info: Arc<Mutex<TrackerInfo>>, tap: Arc<Tap>, eq: Arc<EqParams>) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use openmpt::module::metadata::MetadataKey;
    use openmpt::module::{Logger, Module};

    let Some(device) = cpal::default_host().default_output_device() else {
        eprintln!("tracker: no audio output device");
        return;
    };

    const SR: u32 = 48_000;
    const CH: u16 = 2;
    let cfg =
        cpal::StreamConfig { channels: CH, sample_rate: cpal::SampleRate(SR), buffer_size: cpal::BufferSize::Default };

    // 1 second of stereo headroom.
    let (mut prod, mut cons) = ringbuf::HeapRb::<f32>::new(SR as usize * CH as usize).split();
    // Volume is applied at output time (not when decoding ahead) so changes are
    // heard immediately; the visualizer tap sees the pre-volume signal.
    let volume = Arc::new(AtomicU32::new(1.0f32.to_bits()));
    let out_volume = Arc::clone(&volume);
    let mut equalizer = Equalizer::new(eq);

    let stream = match device.build_output_stream::<f32, _, _>(
        &cfg,
        move |out: &mut [f32], _| {
            let mut played = false;
            for s in out.iter_mut() {
                *s = match cons.pop() {
                    Some(v) => {
                        played = true;
                        v
                    }
                    None => 0.0,
                };
            }
            if played {
                equalizer.process(out, CH as usize, SR);
            }
            // Only feed the visualizer while a module is actually playing, so
            // this idle stream never mixes silence into another player's audio.
            if played {
                tap.push_interleaved(out, CH as usize, SR);
            }
            let v = f32::from_bits(out_volume.load(Ordering::Relaxed));
            for s in out.iter_mut() {
                *s *= v;
            }
        },
        |e| eprintln!("cpal error: {e}"),
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tracker: failed to open audio stream: {e}");
            return;
        }
    };
    if let Err(e) = stream.play() {
        eprintln!("tracker: stream.play failed: {e}");
        return;
    }

    let mut module: Option<Module> = None;
    let mut playing = false;
    let set = |f: &dyn Fn(&mut TrackerInfo)| {
        if let Ok(mut i) = info.lock() {
            f(&mut i);
        }
    };

    loop {
        loop {
            match rx.try_recv() {
                Ok(Cmd::Load { mut data, name }) => {
                    // create_from_memory sizes its read from buf.capacity().
                    if let Ok(mut m) = Module::create_from_memory(&mut data, Logger::None, &[]) {
                        m.set_repeat_count(0);
                        let title = m.get_metadata(MetadataKey::ModuleTitle).unwrap_or_default();
                        let fmt = m.get_metadata(MetadataKey::TypeExt).unwrap_or_default().to_uppercase();
                        let orders = m.get_num_orders();
                        let duration = m.get_duration_seconds();
                        set(&|i| {
                            *i = TrackerInfo {
                                loaded: true,
                                playing: true,
                                title: title.clone(),
                                fmt: fmt.clone(),
                                name: name.clone(),
                                order: 0,
                                orders,
                                position: 0.0,
                                duration,
                                ended: false,
                            }
                        });
                        playing = true;
                        module = Some(m);
                    } else {
                        eprintln!("tracker: could not parse {name}");
                    }
                }
                Ok(Cmd::Pause) => {
                    playing = false;
                    set(&|i| i.playing = false);
                }
                Ok(Cmd::Resume) => {
                    playing = module.is_some();
                    set(&|i| i.playing = i.loaded);
                }
                Ok(Cmd::Stop) => {
                    module = None;
                    playing = false;
                    set(&|i| *i = TrackerInfo::default());
                }
                Ok(Cmd::SetVolume(v)) => volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed),
                Ok(Cmd::Seek(s)) => {
                    if let Some(m) = module.as_mut() {
                        m.set_position_seconds(s.max(0.0));
                    }
                }
                Ok(Cmd::Order(d)) => {
                    if let Some(m) = module.as_mut() {
                        let n = m.get_num_orders().max(1);
                        let order = (m.get_current_order() + d).rem_euclid(n);
                        m.set_position_order_row(order, 0);
                    }
                }
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }

        if playing {
            if let Some(m) = module.as_mut() {
                let (order, position) = (m.get_current_order(), m.get_position_seconds());
                set(&|i| {
                    i.order = order;
                    i.position = position;
                });
                let want = (prod.free_len() / CH as usize).min(4096);
                if want > 0 {
                    // Pre-filled so capacity == want*2; the C API reads capacity>>1 frames.
                    let mut buf = vec![0.0f32; want * CH as usize];
                    let got = m.read_interleaved_float_stereo(SR as i32, &mut buf);
                    for &s in &buf[..got * CH as usize] {
                        prod.push(s).ok();
                    }
                    // The end of the song: wait for the buffer to drain.
                    if got == 0 && prod.is_empty() {
                        playing = false;
                        set(&|i| {
                            i.playing = false;
                            i.ended = true;
                        });
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

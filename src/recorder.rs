//! GIF recording (F11). Frames are grabbed from the framebuffer on the main
//! thread and handed to an encoder thread that downscales, quantises and
//! streams them to disk.

use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::thread::JoinHandle;

/// Capture rate.
pub const FPS: f64 = 20.0;
/// Recordings stop on their own after this long.
pub const MAX_SECS: f64 = 30.0;
/// Width of the GIF (the screen is scaled down to it).
const MAX_WIDTH: u32 = 560;
/// Frames waiting for the encoder; more are dropped rather than stalling the game.
const QUEUE: usize = 48;

struct RawFrame {
    /// Bottom-up RGBA, as read from the framebuffer.
    rgba: Vec<u8>,
    w: u32,
    h: u32,
    time: f64,
}

pub struct Recording {
    tx: SyncSender<RawFrame>,
    worker: JoinHandle<Result<(PathBuf, usize), String>>,
    pub started: f64,
    next_capture: f64,
    pub dropped: usize,
}

/// A recording that was stopped and is still being written.
pub struct Finishing(JoinHandle<Result<(PathBuf, usize), String>>);

impl Finishing {
    /// `Some` once the file is complete (or failed).
    pub fn poll(&mut self) -> Option<Result<(PathBuf, usize), String>> {
        if !self.0.is_finished() {
            return None;
        }
        let handle = std::mem::replace(&mut self.0, std::thread::spawn(|| Err(String::new())));
        Some(handle.join().unwrap_or_else(|_| Err("encoder crashed".into())))
    }
}

impl Recording {
    pub fn start(path: PathBuf, now: f64) -> Self {
        let (tx, rx) = sync_channel::<RawFrame>(QUEUE);
        let worker = std::thread::spawn(move || encode(path, rx));
        Recording { tx, worker, started: now, next_capture: now, dropped: 0 }
    }

    pub fn elapsed(&self, now: f64) -> f64 {
        now - self.started
    }

    /// Whether a frame is due at `now`.
    pub fn due(&self, now: f64) -> bool {
        now >= self.next_capture
    }

    /// Queue a bottom-up RGBA framebuffer copy.
    pub fn push(&mut self, rgba: Vec<u8>, w: u32, h: u32, now: f64) {
        self.next_capture = (self.next_capture + 1.0 / FPS).max(now);
        match self.tx.try_send(RawFrame { rgba, w, h, time: now }) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => self.dropped += 1,
            Err(TrySendError::Disconnected(_)) => self.dropped += 1,
        }
    }

    /// Stop capturing; the encoder finishes in the background.
    pub fn stop(self) -> Finishing {
        drop(self.tx);
        Finishing(self.worker)
    }
}

fn encode(path: PathBuf, rx: std::sync::mpsc::Receiver<RawFrame>) -> Result<(PathBuf, usize), String> {
    use gif::{Encoder, Frame, Repeat};
    let mut encoder: Option<Encoder<std::io::BufWriter<std::fs::File>>> = None;
    let mut size = (0u16, 0u16);
    // Each frame is written when the next one arrives, so its delay is known.
    let mut pending: Option<(Vec<u8>, f64)> = None;
    let mut written = 0usize;

    let write = |enc: &mut Encoder<_>, mut rgba: Vec<u8>, delay_s: f64, size: (u16, u16)| -> Result<(), String> {
        let mut frame = Frame::from_rgba_speed(size.0, size.1, &mut rgba, 20);
        frame.delay = ((delay_s * 100.0).round() as u16).clamp(2, 100);
        enc.write_frame(&frame).map_err(|e| e.to_string())
    };

    for raw in rx.iter() {
        let img = prepare(raw.rgba, raw.w, raw.h);
        if encoder.is_none() {
            size = (img.width() as u16, img.height() as u16);
            let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
            let mut enc =
                Encoder::new(std::io::BufWriter::new(file), size.0, size.1, &[]).map_err(|e| e.to_string())?;
            enc.set_repeat(Repeat::Infinite).map_err(|e| e.to_string())?;
            encoder = Some(enc);
        }
        // The window may have been resized mid-recording: keep the first size.
        let img = if (img.width() as u16, img.height() as u16) != size {
            image::imageops::resize(&img, size.0 as u32, size.1 as u32, image::imageops::FilterType::Triangle)
        } else {
            img
        };
        let enc = encoder.as_mut().expect("encoder created above");
        if let Some((prev, t)) = pending.take() {
            write(enc, prev, raw.time - t, size)?;
            written += 1;
        }
        pending = Some((img.into_raw(), raw.time));
    }
    match (encoder.as_mut(), pending) {
        (Some(enc), Some((prev, _))) => {
            write(enc, prev, 1.0 / FPS, size)?;
            written += 1;
        }
        _ => return Err("no frames captured".into()),
    }
    drop(encoder);
    Ok((path, written))
}

/// Flip the bottom-up framebuffer, force it opaque and scale it down.
fn prepare(rgba: Vec<u8>, w: u32, h: u32) -> image::RgbaImage {
    let row = w as usize * 4;
    let mut flipped = Vec::with_capacity(rgba.len());
    for y in (0..h as usize).rev() {
        flipped.extend_from_slice(&rgba[y * row..(y + 1) * row]);
    }
    for px in flipped.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    let img = image::RgbaImage::from_raw(w, h, flipped).unwrap_or_else(|| image::RgbaImage::new(1, 1));
    if w <= MAX_WIDTH {
        return img;
    }
    let nh = ((h as f32 * MAX_WIDTH as f32 / w as f32).round() as u32).max(1);
    image::imageops::resize(&img, MAX_WIDTH, nh, image::imageops::FilterType::Triangle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_a_small_gif() {
        let dir = std::env::temp_dir().join(format!("ge-rec-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.gif");
        let mut rec = Recording::start(path.clone(), 0.0);
        for i in 0..5 {
            let px = [(i * 50) as u8, 100, 200, 255];
            let rgba: Vec<u8> = px.iter().copied().cycle().take(64 * 32 * 4).collect();
            rec.push(rgba, 64, 32, i as f64 * 0.05);
        }
        let mut fin = rec.stop();
        let result = loop {
            if let Some(r) = fin.poll() {
                break r;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        let (p, n) = result.unwrap();
        assert_eq!(n, 5);
        let bytes = std::fs::read(&p).unwrap();
        assert!(bytes.starts_with(b"GIF89a"));
        let decoded = crate::assets::decode("test.gif", &bytes, 64, false).unwrap();
        assert_eq!(decoded.frames.len(), 5);
        std::fs::remove_dir_all(dir).ok();
    }
}

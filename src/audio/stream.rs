//! Common audio formats (.mp3 / .flac / .wav / .ogg / …) and .pls playlists
//! (including HTTP radio streams) via rodio + symphonia.

use super::analyzer::Tap;
use super::eq::{EqParams, EqSource};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;

/// Pass-through source that copies the samples being played to the
/// visualizer tap, in small chunks.
struct Tapped<S> {
    inner: S,
    tap: Arc<Tap>,
    chunk: Vec<f32>,
}

impl<S: Source<Item = f32>> Tapped<S> {
    fn new(inner: S, tap: Arc<Tap>) -> Self {
        Tapped { inner, tap, chunk: Vec::with_capacity(1024) }
    }
}

impl<S: Source<Item = f32>> Iterator for Tapped<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let s = self.inner.next()?;
        self.chunk.push(s);
        if self.chunk.len() >= 1024 {
            self.tap.push_interleaved(&self.chunk, self.inner.channels() as usize, self.inner.sample_rate());
            self.chunk.clear();
        }
        Some(s)
    }
}

impl<S: Source<Item = f32>> Source for Tapped<S> {
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
        self.chunk.clear();
        self.inner.try_seek(pos)
    }
}

/// Length of a local audio file, from its container (rodio 0.19 gets the
/// fractional part of `total_duration` wrong).
pub fn probe_duration(path: &str) -> Option<Duration> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(&crate::config::ext_of(path));
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let params = &probed.format.default_track()?.codec_params;
    let (frames, rate) = (params.n_frames?, params.sample_rate?);
    (rate > 0).then(|| Duration::from_secs_f64(frames as f64 / rate as f64))
}

/// Parse a .pls playlist. Relative paths are resolved against the playlist's
/// folder; missing local files are skipped, network URLs are kept verbatim.
pub fn parse_pls(content: &str, dir: &std::path::Path) -> Vec<String> {
    let mut entries = std::collections::BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.to_ascii_lowercase().starts_with("file") {
            continue;
        }
        let rest = &line[4..];
        let Some(eq) = rest.find('=') else { continue };
        let Ok(idx) = rest[..eq].trim().parse::<u32>() else { continue };
        let raw = rest[eq + 1..].trim();
        let lower = raw.to_ascii_lowercase();
        let is_url = ["http://", "https://", "rtsp://"].iter().any(|p| lower.starts_with(p));
        let entry = if is_url || std::path::Path::new(raw).is_absolute() {
            raw.to_string()
        } else {
            dir.join(raw).to_string_lossy().into_owned()
        };
        if is_url || std::path::Path::new(&entry).exists() {
            entries.insert(idx, entry);
        }
    }
    entries.into_values().collect()
}

/// Parse an .m3u / .m3u8 playlist: one path or URL per line, `#` lines
/// are comments. Relative paths are resolved like in [`parse_pls`].
pub fn parse_m3u(content: &str, dir: &std::path::Path) -> Vec<String> {
    content
        .lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|raw| {
            let lower = raw.to_ascii_lowercase();
            let is_url = ["http://", "https://"].iter().any(|p| lower.starts_with(p));
            let path = raw.strip_prefix("file://").unwrap_or(raw);
            let entry = if is_url || std::path::Path::new(path).is_absolute() {
                path.to_string()
            } else {
                dir.join(path).to_string_lossy().into_owned()
            };
            (is_url || std::path::Path::new(&entry).exists()).then_some(entry)
        })
        .collect()
}

/// Lets an HTTP body satisfy rodio's `Read + Seek` bound. Seeking is a no-op;
/// symphonia never needs to rewind MP3/AAC radio streams.
struct HttpStream {
    inner: Box<dyn std::io::Read + Send + Sync + 'static>,
    pos: u64,
}

impl HttpStream {
    fn open(url: &str) -> Option<Self> {
        let agent = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_secs(8)).build();
        let resp = agent
            .get(url)
            .set("User-Agent", concat!("gravity-engine/", env!("CARGO_PKG_VERSION")))
            .set("Icy-MetaData", "0")
            .call()
            .ok()?;
        Some(HttpStream { inner: Box::new(resp.into_reader()), pos: 0 })
    }
}

impl std::io::Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl std::io::Seek for HttpStream {
    fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
        Ok(self.pos)
    }
}

#[derive(Clone, Default)]
pub struct StreamInfo {
    pub loaded: bool,
    pub playing: bool,
    pub name: String,
    pub fmt: String,
}

/// Plays one file or radio stream at a time; the playlist is the caller's.
pub struct StreamPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    state: StreamInfo,
    duration: Option<Duration>,
    volume: f32,
    tap: Arc<Tap>,
    eq: Arc<EqParams>,
}

impl StreamPlayer {
    pub fn new(tap: Arc<Tap>, eq: Arc<EqParams>) -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Self {
            _stream: stream,
            handle,
            sink: None,
            state: StreamInfo::default(),
            duration: None,
            volume: 1.0,
            tap,
            eq,
        })
    }

    /// Start playing `path` (a file or an http(s) URL) from the beginning.
    pub fn play(&mut self, path: &str) -> Result<(), String> {
        type Boxed = Box<dyn Source<Item = f32> + Send>;
        let lower = path.to_ascii_lowercase();
        let source: Boxed = if lower.starts_with("http://") || lower.starts_with("https://") {
            let s = HttpStream::open(path).ok_or("could not open the stream")?;
            Box::new(Decoder::new(BufReader::new(s)).map_err(|e| e.to_string())?.convert_samples())
        } else {
            let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
            Box::new(Decoder::new(BufReader::new(f)).map_err(|e| e.to_string())?.convert_samples())
        };
        if let Some(s) = self.sink.take() {
            s.stop();
        }
        let sink = Sink::try_new(&self.handle).map_err(|e| e.to_string())?;
        sink.set_volume(self.volume);
        sink.append(Tapped::new(EqSource::new(source, self.eq.clone()), self.tap.clone()));
        self.sink = Some(sink);
        self.duration = probe_duration(path);
        self.state = StreamInfo {
            loaded: true,
            playing: true,
            name: crate::config::file_name_of(path),
            fmt: crate::config::ext_of(path).to_uppercase(),
        };
        Ok(())
    }

    /// The song played to its end.
    pub fn finished(&self) -> bool {
        self.state.loaded && self.state.playing && self.sink.as_ref().is_some_and(|s| s.empty())
    }

    /// Seconds played, and the length when known.
    pub fn position(&self) -> Option<(f64, Option<f64>)> {
        let sink = self.sink.as_ref()?;
        Some((sink.get_pos().as_secs_f64(), self.duration.map(|d| d.as_secs_f64())))
    }

    /// Jump to `frac` (0‥1) of the song, when its length is known.
    pub fn seek(&mut self, frac: f64) {
        let (Some(sink), Some(len)) = (&self.sink, self.duration) else { return };
        let len = len.as_secs_f64();
        sink.try_seek(Duration::from_secs_f64((len * frac.clamp(0.0, 1.0)).min(len - 0.05).max(0.0))).ok();
    }

    pub fn toggle_pause(&mut self) {
        if let Some(sink) = &self.sink {
            if self.state.playing {
                sink.pause();
            } else {
                sink.play();
            }
            self.state.playing = !self.state.playing;
        }
    }

    pub fn stop(&mut self) {
        if let Some(s) = self.sink.take() {
            s.stop();
        }
        self.duration = None;
        self.state = StreamInfo::default();
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        if let Some(s) = &self.sink {
            s.set_volume(self.volume);
        }
    }

    pub fn info(&self) -> &StreamInfo {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pls_orders_and_filters_entries() {
        let dir = std::env::temp_dir().join(format!("ge_pls_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.mp3"), b"x").unwrap();
        let pls = "[playlist]\nFile2=http://radio.example/stream\nFile1=a.mp3\nFile3=missing.ogg\nNumberOfEntries=3\n";
        let got = parse_pls(pls, &dir);
        assert_eq!(got.len(), 2);
        assert!(got[0].ends_with("a.mp3"));
        assert_eq!(got[1], "http://radio.example/stream");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn m3u_skips_comments_and_missing_files() {
        let dir = std::env::temp_dir().join(format!("ge_m3u_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b.ogg"), b"x").unwrap();
        let m3u = "#EXTM3U\n#EXTINF:12,Song\nb.ogg\n\nmissing.mp3\nhttps://radio.example/live\n";
        let got = parse_m3u(m3u, &dir);
        assert_eq!(got.len(), 2);
        assert!(got[0].ends_with("b.ogg"));
        assert_eq!(got[1], "https://radio.example/live");
        std::fs::remove_dir_all(dir).ok();
    }
}

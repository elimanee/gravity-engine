//! Common audio formats (.mp3 / .flac / .wav / .ogg / …) and .pls playlists
//! (including HTTP radio streams) via rodio + symphonia.

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::BufReader;

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

pub struct StreamPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    state: StreamInfo,
    /// Non-empty when a playlist is loaded; re-queued when the sink drains.
    playlist: Vec<String>,
    volume: f32,
}

impl StreamPlayer {
    pub fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Self { _stream: stream, handle, sink: None, state: StreamInfo::default(), playlist: vec![], volume: 1.0 })
    }

    fn new_sink(&mut self) -> Result<Sink, String> {
        if let Some(s) = self.sink.take() {
            s.stop();
        }
        let sink = Sink::try_new(&self.handle).map_err(|e| e.to_string())?;
        sink.set_volume(self.volume);
        Ok(sink)
    }

    /// Play a single file, looping forever.
    pub fn load(&mut self, path: &str) -> Result<(), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let decoder = Decoder::new(BufReader::new(file)).map_err(|e| e.to_string())?;
        let sink = self.new_sink()?;
        sink.append(decoder.repeat_infinite());
        self.sink = Some(sink);
        self.playlist.clear();
        self.state = StreamInfo {
            loaded: true,
            playing: true,
            name: crate::config::file_name_of(path),
            fmt: crate::config::ext_of(path).to_uppercase(),
        };
        Ok(())
    }

    /// Queue every entry of a playlist; loops as a whole.
    pub fn load_playlist(&mut self, paths: Vec<String>, display_name: String) -> Result<usize, String> {
        let sink = self.new_sink()?;
        let mut n = 0usize;
        for p in &paths {
            let lower = p.to_ascii_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") {
                match HttpStream::open(p).map(|s| Decoder::new(BufReader::new(s))) {
                    Some(Ok(d)) => {
                        sink.append(d);
                        n += 1;
                    }
                    _ => eprintln!("stream failed: {p}"),
                }
            } else if let Ok(f) = std::fs::File::open(p) {
                if let Ok(d) = Decoder::new(BufReader::new(f)) {
                    sink.append(d);
                    n += 1;
                }
            }
        }
        if n == 0 {
            return Err("no playable entries".into());
        }
        self.sink = Some(sink);
        self.playlist = paths;
        self.state = StreamInfo { loaded: true, playing: true, name: display_name, fmt: "PLS".into() };
        Ok(n)
    }

    /// Call once per frame: re-queues the playlist when the sink drains.
    pub fn tick(&mut self) {
        if self.playlist.is_empty() || !self.state.playing {
            return;
        }
        if self.sink.as_ref().is_some_and(|s| s.empty()) {
            let paths = self.playlist.clone();
            let name = self.state.name.clone();
            if self.load_playlist(paths, name).is_err() {
                self.stop();
            }
        }
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
        self.playlist.clear();
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
}

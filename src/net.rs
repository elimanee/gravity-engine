//! Background web fetchers: classic 88×31 buttons and SteamGridDB game logos.
//! Everything runs on worker threads; the UI polls a `FetchJob` each frame.

use crate::util::Lcg;
use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub enum FetchEvent {
    Item { name: String, data: Vec<u8> },
    Failed(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FetchKind {
    Buttons,
    Logos,
}

impl FetchKind {
    pub fn label(self) -> &'static str {
        match self {
            FetchKind::Buttons => "88×31 buttons",
            FetchKind::Logos => "game logos",
        }
    }
}

pub struct FetchJob {
    pub kind: FetchKind,
    pub wanted: usize,
    pub received: usize,
    pub done: bool,
    /// A failure was already reported to the user.
    pub failed: bool,
    rx: Receiver<FetchEvent>,
}

impl FetchJob {
    pub fn buttons(count: usize) -> Self {
        Self::spawn(FetchKind::Buttons, count, fetch_buttons)
    }

    pub fn logos(count: usize) -> Self {
        Self::spawn(FetchKind::Logos, count, fetch_logos)
    }

    fn spawn(kind: FetchKind, count: usize, f: fn(Sender<FetchEvent>, usize)) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || f(tx, count));
        FetchJob { kind, wanted: count, received: 0, done: false, failed: false, rx }
    }

    /// Drain pending events without blocking.
    pub fn poll(&mut self) -> Vec<FetchEvent> {
        let mut out = vec![];
        loop {
            match self.rx.try_recv() {
                Ok(ev) => {
                    match ev {
                        FetchEvent::Item { .. } => self.received += 1,
                        FetchEvent::Failed(_) => self.failed = true,
                    }
                    out.push(ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.done = true;
                    break;
                }
            }
        }
        out
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(6))
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("gravity-engine/", env!("CARGO_PKG_VERSION")))
        .build()
}

fn download(agent: &ureq::Agent, url: &str, limit: u64) -> Option<Vec<u8>> {
    let resp = agent.get(url).call().ok()?;
    let mut bytes = Vec::new();
    resp.into_reader().take(limit).read_to_end(&mut bytes).ok()?;
    (bytes.len() > 32).then_some(bytes)
}

/// Run `job` over `inputs` on a few worker threads until `count` items have
/// been delivered, the inputs run out, or the receiver hangs up.
fn parallel<T: Send + 'static>(
    inputs: Vec<T>,
    count: usize,
    threads: usize,
    tx: Sender<FetchEvent>,
    job: impl Fn(&ureq::Agent, T) -> Option<(String, Vec<u8>)> + Send + Sync + 'static,
) {
    let queue = Arc::new(Mutex::new(VecDeque::from(inputs)));
    let sent = Arc::new(AtomicUsize::new(0));
    let job = Arc::new(job);
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let (queue, sent, job, tx) = (queue.clone(), sent.clone(), job.clone(), tx.clone());
            std::thread::spawn(move || {
                let agent = agent();
                loop {
                    if sent.load(Ordering::SeqCst) >= count {
                        return;
                    }
                    let Some(item) = queue.lock().ok().and_then(|mut q| q.pop_front()) else { return };
                    if let Some((name, data)) = job(&agent, item) {
                        if sent.fetch_add(1, Ordering::SeqCst) >= count {
                            return;
                        }
                        if tx.send(FetchEvent::Item { name, data }).is_err() {
                            return;
                        }
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().ok();
    }
}

// ── 88×31 buttons ───────────────────────────────────────────────
const BUTTON_SOURCES: &[(&str, &str)] = &[
    ("https://cyber.dabamos.de/88x31/", "https://cyber.dabamos.de/88x31/"),
    ("https://hellnet.work/8831/", "https://hellnet.work/8831/"),
    ("https://anlucas.neocities.org/88x31Buttons", "https://anlucas.neocities.org/"),
    ("https://88x31.neocities.org/", "https://88x31.neocities.org/"),
    ("https://goblin-heart.net/sadgrl/webmastery/", "https://goblin-heart.net/"),
    ("https://pixelbuttons.neocities.org/", "https://pixelbuttons.neocities.org/"),
    ("https://alistairshepherd.uk/buttons/", "https://alistairshepherd.uk/buttons/"),
    ("https://dimden.dev/buttons/", "https://dimden.dev/buttons/"),
];

/// Extract image URLs from `src="…"` attributes.
pub fn scrape_image_urls(html: &str, base: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for chunk in html.split("src=\"").skip(1) {
        let Some(end) = chunk.find('"') else { continue };
        let src = &chunk[..end];
        let lower = src.to_ascii_lowercase();
        if ![".gif", ".png", ".jpg", ".jpeg", ".webp"].iter().any(|e| lower.ends_with(e)) {
            continue;
        }
        let url = if src.starts_with("http://") || src.starts_with("https://") {
            src.to_string()
        } else if let Some(rest) = src.strip_prefix("//") {
            format!("https://{rest}")
        } else {
            format!("{}{}", base, src.trim_start_matches("./").trim_start_matches('/'))
        };
        if !urls.contains(&url) {
            urls.push(url);
        }
    }
    urls
}

fn fetch_buttons(tx: Sender<FetchEvent>, count: usize) {
    // Scrape every gallery in parallel.
    let scrapers: Vec<_> = BUTTON_SOURCES
        .iter()
        .map(|&(page, base)| {
            std::thread::spawn(move || {
                agent()
                    .get(page)
                    .call()
                    .ok()
                    .and_then(|r| r.into_string().ok())
                    .map(|html| scrape_image_urls(&html, base))
                    .unwrap_or_default()
            })
        })
        .collect();
    let mut urls: Vec<String> = scrapers.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect();
    urls.sort();
    urls.dedup();
    if urls.is_empty() {
        tx.send(FetchEvent::Failed("could not reach any button gallery".into())).ok();
        return;
    }
    Lcg::from_time().shuffle(&mut urls);
    urls.truncate(count * 3);
    parallel(urls, count, 6, tx, |agent, url| {
        let data = download(agent, &url, 512_000)?;
        let name = url.rsplit('/').next().unwrap_or("button.gif").to_string();
        Some((name, data))
    });
}

// ── SteamGridDB logos ───────────────────────────────────────────
const STEAM_APP_IDS: &[u32] = &[
    // Valve
    730, 570, 440, 400, 620, 220, 550, 4000, 240, 380, 10, 70, 80, // Action / shooter
    1245620, 1091500, 271590, 578080, 359550, 1938090, 1172470, 1551360, 1716740, 1203220, 814380, 570940, 1517290,
    1093900, // RPG
    292030, 374320, 489830, 72850, 306130, 413150, 1145360, 582010, 1174180, 2767030, 2358720,
    // Indie / platformer
    367520, 1222670, 391540, 548430, 824270, 1942280, 1085660, 105600, 1318690, 1062090, 774361, 1150690,
    // Strategy / simulation
    281990, 8930, 294100, 739630, 1158310, 255710, 236850, 322330, 4920, 251570, 242760, 108600,
    // Horror / thriller
    319510, 418370, 601150, 698780, 1253920, 952060, 782330, // Sports / racing
    1449560, 1303850, 230410, 304930, 311210, // Classics
    4560, 8190, 17390, 22200, 32400, 7670, 2600, 6860, // More popular
    1172620, 1283500, 1593500, 1818750, 1817190,
];

/// API key from `$SGDB_API_KEY` or `~/.config/gravity_engine/sgdb_key`.
pub fn sgdb_key() -> Option<String> {
    if let Ok(k) = std::env::var("SGDB_API_KEY") {
        if !k.trim().is_empty() {
            return Some(k.trim().to_string());
        }
    }
    let path = crate::config::config_dir()?.join("sgdb_key");
    let k = std::fs::read_to_string(path).ok()?;
    let k = k.trim();
    (!k.is_empty()).then(|| k.to_string())
}

/// First logo URL in a SteamGridDB `/logos` response.
pub fn sgdb_first_url(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.get("data")?
        .as_array()?
        .iter()
        .filter_map(|e| e.get("url")?.as_str())
        .find(|u| u.starts_with("https://"))
        .map(str::to_string)
}

fn fetch_logos(tx: Sender<FetchEvent>, count: usize) {
    let Some(key) = sgdb_key() else {
        tx.send(FetchEvent::Failed(
            "No SteamGridDB key — set SGDB_API_KEY or ~/.config/gravity_engine/sgdb_key".into(),
        ))
        .ok();
        return;
    };
    let mut ids = STEAM_APP_IDS.to_vec();
    Lcg::from_time().shuffle(&mut ids);
    ids.truncate(count * 3);
    let auth = format!("Bearer {key}");
    parallel(ids, count, 4, tx, move |agent, id| {
        let json = agent
            .get(&format!("https://www.steamgriddb.com/api/v2/logos/steam/{id}"))
            .set("Authorization", &auth)
            .call()
            .ok()?
            .into_string()
            .ok()?;
        let url = sgdb_first_url(&json)?;
        let data = download(agent, &url, 4_000_000)?;
        Some((format!("sgdb_{id}.png"), data))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrapes_relative_and_absolute_urls() {
        let html = r#"<img src="a.gif"><img src="./b.png"><img src="/c.jpg">
            <img src="https://x.org/d.gif"><img src="//cdn.org/e.png"><img src="f.html"><img src="a.gif">"#;
        let got = scrape_image_urls(html, "https://site/");
        assert_eq!(
            got,
            vec![
                "https://site/a.gif",
                "https://site/b.png",
                "https://site/c.jpg",
                "https://x.org/d.gif",
                "https://cdn.org/e.png"
            ]
        );
    }

    #[test]
    fn parses_sgdb_json() {
        let json = r#"{"success":true,"data":[{"id":1,"url":"https://cdn2.steamgriddb.com/logo/x.png"}]}"#;
        assert_eq!(sgdb_first_url(json).as_deref(), Some("https://cdn2.steamgriddb.com/logo/x.png"));
        assert_eq!(sgdb_first_url(r#"{"data":[]}"#), None);
    }
}

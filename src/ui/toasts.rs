//! Transient notifications and background job progress.

use super::theme::*;
use crate::net::FetchJob;
use crate::util::approach;
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Info,
    Success,
    Warning,
    Error,
}

impl Kind {
    fn color(self) -> Color {
        match self {
            Kind::Info => ACCENT_HI,
            Kind::Success => SUCCESS,
            Kind::Warning => WARNING,
            Kind::Error => DANGER,
        }
    }
}

struct Toast {
    key: Option<&'static str>,
    text: String,
    kind: Kind,
    age: f32,
    life: f32,
    y: f32,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, kind: Kind, text: impl Into<String>) {
        self.push_keyed(None, kind, text);
    }

    /// Toasts sharing a key replace each other (e.g. repeatedly cycling a setting).
    pub fn push_keyed(&mut self, key: Option<&'static str>, kind: Kind, text: impl Into<String>) {
        let text = text.into();
        let life = if matches!(kind, Kind::Error | Kind::Warning) { 4.5 } else { 2.4 };
        if let Some(t) = self.items.iter_mut().find(|t| t.text == text || (key.is_some() && t.key == key)) {
            t.text = text;
            t.kind = kind;
            t.age = t.age.min(0.2);
            t.life = life;
            return;
        }
        self.items.push(Toast { key, text, kind, age: 0.0, life, y: -1.0 });
        if self.items.len() > 5 {
            self.items.remove(0);
        }
    }

    pub fn info(&mut self, t: impl Into<String>) {
        self.push(Kind::Info, t);
    }
    /// Info toast that replaces the previous one with the same key.
    pub fn status(&mut self, key: &'static str, t: impl Into<String>) {
        self.push_keyed(Some(key), Kind::Info, t);
    }
    pub fn success(&mut self, t: impl Into<String>) {
        self.push(Kind::Success, t);
    }
    pub fn warn(&mut self, t: impl Into<String>) {
        self.push(Kind::Warning, t);
    }
    pub fn error(&mut self, t: impl Into<String>) {
        self.push(Kind::Error, t);
    }

    pub fn update(&mut self, dt: f32) {
        for t in &mut self.items {
            t.age += dt;
        }
        self.items.retain(|t| t.age < t.life);
    }

    pub fn draw(&mut self, top: f32, jobs: &[FetchJob], dt: f32) {
        let sw = screen_width();
        let mut y = top + 8.0;
        let font = 13.0;
        let h = 32.0;

        for job in jobs {
            let label = format!("Fetching {} …  {}/{}", job.kind.label(), job.received, job.wanted);
            let w = measure(&label, font) + 44.0;
            let r = Rect::new((sw - w) / 2.0, y, w, h);
            panel(r, 1.0);
            let t = get_time() as f32;
            for i in 0..3 {
                let a = 0.3 + 0.7 * ((t * 5.0 - i as f32 * 0.6).sin() * 0.5 + 0.5);
                draw_circle(r.x + 14.0 + i as f32 * 6.0, r.y + h / 2.0, 2.2, alpha(ACCENT_HI, a));
            }
            text(&label, r.x + 34.0, baseline(r.y + h / 2.0, font), font, TEXT_DIM);
            let p = job.received as f32 / job.wanted.max(1) as f32;
            rrect(Rect::new(r.x + 10.0, r.y + h - 4.0, (r.w - 20.0) * p, 2.0), 1.0, ACCENT);
            y += h + 6.0;
        }

        for t in &mut self.items {
            let target = y;
            t.y = if t.y < 0.0 { target - 10.0 } else { approach(t.y, target, 16.0, dt) };
            let fade_in = (t.age / 0.18).min(1.0);
            let fade_out = ((t.life - t.age) / 0.4).min(1.0);
            let f = ease_out_cubic(fade_in.min(fade_out));
            let mut shown = t.text.clone();
            while measure(&shown, font) + 40.0 > sw - 40.0 && shown.chars().count() > 8 {
                shown = crate::util::ellipsize(&shown, shown.chars().count() - 4);
            }
            let w = measure(&shown, font) + 40.0;
            let r = Rect::new((sw - w) / 2.0, t.y, w, h);
            panel(r, f);
            draw_circle(r.x + 16.0, r.y + h / 2.0, 4.0, fade(t.kind.color(), f));
            text(&shown, r.x + 28.0, baseline(r.y + h / 2.0, font), font, fade(TEXT, f));
            y += (h + 6.0) * f;
        }
    }
}

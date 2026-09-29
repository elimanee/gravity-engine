//! Settings drawer: slides in from the right while the simulation is paused
//! and gathers every setting in one scrollable place.

use super::theme::*;
use super::widgets::*;
use super::{Action, Fader, Input};
use crate::settings::*;
use macroquad::prelude::*;

const W: f32 = 300.0;
const PAD: f32 = 16.0;

#[derive(Clone, Copy, PartialEq)]
enum SliderId {
    Gravity,
    TimeScale,
    TrailLength,
    TrailFade,
    Shake,
    Volume,
    VisGain,
    WaterLevel,
    WaterDensity,
    SfxVolume,
}

#[derive(Clone, Copy, PartialEq)]
enum ToggleId {
    Trails,
    Shake,
    Dance,
    Water,
    Effects,
    Sfx,
    SlowMo,
}

#[derive(Clone, Copy, PartialEq)]
enum StepperId {
    Border,
    Background,
    VisBackground,
    VisObject,
    WorldSize,
}

enum Row {
    Header(&'static str),
    Slider(SliderId),
    Toggle(ToggleId),
    Stepper(StepperId),
    Buttons(&'static [(&'static str, Action)]),
}

impl Row {
    fn height(&self) -> f32 {
        match self {
            Row::Header(_) => 30.0,
            Row::Slider(_) => SLIDER_ROW_H + 6.0,
            Row::Toggle(_) => 32.0,
            Row::Stepper(_) => 38.0,
            Row::Buttons(_) => 38.0,
        }
    }
}

const WORLD_BUTTONS: &[(&str, Action)] = &[("Image…", Action::PickBackground), ("Reset all", Action::ResetSettings)];
const AUDIO_BUTTONS: &[(&str, Action)] = &[("Load…", Action::LoadAudio), ("Play / Pause", Action::ToggleAudio)];
const ADD_BUTTONS: &[(&str, Action)] = &[("Images…", Action::AddImages), ("Shapes", Action::ToggleSpawner)];
const WEB_BUTTONS: &[(&str, Action)] = &[("88×31 buttons", Action::FetchButtons), ("Game logos", Action::FetchLogos)];
const SCENE_BUTTONS: &[(&str, Action)] = &[("Save…", Action::SaveScene), ("Open…", Action::LoadScene)];
const VIS_BUTTONS: &[(&str, Action)] = &[("Spawn visualizer  (Shift+V)", Action::SpawnVisualizer)];
const SCENE_BUTTONS_2: &[(&str, Action)] = &[("Screenshot", Action::Screenshot), ("Clear all", Action::ClearAll)];
const EDIT_BUTTONS: &[(&str, Action)] = &[("Undo  (Ctrl+Z)", Action::Undo), ("Redo  (Ctrl+Y)", Action::Redo)];
const GIF_BUTTONS: &[(&str, Action)] = &[("Record / stop a GIF  (F11)", Action::ToggleRecording)];
const LIBRARY_BUTTONS: &[(&str, Action)] = &[("Examples & challenges  (E)", Action::OpenLibrary)];

#[derive(Default)]
pub struct Drawer {
    pub fader: Fader,
    scroll: f32,
    sliders: [SliderState; 10],
}

fn slider_index(id: SliderId) -> usize {
    id as usize
}

fn slider_spec(id: SliderId, s: &Settings) -> (SliderSpec<'static>, f32) {
    let (label, range, value, text, accent) = match id {
        SliderId::Gravity => ("Gravity", GRAVITY_RANGE, s.gravity, format!("{:+.1} m/s²", s.gravity), ACCENT),
        SliderId::TimeScale => ("Time scale", TIME_SCALE_RANGE, s.time_scale, format!("×{:.2}", s.time_scale), ACCENT),
        SliderId::TrailLength => {
            ("Length", TRAIL_LEN_RANGE, s.trail_length, format!("{}", s.trail_length as usize), SUCCESS)
        }
        SliderId::TrailFade => ("Fade", TRAIL_FADE_RANGE, s.trail_fade, format!("{:.1} s", s.trail_fade), SUCCESS),
        SliderId::Shake => ("Force", SHAKE_RANGE, s.shake_force, format!("{:.1}", s.shake_force), WARNING),
        SliderId::Volume => {
            ("Volume", (0.0, 1.0), s.volume, format!("{:.0}%", s.volume * 100.0), Color::new(0.4, 0.8, 1.0, 1.0))
        }
        SliderId::VisGain => {
            ("Sensitivity", VIS_GAIN_RANGE, s.vis_gain, format!("×{:.1}", s.vis_gain), Color::new(0.9, 0.5, 1.0, 1.0))
        }
        SliderId::WaterLevel => {
            let text = format!("{:.0}%", s.water_level * 100.0);
            ("Level", WATER_LEVEL_RANGE, s.water_level, text, Color::new(0.35, 0.65, 1.0, 1.0))
        }
        SliderId::WaterDensity => {
            let text = format!("×{:.1}", s.water_density);
            ("Density", WATER_DENSITY_RANGE, s.water_density, text, Color::new(0.35, 0.65, 1.0, 1.0))
        }
        SliderId::SfxVolume => {
            let text = format!("{:.0}%", s.sfx_volume * 100.0);
            ("Effects volume", (0.0, 1.0), s.sfx_volume, text, Color::new(0.4, 0.8, 1.0, 1.0))
        }
    };
    (SliderSpec { label, value_text: text, min: range.0, max: range.1, accent }, value)
}

fn slider_value(id: SliderId, s: &mut Settings) -> &mut f32 {
    match id {
        SliderId::Gravity => &mut s.gravity,
        SliderId::TimeScale => &mut s.time_scale,
        SliderId::TrailLength => &mut s.trail_length,
        SliderId::TrailFade => &mut s.trail_fade,
        SliderId::Shake => &mut s.shake_force,
        SliderId::Volume => &mut s.volume,
        SliderId::VisGain => &mut s.vis_gain,
        SliderId::WaterLevel => &mut s.water_level,
        SliderId::WaterDensity => &mut s.water_density,
        SliderId::SfxVolume => &mut s.sfx_volume,
    }
}

impl Drawer {
    fn rows(s: &Settings) -> Vec<Row> {
        let mut rows = vec![
            Row::Header("WORLD"),
            Row::Slider(SliderId::Gravity),
            Row::Slider(SliderId::TimeScale),
            Row::Stepper(StepperId::Border),
            Row::Stepper(StepperId::Background),
            Row::Stepper(StepperId::WorldSize),
            Row::Buttons(WORLD_BUTTONS),
            Row::Header("TRAILS"),
            Row::Toggle(ToggleId::Trails),
        ];
        if s.trails {
            rows.push(Row::Slider(SliderId::TrailLength));
            rows.push(Row::Slider(SliderId::TrailFade));
        }
        rows.push(Row::Header("WINDOW SHAKE"));
        rows.push(Row::Toggle(ToggleId::Shake));
        if s.window_shake {
            rows.push(Row::Slider(SliderId::Shake));
        }
        rows.push(Row::Header("WATER"));
        rows.push(Row::Toggle(ToggleId::Water));
        if s.water {
            rows.push(Row::Slider(SliderId::WaterLevel));
            rows.push(Row::Slider(SliderId::WaterDensity));
        }
        rows.extend([
            Row::Header("AUDIO"),
            Row::Slider(SliderId::Volume),
            Row::Buttons(AUDIO_BUTTONS),
            Row::Toggle(ToggleId::Sfx),
        ]);
        if s.sfx {
            rows.push(Row::Slider(SliderId::SfxVolume));
        }
        rows.extend([
            Row::Header("VISUALIZER"),
            Row::Stepper(StepperId::VisBackground),
            Row::Stepper(StepperId::VisObject),
            Row::Slider(SliderId::VisGain),
            Row::Toggle(ToggleId::Dance),
            Row::Buttons(VIS_BUTTONS),
            Row::Header("EFFECTS"),
            Row::Toggle(ToggleId::Effects),
            Row::Toggle(ToggleId::SlowMo),
            Row::Header("ADD OBJECTS"),
            Row::Buttons(LIBRARY_BUTTONS),
            Row::Buttons(ADD_BUTTONS),
            Row::Buttons(WEB_BUTTONS),
            Row::Header("SCENE"),
            Row::Buttons(SCENE_BUTTONS),
            Row::Buttons(SCENE_BUTTONS_2),
            Row::Buttons(EDIT_BUTTONS),
            Row::Buttons(GIF_BUTTONS),
        ]);
        rows
    }

    fn panel_rect(&self, top: f32) -> Rect {
        let (sw, sh) = (screen_width(), screen_height());
        let f = self.fader.value();
        let top = top.max(10.0);
        Rect::new(sw - W - 10.0 + (1.0 - f) * (W + 30.0), top, W, (sh - top - 10.0).max(120.0))
    }

    /// Row rectangles in screen space (after scrolling), plus the content viewport.
    fn layout(&self, s: &Settings, top: f32) -> (Rect, Rect, Vec<(Row, Rect)>, f32) {
        let p = self.panel_rect(top);
        let view = Rect::new(p.x, p.y + 44.0, p.w, p.h - 44.0 - 8.0);
        let mut y = view.y - self.scroll;
        let mut out = vec![];
        for row in Self::rows(s) {
            let h = row.height();
            out.push((row, Rect::new(p.x + PAD, y, p.w - PAD * 2.0, h)));
            y += h;
        }
        let content_h = y + self.scroll - view.y + 8.0;
        (p, view, out, content_h)
    }

    pub fn update(&mut self, dt: f32, s: &mut Settings, top: f32, input: &mut Input, actions: &mut Vec<Action>) {
        self.fader.update(dt, 5.0);
        if !self.fader.visible() {
            for st in &mut self.sliders {
                st.dragging = false;
            }
            return;
        }
        let (p, view, rows, content_h) = self.layout(s, top);
        if p.contains(input.mouse) && input.wheel != 0.0 {
            self.scroll -= input.wheel * 40.0;
        }
        self.scroll = self.scroll.clamp(0.0, (content_h - view.h).max(0.0));

        // Header "resume" button.
        let resume = Rect::new(p.x + p.w - 96.0, p.y + 10.0, 84.0, 26.0);
        if button(resume, input) {
            actions.push(Action::TogglePause);
        }

        for (row, r) in rows {
            let visible = r.y + r.h > view.y && r.y < view.y + view.h;
            let pointer_in_view = view.contains(input.mouse);
            match row {
                Row::Slider(id) => {
                    let st = &mut self.sliders[slider_index(id)];
                    let (spec, _) = slider_spec(id, s);
                    if st.dragging || (visible && pointer_in_view) {
                        st.update(r, slider_value(id, s), spec.min, spec.max, input);
                        if id == SliderId::Gravity && st.dragging {
                            // Snap to presets when close.
                            for (_, g) in crate::config::GRAVITY_PRESETS {
                                if (s.gravity - g).abs() < 0.6 {
                                    s.gravity = *g;
                                }
                            }
                        }
                    }
                }
                _ if !visible || !pointer_in_view => {}
                Row::Toggle(id) => {
                    if toggle_row(r, input) {
                        match id {
                            ToggleId::Trails => s.trails = !s.trails,
                            ToggleId::Shake => s.window_shake = !s.window_shake,
                            ToggleId::Dance => s.vis_dance = !s.vis_dance,
                            ToggleId::Water => actions.push(Action::ToggleWater),
                            ToggleId::Effects => actions.push(Action::ToggleEffects),
                            ToggleId::Sfx => s.sfx = !s.sfx,
                            ToggleId::SlowMo => s.slow_motion = !s.slow_motion,
                        }
                    }
                }
                Row::Stepper(id) => {
                    let sr = Rect::new(r.x, r.y + 3.0, r.w, r.h - 8.0);
                    let d = stepper(sr, input);
                    if d != 0 {
                        actions.push(match id {
                            StepperId::Border => Action::CycleBorder(d),
                            StepperId::Background => Action::CycleBackground(d),
                            StepperId::VisBackground => Action::CycleVisualizer(d),
                            StepperId::VisObject => Action::CycleVisualizerObject(d),
                            StepperId::WorldSize => Action::CycleWorldSize(d),
                        });
                    }
                }
                Row::Buttons(btns) => {
                    for (i, br) in button_rects(r, btns.len()).into_iter().enumerate() {
                        if button(br, input) {
                            actions.push(btns[i].1);
                        }
                    }
                }
                Row::Header(_) => {}
            }
        }
        input.block(p);
    }

    pub fn draw(&self, s: &Settings, top: f32, mouse: Vec2, audio_loaded: bool) {
        if !self.fader.visible() {
            return;
        }
        let f = self.fader.value();
        let (p, view, rows, content_h) = self.layout(s, top);
        panel(p, f);

        text_bold("Settings", p.x + PAD, baseline(p.y + 23.0, 17.0), 17.0, fade(TEXT, f));
        let resume = Rect::new(p.x + p.w - 96.0, p.y + 10.0, 84.0, 26.0);
        draw_button(resume, "Resume", resume.contains(mouse), true, f);
        draw_line(p.x + 1.0, view.y - 1.0, p.x + p.w - 1.0, view.y - 1.0, 1.0, fade(BORDER, f));

        clip(Some(view));
        for (row, r) in rows {
            if r.y + r.h < view.y || r.y > view.y + view.h {
                continue;
            }
            let hov = r.contains(mouse) && view.contains(mouse);
            match row {
                Row::Header(title) => {
                    section(r.x, r.y + 6.0, r.w, title, f);
                }
                Row::Slider(id) => {
                    let (spec, value) = slider_spec(id, s);
                    let dim = id == SliderId::Volume && !audio_loaded;
                    let st = &self.sliders[slider_index(id)];
                    draw_slider(
                        Rect::new(r.x, r.y, r.w, SLIDER_ROW_H),
                        value,
                        &spec,
                        hov || st.dragging,
                        if dim { f * 0.5 } else { f },
                    );
                }
                Row::Toggle(id) => {
                    let (label, on) = match id {
                        ToggleId::Trails => ("Motion trails  (T)", s.trails),
                        ToggleId::Shake => ("React to window moves  (W)", s.window_shake),
                        ToggleId::Dance => ("Objects jump on the beat", s.vis_dance),
                        ToggleId::Water => ("Water  (H)", s.water),
                        ToggleId::Effects => ("Sparks, dust and splashes", s.effects),
                        ToggleId::Sfx => ("Sound effects", s.sfx),
                        ToggleId::SlowMo => ("Slow motion on big hits", s.slow_motion),
                    };
                    draw_toggle_row(r, label, on, hov, f);
                }
                Row::Stepper(id) => {
                    let sr = Rect::new(r.x, r.y + 3.0, r.w, r.h - 8.0);
                    match id {
                        StepperId::Border => draw_stepper(sr, "Borders", s.border.label(), s.border.accent(), mouse, f),
                        StepperId::Background => {
                            draw_stepper(sr, "Background", s.background.label(), ACCENT_HI, mouse, f)
                        }
                        StepperId::VisBackground => {
                            draw_stepper(sr, "Behind objects  (V)", s.vis_background.label(), ACCENT_HI, mouse, f)
                        }
                        StepperId::WorldSize => {
                            let v = format!("×{}", s.world_size);
                            draw_stepper(sr, "World size", &v, ACCENT_HI, mouse, f)
                        }
                        StepperId::VisObject => {
                            draw_stepper(sr, "Object style", s.vis_object.label(), ACCENT_HI, mouse, f)
                        }
                    }
                }
                Row::Buttons(btns) => {
                    for (i, br) in button_rects(r, btns.len()).into_iter().enumerate() {
                        draw_button(br, btns[i].0, br.contains(mouse) && view.contains(mouse), false, f);
                    }
                }
            }
        }
        clip(None);

        // Scrollbar.
        if content_h > view.h {
            let ratio = view.h / content_h;
            let max_scroll = content_h - view.h;
            let bar_h = (view.h * ratio).max(24.0);
            let y = view.y + (view.h - bar_h) * (self.scroll / max_scroll);
            rrect(Rect::new(p.x + p.w - 6.0, y, 3.0, bar_h), 1.5, fade(BORDER_HI, f * 0.6));
        }
    }
}

fn button_rects(r: Rect, n: usize) -> Vec<Rect> {
    let gap = 8.0;
    let w = (r.w - gap * (n as f32 - 1.0)) / n as f32;
    (0..n).map(|i| Rect::new(r.x + i as f32 * (w + gap), r.y + 3.0, w, r.h - 8.0)).collect()
}

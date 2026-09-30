//! The playlist window (`pledit.bmp` + `pledit.txt`): the songs, with the
//! current one highlighted. Click to select (Ctrl / Shift to extend),
//! double-click or Enter to play, drag to reorder, Del to remove; the
//! bottom buttons add files, remove, select all, sort and clear.

use super::*;

const ROW_H: f32 = 13.0;
const LIST_TOP: f32 = 20.0;
const LIST_LEFT: f32 = 12.0;
const BOTTOM_H: f32 = 38.0;
const THUMB_H: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlButton {
    Add,
    Remove,
    Select,
    Sort,
    Clear,
    Close,
}

impl PlButton {
    const ALL: [PlButton; 6] =
        [PlButton::Add, PlButton::Remove, PlButton::Select, PlButton::Sort, PlButton::Clear, PlButton::Close];

    /// Where the button is in a window of `size` (skin px).
    pub fn rect(self, size: Vec2) -> Rect {
        let y = size.y - 30.0;
        match self {
            PlButton::Add => Rect::new(14.0, y, 25.0, 18.0),
            PlButton::Remove => Rect::new(43.0, y, 25.0, 18.0),
            PlButton::Select => Rect::new(72.0, y, 25.0, 18.0),
            PlButton::Sort => Rect::new(101.0, y, 25.0, 18.0),
            PlButton::Clear => Rect::new(size.x - 44.0, y, 25.0, 18.0),
            PlButton::Close => Rect::new(size.x - 11.0, 3.0, 9.0, 9.0),
        }
    }

    fn label(self) -> &'static str {
        match self {
            PlButton::Add => "+ADD",
            PlButton::Remove => "-REM",
            PlButton::Select => "ALL",
            PlButton::Sort => "SORT",
            PlButton::Clear => "CLR",
            PlButton::Close => "",
        }
    }
}

fn list_rect(size: Vec2) -> Rect {
    Rect::new(LIST_LEFT, LIST_TOP, size.x - LIST_LEFT - 20.0, size.y - LIST_TOP - BOTTOM_H)
}

fn visible_rows(size: Vec2) -> usize {
    (list_rect(size).h / ROW_H).floor().max(1.0) as usize
}

fn fmt_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

impl SkinPlayer {
    fn max_scroll(len: usize, size: Vec2) -> usize {
        len.saturating_sub(visible_rows(size))
    }

    pub(super) fn pl_scroll_by(&mut self, rows: i32, len: usize, size: Vec2) {
        let max = Self::max_scroll(len, size) as i32;
        self.pl_scroll = (self.pl_scroll as i32 + rows).clamp(0, max) as usize;
    }

    /// Scroll so the thumb's top is at `y` (window px).
    pub(super) fn pl_scroll_to_thumb(&mut self, y: f32, len: usize, size: Vec2) {
        let list = list_rect(size);
        let travel = (list.h - THUMB_H).max(1.0);
        let t = ((y - list.y) / travel).clamp(0.0, 1.0);
        self.pl_scroll = (t * Self::max_scroll(len, size) as f32).round() as usize;
    }

    fn thumb_y(&self, len: usize, size: Vec2) -> f32 {
        let list = list_rect(size);
        let max = Self::max_scroll(len, size);
        let t = if max == 0 { 0.0 } else { self.pl_scroll.min(max) as f32 / max as f32 };
        list.y + t * (list.h - THUMB_H)
    }

    /// Playlist entry under `local` (window px).
    pub(super) fn pl_row_at(&self, local: Vec2, len: usize, size: Vec2) -> Option<usize> {
        let list = list_rect(size);
        if !list.contains(local) {
            return None;
        }
        let i = self.pl_scroll + ((local.y - list.y) / ROW_H) as usize;
        (i < len).then_some(i)
    }

    pub(super) fn pl_press(
        &mut self,
        v: &PlayerView,
        size: Vec2,
        local: Vec2,
        input: &Input,
        grab: Vec2,
        actions: &mut Vec<Action>,
    ) {
        let len = v.playlist.len();
        let list = list_rect(size);
        let scrollbar = Rect::new(size.x - 15.0, list.y, 8.0, list.h);
        if let Some(b) = PlButton::ALL.into_iter().find(|b| b.rect(size).contains(local)) {
            self.pressed = Some(Press::Playlist(b));
        } else if scrollbar.contains(local) {
            let top = self.thumb_y(len, size);
            let grab = if (top..top + THUMB_H).contains(&local.y) { local.y - top } else { THUMB_H / 2.0 };
            self.pl_scroll_to_thumb(local.y - grab, len, size);
            self.drag = Some(Drag::PlScroll(grab));
        } else if let Some(i) = self.pl_row_at(local, len, size) {
            let now = get_time();
            let double = self.pl_last_click.is_some_and(|(row, t)| row == i && now - t < 0.35);
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if double {
                actions.push(Action::Player(PlayerCmd::PlayEntry(i)));
            } else if input.shift {
                let anchor = self.pl_last_click.map_or(i, |(row, _)| row.min(len.saturating_sub(1)));
                self.pl_selected = (anchor.min(i)..=anchor.max(i)).collect();
            } else if ctrl {
                if !self.pl_selected.remove(&i) {
                    self.pl_selected.insert(i);
                }
            } else {
                if !self.pl_selected.contains(&i) {
                    self.pl_selected = BTreeSet::from([i]);
                }
                self.drag = Some(Drag::PlRows(i));
            }
            if !input.shift {
                self.pl_last_click = Some((i, now));
            }
        } else if local.y < LIST_TOP {
            self.title_press(grab, actions);
        } else if list.contains(local) {
            self.pl_selected.clear();
        }
    }

    pub(super) fn pl_button(&mut self, b: PlButton, v: &PlayerView, actions: &mut Vec<Action>) {
        let cmd = |c| Action::Player(c);
        match b {
            PlButton::Add => actions.push(cmd(PlayerCmd::AddFiles)),
            PlButton::Remove => self.pl_remove_selected(actions),
            PlButton::Select => {
                let all = self.pl_selected.len() == v.playlist.len();
                self.pl_selected = if all { BTreeSet::new() } else { (0..v.playlist.len()).collect() };
            }
            PlButton::Sort => {
                self.pl_selected.clear();
                actions.push(cmd(PlayerCmd::SortPlaylist));
            }
            PlButton::Clear => {
                self.pl_selected.clear();
                self.pl_scroll = 0;
                actions.push(cmd(PlayerCmd::ClearPlaylist));
            }
            PlButton::Close => actions.push(cmd(PlayerCmd::TogglePlaylist)),
        }
    }

    fn pl_remove_selected(&mut self, actions: &mut Vec<Action>) {
        // From the end, so the indices of the others stay right.
        for &i in self.pl_selected.iter().rev() {
            actions.push(Action::Player(PlayerCmd::RemoveEntry(i)));
        }
        self.pl_selected.clear();
    }

    /// Keys while the pointer is over the playlist.
    pub(super) fn pl_keys(&mut self, v: &PlayerView, actions: &mut Vec<Action>) {
        if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
            self.pl_remove_selected(actions);
        }
        if is_key_pressed(KeyCode::Enter) {
            if let Some(&i) = self.pl_selected.first().filter(|&&i| i < v.playlist.len()) {
                actions.push(Action::Player(PlayerCmd::PlayEntry(i)));
            }
        }
    }

    pub(super) fn pl_draw(&self, v: &PlayerView, o: Vec2, size: Vec2, s: f32, local: Vec2) {
        let colors = v.skin.map_or_else(
            || PlaylistColors {
                normal: [150, 200, 255],
                current: [255, 255, 255],
                normal_bg: [8, 10, 16],
                selected_bg: [60, 50, 130],
            },
            |k| k.playlist,
        );
        match v.skin.filter(|k| k.has("pledit")) {
            Some(skin) => self.pl_frame_skin(skin, size, o, s, local, v.playlist.len()),
            None => self.pl_frame_builtin(size, o, s, local, v.playlist.len()),
        }
        let rgb = |c: [u8; 3]| Color::from_rgba(c[0], c[1], c[2], 255);
        let list = list_rect(size);
        let lr = Rect::new(o.x + list.x * s, o.y + list.y * s, list.w * s, list.h * s);
        draw_rectangle(lr.x, lr.y, lr.w, lr.h, rgb(colors.normal_bg));
        let current = v.now.and_then(|n| n.index);
        let dragging = matches!(self.drag, Some(Drag::PlRows(_)));
        let target = dragging.then(|| self.pl_row_at(local, v.playlist.len(), size)).flatten();
        super::super::widgets::clip(Some(lr));
        let size_px = 9.0 * s;
        for (k, i) in (self.pl_scroll..v.playlist.len()).take(visible_rows(size) + 1).enumerate() {
            let e = &v.playlist[i];
            let y = lr.y + k as f32 * ROW_H * s;
            if self.pl_selected.contains(&i) {
                draw_rectangle(lr.x, y, lr.w, ROW_H * s, rgb(colors.selected_bg));
            }
            if target == Some(i) {
                draw_rectangle(lr.x, y + (ROW_H - 1.0) * s, lr.w, s, rgb(colors.current));
            }
            let col = rgb(if current == Some(i) { colors.current } else { colors.normal });
            let base = y + ROW_H * s * 0.72;
            let time = e.duration.map(fmt_time).unwrap_or_default();
            let tw = measure(&time, size_px);
            text(&time, lr.x + lr.w - tw - 3.0 * s, base, size_px, col);
            let room = lr.w - tw - 10.0 * s;
            let mut label = format!("{}. {}", i + 1, e.title);
            if measure(&label, size_px) > room {
                while !label.is_empty() && measure(&format!("{label}…"), size_px) > room {
                    label.pop();
                }
                label.push('…');
            }
            text(&label, lr.x + 3.0 * s, base, size_px, col);
        }
        if v.playlist.is_empty() {
            let hint = "Drop music here or press +ADD";
            text_centered(hint, lr.x + lr.w / 2.0, lr.y + lr.h / 2.0, size_px, alpha(rgb(colors.normal), 0.6));
        }
        super::super::widgets::clip(None);

        // Total length of the playlist.
        let total: f64 = v.playlist.iter().filter_map(|e| e.duration).sum();
        let info = format!("{} songs  {}", v.playlist.len(), fmt_time(total));
        let (ix, iy) = (o.x + (size.x - 150.0 + 8.0) * s, o.y + (size.y - BOTTOM_H + 12.0) * s);
        if size.x >= 275.0 {
            text(&info, ix, iy, 7.0 * s, rgb(colors.normal));
        }
    }

    fn pl_held(&self, b: PlButton, size: Vec2, local: Vec2) -> bool {
        self.pressed == Some(Press::Playlist(b)) && b.rect(size).contains(local)
    }

    fn pl_frame_skin(&self, skin: &Skin, size: Vec2, o: Vec2, s: f32, local: Vec2, len: usize) {
        let sprite = |src: Rect, at: Vec2| skin.sprite("pledit", src, at, o, s);
        let (w, h) = (size.x, size.y);
        // Top: corners, tiles and the centred title.
        let mut x = 25.0;
        while x < w - 25.0 {
            sprite(Rect::new(127.0, 0.0, 25.0, 20.0), vec2(x, 0.0));
            x += 25.0;
        }
        sprite(Rect::new(0.0, 0.0, 25.0, 20.0), Vec2::ZERO);
        sprite(Rect::new(26.0, 0.0, 100.0, 20.0), vec2(((w - 100.0) / 2.0).round(), 0.0));
        sprite(Rect::new(153.0, 0.0, 25.0, 20.0), vec2(w - 25.0, 0.0));
        // Sides.
        let mut y = LIST_TOP;
        while y < h - BOTTOM_H {
            sprite(Rect::new(0.0, 42.0, 12.0, 29.0), vec2(0.0, y));
            sprite(Rect::new(31.0, 42.0, 20.0, 29.0), vec2(w - 20.0, y));
            y += 29.0;
        }
        // Bottom.
        let mut x = 125.0;
        while x < w - 150.0 {
            sprite(Rect::new(179.0, 0.0, 25.0, 38.0), vec2(x, h - BOTTOM_H));
            x += 25.0;
        }
        sprite(Rect::new(0.0, 72.0, 125.0, 38.0), vec2(0.0, h - BOTTOM_H));
        sprite(Rect::new(126.0, 72.0, 150.0, 38.0), vec2(w - 150.0, h - BOTTOM_H));
        if self.pl_held(PlButton::Close, size, local) {
            sprite(Rect::new(52.0, 42.0, 9.0, 9.0), PlButton::Close.rect(size).point());
        }
        let pressed = matches!(self.drag, Some(Drag::PlScroll(_)));
        sprite(
            Rect::new(if pressed { 61.0 } else { 52.0 }, 53.0, 8.0, THUMB_H),
            vec2(w - 15.0, self.thumb_y(len, size)),
        );
        // Pressed buttons darken (their pop-up menus are not reproduced).
        for b in PlButton::ALL.into_iter().filter(|&b| b != PlButton::Close && self.pl_held(b, size, local)) {
            let r = b.rect(size);
            draw_rectangle(o.x + r.x * s, o.y + r.y * s, r.w * s, r.h * s, Color::new(0.0, 0.0, 0.0, 0.35));
        }
    }

    fn pl_frame_builtin(&self, size: Vec2, o: Vec2, s: f32, local: Vec2, len: usize) {
        let r = |x: f32, y: f32, w: f32, h: f32| Rect::new(o.x + x * s, o.y + y * s, w * s, h * s);
        panel(r(0.0, 0.0, size.x, size.y), 1.0);
        text_centered("PLAYLIST", o.x + size.x / 2.0 * s, o.y + 11.0 * s, 8.0 * s, TEXT_DIM);
        let c = PlButton::Close.rect(size);
        let c = r(c.x, c.y, c.w, c.h);
        draw_line(c.x + 2.0, c.y + 2.0, c.x + c.w - 2.0, c.y + c.h - 2.0, s, TEXT_MUTED);
        draw_line(c.x + c.w - 2.0, c.y + 2.0, c.x + 2.0, c.y + c.h - 2.0, s, TEXT_MUTED);
        for b in PlButton::ALL.into_iter().filter(|&b| b != PlButton::Close) {
            let br = b.rect(size);
            let rr = r(br.x, br.y, br.w, br.h);
            rrect(rr, 3.0 * s, if self.pl_held(b, size, local) { SURFACE_HI } else { SURFACE_2 });
            rrect_lines(rr, 3.0 * s, 1.0, BORDER);
            text_centered(b.label(), rr.x + rr.w / 2.0, rr.y + rr.h / 2.0 + s, 6.0 * s, TEXT);
        }
        let list = list_rect(size);
        let groove = r(size.x - 15.0, list.y, 8.0, list.h);
        rrect(groove, 3.0 * s, SURFACE_HI);
        let ty = self.thumb_y(len, size);
        rrect(r(size.x - 15.0, ty, 8.0, THUMB_H), 3.0 * s, TEXT_MUTED);
    }
}

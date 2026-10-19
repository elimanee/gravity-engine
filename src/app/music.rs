//! Sonic's music in the game mode: the zone's song (the endless run's
//! zone, or the act's), the boss's in his arena, invincibility, Super
//! Sonic's, the special stage's, and the jingles (act clear, 1-up, Game
//! Over, an emerald, the drowning countdown). It keeps quiet while the
//! player's own music plays. Songs are made once (see `audio::music`) and
//! kept.

use super::*;
use crate::audio::music::{self, Jingle, Song};
use crate::audio::out::{Output, Voice};

#[derive(Default)]
pub(super) struct Music {
    /// Opened the first time music plays.
    out: Option<Output>,
    tried: bool,
    songs: HashMap<Song, Vec<f32>>,
    playing: Option<(Song, Voice)>,
    /// A jingle playing: which, when it ends, and its voice.
    jingle: Option<(Jingle, f64, Option<Voice>)>,
    volume: f32,
}

impl Music {
    fn output(&mut self) -> Option<&Output> {
        if !self.tried {
            self.tried = true;
            self.out = Output::new();
        }
        self.out.as_ref()
    }

    fn stop(&mut self) {
        if let Some((_, voice)) = self.playing.take() {
            voice.stop();
        }
    }

    fn start(&mut self, song: Song, volume: f32) {
        self.stop();
        let samples = self.songs.entry(song).or_insert_with(|| music::song(song)).clone();
        let Some(out) = self.output() else { return };
        if let Some(voice) = out.play_loop(samples, 1, music::RATE, volume) {
            self.playing = Some((song, voice));
            self.volume = volume;
        }
    }

    /// Play `song` (or nothing) at `volume`; jingles hold the song back.
    fn update(&mut self, song: Option<Song>, volume: f32, now: f64) {
        if let Some((j, end, _)) = &self.jingle {
            if now < *end {
                return;
            }
            if j.ends_song() {
                self.stop();
            }
            self.end_jingle(volume);
        }
        match (song, &self.playing) {
            (None, Some(_)) => self.stop(),
            (Some(want), Some((playing, _))) if *playing == want => {
                if (volume - self.volume).abs() > 1e-3 {
                    if let Some((_, voice)) = &self.playing {
                        voice.set_volume(volume);
                    }
                    self.volume = volume;
                }
            }
            (Some(want), _) => self.start(want, volume),
            (None, None) => {}
        }
    }

    /// The jingle is over (or cut short): the song comes back.
    fn end_jingle(&mut self, volume: f32) {
        if let Some((_, _, Some(voice))) = self.jingle.take() {
            voice.stop();
        }
        if let Some((_, voice)) = &self.playing {
            voice.set_volume(volume);
            self.volume = volume;
        }
    }

    fn jingle(&mut self, j: Jingle, volume: f32, now: f64) {
        let samples = music::jingle(j);
        let secs = samples.len() as f64 / music::RATE as f64;
        if j.ends_song() {
            self.stop();
        } else if let Some((_, voice)) = &self.playing {
            voice.set_volume(0.0);
        }
        if let Some((_, _, Some(voice))) = self.jingle.take() {
            voice.stop();
        }
        let voice = self.output().and_then(|out| out.play_voice(samples, 1, music::RATE, volume));
        self.jingle = Some((j, now + secs, voice));
    }

    /// Stop jingle `j` if it is playing (a breath of air ends the countdown).
    fn cancel(&mut self, j: Jingle, volume: f32) {
        if self.jingle.as_ref().is_some_and(|(playing, _, _)| *playing == j) {
            self.end_jingle(volume);
        }
    }
}

impl App {
    /// The song for what is happening now, if any.
    fn wanted_song(&self) -> Option<Song> {
        if !self.s.sonic_music || self.s.music_volume <= 0.0 {
            return None;
        }
        // The player's own music comes first.
        if self.audio.now_playing().is_some_and(|n| n.playing) {
            return None;
        }
        if self.special.is_some() {
            return Some(Song::Special);
        }
        let g = self.game.as_ref()?;
        if matches!(g.phase, crate::game::Phase::Over(_)) {
            return None;
        }
        if self.sonic.as_ref().is_some_and(|r| r.sonic.super_form) {
            return Some(Song::Super);
        }
        if g.invincible > 0.0 {
            return Some(Song::Invincible);
        }
        if g.arena().is_some() {
            return Some(Song::Boss);
        }
        if let Some(song) = self.endless_song() {
            return Some(song);
        }
        use crate::library::acts::{Zone, ALL};
        Some(match self.act.and_then(|i| ALL.get(i)).map(|a| a.zone) {
            Some(Zone::Harbour) => Song::Harbour,
            _ => Song::Meadow,
        })
    }

    /// Once a frame: the right song, at the right volume.
    pub(super) fn update_music(&mut self) {
        let song = self.wanted_song();
        let volume = self.s.music_volume;
        self.music.update(song, volume, get_time());
    }

    pub(super) fn play_jingle(&mut self, j: Jingle) {
        if !self.s.sonic_music || self.s.music_volume <= 0.0 {
            return;
        }
        self.music.jingle(j, self.s.music_volume, get_time());
    }

    pub(super) fn cancel_jingle(&mut self, j: Jingle) {
        self.music.cancel(j, self.s.music_volume);
    }
}

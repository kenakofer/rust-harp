use crate::notes::{MidiNote, NoteVolume};
use crate::output_midi::MidiVelocityPair;

use midir::MidiOutputConnection;

pub struct MidiBackend {
    conn: Option<MidiOutputConnection>,
    pub main_channel: u8,
    pub bass_channel: u8,
}

impl MidiBackend {
    pub fn new(conn: Option<MidiOutputConnection>, main_channel: u8, bass_channel: u8) -> Self {
        Self {
            conn,
            main_channel,
            bass_channel,
        }
    }

    pub fn conn_mut(&mut self) -> Option<&mut MidiOutputConnection> {
        self.conn.as_mut()
    }

    pub fn is_available(&self) -> bool {
        self.conn.is_some()
    }

    pub fn send_note_on(&mut self, channel: u8, note: MidiNote, vel: u8) {
        if vel == 0 {
            self.send_note_off(channel, note);
            return;
        }
        if let Some(c) = self.conn.as_mut() {
            let on = 0x90 | (channel & 0x0F);
            let _ = c.send(&[on, note.0, vel]);
        }
    }

    pub fn send_note_off(&mut self, channel: u8, note: MidiNote) {
        if let Some(c) = self.conn.as_mut() {
            let off = 0x80 | (channel & 0x0F);
            let _ = c.send(&[off, note.0, 0]);
        }
    }

    pub fn play_note(&mut self, midi_note: MidiNote, volume: NoteVolume) {
        let pair = MidiVelocityPair::from_note_and_volume(midi_note, volume);

        if pair.main > 0 {
            self.send_note_on(self.main_channel, midi_note, pair.main);
        }
        if pair.bass > 0 {
            // Send an off to bass first to get a solid rearticulation
            self.send_note_off(self.bass_channel, midi_note);
            self.send_note_on(self.bass_channel, midi_note, pair.bass);
        }
    }

    pub fn stop_note(&mut self, midi_note: MidiNote) {
        self.send_note_off(self.main_channel, midi_note);
        self.send_note_off(self.bass_channel, midi_note);
    }
}

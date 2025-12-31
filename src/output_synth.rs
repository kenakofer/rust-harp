use crate::notes::{MidiNote, NoteVolume};
use crate::synth::SquareSynth;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};

#[derive(Debug, Clone, Copy)]
enum Msg {
    NoteOn(Option<crate::touch::PointerId>, MidiNote, NoteVolume),
    NoteOff(MidiNote),
    StopPointer(crate::touch::PointerId),
    Bend(crate::touch::PointerId, MidiNote),
    SetA4Tuning(u16),
}

pub struct SynthBackend {
    tx: Sender<Msg>,
    // Keep stream alive.
    _stream: cpal::Stream,
}

impl SynthBackend {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no default output device".to_string())?;

        let supported = device
            .default_output_config()
            .map_err(|e| format!("default_output_config: {e}"))?;

        let (tx, rx) = crossbeam_channel::unbounded();

        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;

        match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                let config: cpal::StreamConfig = supported.into();
                let stream = build_stream_f32(&device, &config, rx, sample_rate, channels)?;
                Ok(Self {
                    tx,
                    _stream: stream,
                })
            }
            cpal::SampleFormat::I16 => {
                let config: cpal::StreamConfig = supported.into();
                let stream = build_stream_i16(&device, &config, rx, sample_rate, channels)?;
                Ok(Self {
                    tx,
                    _stream: stream,
                })
            }
            other => Err(format!("unsupported sample format: {other:?}")),
        }
    }

    pub fn play_note(&self, midi_note: MidiNote, volume: NoteVolume) {
        let _ = self.tx.send(Msg::NoteOn(None, midi_note, volume));
    }

    pub fn play_pointer_note(
        &self,
        pointer: crate::touch::PointerId,
        midi_note: MidiNote,
        volume: NoteVolume,
    ) {
        let _ = self.tx.send(Msg::NoteOn(Some(pointer), midi_note, volume));
    }

    pub fn stop_pointer(&self, pointer: crate::touch::PointerId) {
        let _ = self.tx.send(Msg::StopPointer(pointer));
    }

    pub fn bend_note(&self, pointer: crate::touch::PointerId, midi_note: MidiNote) {
        let _ = self.tx.send(Msg::Bend(pointer, midi_note));
    }

    pub fn stop_note(&self, midi_note: MidiNote) {
        let _ = self.tx.send(Msg::NoteOff(midi_note));
    }

    pub fn set_a4_tuning_hz(&self, a4_tuning_hz: u16) {
        let _ = self.tx.send(Msg::SetA4Tuning(a4_tuning_hz));
    }
}

fn drain_msgs(rx: &Receiver<Msg>, synth: &mut SquareSynth) {
    crate::synth::drain_messages(
        || rx.try_recv().ok(),
        |m| match m {
            Msg::NoteOn(Some(pid), note, vol) => synth.pointer_note_on(pid, note, vol.0),
            Msg::NoteOn(None, note, vol) => synth.note_on(note, vol.0),
            Msg::NoteOff(note) => synth.note_off(note),
            Msg::StopPointer(pid) => synth.pointer_note_off(pid),
            Msg::Bend(pid, note) => synth.pointer_bend(pid, note),
            Msg::SetA4Tuning(a4) => synth.set_a4_tuning_hz(a4),
        },
    );
}

fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<Msg>,
    sample_rate: u32,
    channels: usize,
) -> Result<cpal::Stream, String> {
    let mut synth = SquareSynth::new(sample_rate.max(1));

    let err_fn = |e| log::error!("cpal stream error: {e}");

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                drain_msgs(&rx, &mut synth);
                synth.render_f32_interleaved(data, channels);
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("build_output_stream(f32): {e}"))?;

    stream.play().map_err(|e| format!("stream.play: {e}"))?;

    Ok(stream)
}

fn build_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<Msg>,
    sample_rate: u32,
    channels: usize,
) -> Result<cpal::Stream, String> {
    let mut synth = SquareSynth::new(sample_rate.max(1));

    let err_fn = |e| log::error!("cpal stream error: {e}");

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [i16], _| {
                drain_msgs(&rx, &mut synth);
                synth.render_i16_interleaved(data, channels);
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("build_output_stream(i16): {e}"))?;

    stream.play().map_err(|e| format!("stream.play: {e}"))?;

    Ok(stream)
}

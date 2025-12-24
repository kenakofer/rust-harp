use crate::notes::MidiNote;

#[derive(Clone, Copy, Debug)]
struct Voice {
    midi: MidiNote,
    start_sample: u64,
    stop_sample: Option<u64>,
    phase: f32,
    phase_inc: f32,
    amp0: f32,
    max_harmonic_odd: u32,
}

pub struct SquareSynth {
    sample_rate_hz: f32,
    a4_tuning_hz: f32,
    sample: u64,
    voices: Vec<Voice>,
}

impl SquareSynth {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self::with_tuning(sample_rate_hz, 440)
    }

    pub fn with_tuning(sample_rate_hz: u32, a4_tuning_hz: u16) -> Self {
        Self {
            sample_rate_hz: sample_rate_hz as f32,
            a4_tuning_hz: a4_tuning_hz.clamp(430, 450) as f32,
            sample: 0,
            voices: Vec::new(),
        }
    }

    pub fn a4_tuning_hz(&self) -> u16 {
        self.a4_tuning_hz.round() as u16
    }

    pub fn set_a4_tuning_hz(&mut self, a4_tuning_hz: u16) {
        self.a4_tuning_hz = a4_tuning_hz.clamp(430, 450) as f32;
    }

    pub fn note_on(&mut self, midi: MidiNote, volume_0_to_127: u8) {
        let freq_hz = midi_to_hz(midi.0 as f32, self.a4_tuning_hz);

        // Conservative headroom; we’ll also soft-limit after mixing.
        let amp0 = (volume_0_to_127 as f32 / 127.0) * 0.12;

        let phase_inc = (2.0 * std::f32::consts::PI * freq_hz) / self.sample_rate_hz;

        // Band-limit the square by only summing harmonics under Nyquist.
        // Limit upper harmonics to keep CPU bounded.
        let nyquist = self.sample_rate_hz * 0.5;
        let mut max_harmonic = (nyquist / freq_hz).floor() as u32;
        if max_harmonic < 1 {
            max_harmonic = 1;
        }
        if (max_harmonic & 1) == 0 {
            max_harmonic = max_harmonic.saturating_sub(1);
        }
        max_harmonic = max_harmonic.min(15); // 1..15 odd => at most 8 sines (CPU headroom)

        // Ensure the same note never has two instances playing at once.
        if let Some(v) = self.voices.iter_mut().find(|v| v.midi == midi) {
            *v = Voice {
                midi,
                start_sample: self.sample,
                stop_sample: None,
                phase: 0.0,
                phase_inc,
                amp0,
                max_harmonic_odd: max_harmonic,
            };
            return;
        }

        const MAX_VOICES: usize = 16;
        if self.voices.len() >= MAX_VOICES {
            self.voices.swap_remove(0);
        }

        self.voices.push(Voice {
            midi,
            start_sample: self.sample,
            stop_sample: None,
            phase: 0.0,
            phase_inc,
            amp0,
            max_harmonic_odd: max_harmonic,
        });
    }

    pub fn note_off(&mut self, midi: MidiNote) {
        for v in &mut self.voices {
            if v.midi == midi {
                v.stop_sample = Some(self.sample);
            }
        }
    }

    pub fn render_i16_mono(&mut self, out: &mut [i16]) {
        self.render_i16_interleaved(out, 1);
    }

    pub fn render_f32_mono(&mut self, out: &mut [f32]) {
        self.render_f32_interleaved(out, 1);
    }

    fn render_sample(&mut self) -> f32 {
        // Exponential decay time constant (seconds)
        const TAU_S: f32 = 0.35;
        const ATTACK_S: f32 = 0.004; // short ramp to prevent clicks
        const RELEASE_S: f32 = 0.10; // fade-to-silence on note_off
        const SILENCE: f32 = 1.0e-4;

        let mut acc = 0.0f32;
        for v in &mut self.voices {
            let age_s = (self.sample - v.start_sample) as f32 / self.sample_rate_hz;

            let attack = (age_s / ATTACK_S).min(1.0);
            let decay = (-age_s / TAU_S).exp();
            let release = match v.stop_sample {
                Some(ss) => {
                    let t = (self.sample.saturating_sub(ss)) as f32 / self.sample_rate_hz;
                    (1.0 - (t / RELEASE_S)).clamp(0.0, 1.0)
                }
                None => 1.0,
            };
            let env = attack * decay * release;

            // Band-limited square: sum odd harmonics under Nyquist.
            // square(t) = (4/pi) * Σ_{n odd} sin(n*phase)/n
            let mut sq = 0.0f32;
            let mut n = 1u32;
            while n <= v.max_harmonic_odd {
                sq += (n as f32 * v.phase).sin() / (n as f32);
                n += 2;
            }
            sq *= 4.0 / std::f32::consts::PI;

            acc += v.amp0 * env * sq;

            v.phase += v.phase_inc;
            if v.phase >= 2.0 * std::f32::consts::PI {
                v.phase -= 2.0 * std::f32::consts::PI;
            }
        }

        self.sample += 1;

        // Periodically prune finished voices.
        if (self.sample & 0xFF) == 0 {
            self.voices.retain(|v| {
                let age_s = (self.sample - v.start_sample) as f32 / self.sample_rate_hz;
                let decay = (-(age_s) / TAU_S).exp();

                let release = match v.stop_sample {
                    Some(ss) => {
                        let t = (self.sample.saturating_sub(ss)) as f32 / self.sample_rate_hz;
                        (1.0 - (t / RELEASE_S)).clamp(0.0, 1.0)
                    }
                    None => 1.0,
                };

                v.amp0 * decay * release > SILENCE
            });
        }

        // Cheap soft limiter to avoid harsh clipping when multiple voices overlap.
        acc / (1.0 + acc.abs())
    }

    pub fn render_i16_interleaved(&mut self, out: &mut [i16], channels: usize) {
        assert!(channels >= 1);
        assert!(out.len() % channels == 0);

        let frames = out.len() / channels;
        for frame in 0..frames {
            let s = (self.render_sample() * i16::MAX as f32) as i16;
            let base = frame * channels;
            for ch in 0..channels {
                out[base + ch] = s;
            }
        }
    }

    pub fn render_f32_interleaved(&mut self, out: &mut [f32], channels: usize) {
        assert!(channels >= 1);
        assert!(out.len() % channels == 0);

        let frames = out.len() / channels;
        for frame in 0..frames {
            let s = self.render_sample();
            let base = frame * channels;
            for ch in 0..channels {
                out[base + ch] = s;
            }
        }
    }
}

fn midi_to_hz(midi: f32, a4_tuning_hz: f32) -> f32 {
    a4_tuning_hz * (2.0f32).powf((midi - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_synth_note_on_produces_audio_i16() {
        let mut s = SquareSynth::new(48_000);
        s.note_on(MidiNote(69), 100); // A4

        let mut buf = [0i16; 512];
        s.render_i16_mono(&mut buf);

        assert!(buf.iter().any(|&x| x != 0));
    }

    #[test]
    fn square_synth_note_on_produces_audio_f32() {
        let mut s = SquareSynth::new(48_000);
        s.note_on(MidiNote(69), 100); // A4

        let mut buf = [0.0f32; 512];
        s.render_f32_mono(&mut buf);

        assert!(buf.iter().any(|&x| x != 0.0));
        assert!(buf.iter().all(|&x| x.abs() <= 1.0));
    }

    #[test]
    fn note_off_fades_to_silence() {
        let mut s = SquareSynth::new(48_000);
        s.note_on(MidiNote(69), 100);

        let mut warmup = [0.0f32; 256];
        s.render_f32_mono(&mut warmup);

        s.note_off(MidiNote(69));

        // 100ms release @ 48kHz is 4800 samples; render a bit more to ensure we hit silence.
        let mut buf = [0.0f32; 6000];
        s.render_f32_mono(&mut buf);

        let tail_max = buf[5500..].iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        assert!(tail_max < 1.0e-3, "expected near-silence, got tail_max={tail_max}");
    }

    #[test]
    fn tuning_a4_is_clamped_and_settable() {
        let mut s = SquareSynth::with_tuning(48_000, 432);
        assert_eq!(s.a4_tuning_hz(), 432);

        s.set_a4_tuning_hz(450);
        assert_eq!(s.a4_tuning_hz(), 450);

        s.set_a4_tuning_hz(1000);
        assert_eq!(s.a4_tuning_hz(), 450);
    }
}

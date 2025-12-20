use crate::notes::MidiNote;

#[derive(Clone, Copy, Debug)]
struct Voice {
    freq_hz: f32,
    start_sample: u64,
    phase: f32,
    phase_inc: f32,
    amp0: f32,
}

pub struct SquareSynth {
    sample_rate_hz: f32,
    sample: u64,
    voices: Vec<Voice>,
}

impl SquareSynth {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self {
            sample_rate_hz: sample_rate_hz as f32,
            sample: 0,
            voices: Vec::new(),
        }
    }

    pub fn note_on(&mut self, midi: MidiNote, volume_0_to_127: u8) {
        let freq_hz = midi_to_hz(midi.0 as f32);
        let amp0 = (volume_0_to_127 as f32 / 127.0) * 0.2; // conservative
        let phase_inc = (2.0 * std::f32::consts::PI * freq_hz) / self.sample_rate_hz;
        self.voices.push(Voice {
            freq_hz,
            start_sample: self.sample,
            phase: 0.0,
            phase_inc,
            amp0,
        });
    }

    pub fn render_i16_mono(&mut self, out: &mut [i16]) {
        // Exponential decay time constant (seconds)
        const TAU_S: f32 = 0.35;
        const SILENCE: f32 = 1.0e-4;

        for o in out.iter_mut() {
            let t_s = self.sample as f32 / self.sample_rate_hz;

            let mut acc = 0.0f32;
            for v in &mut self.voices {
                let age_s = (self.sample - v.start_sample) as f32 / self.sample_rate_hz;
                let env = (-age_s / TAU_S).exp();

                // square from phase
                let sq = if v.phase <= std::f32::consts::PI {
                    1.0
                } else {
                    -1.0
                };

                acc += v.amp0 * env * sq;

                v.phase += v.phase_inc;
                if v.phase >= 2.0 * std::f32::consts::PI {
                    v.phase -= 2.0 * std::f32::consts::PI;
                }
            }

            // Soft clamp to avoid harsh clipping when multiple voices overlap.
            acc = acc.clamp(-1.0, 1.0);
            *o = (acc * i16::MAX as f32) as i16;

            self.sample += 1;

            // Periodically prune finished voices.
            // (Doing it per-sample would be wasteful.)
            if (self.sample & 0xFF) == 0 {
                self.voices.retain(|v| {
                    let age_s = (self.sample - v.start_sample) as f32 / self.sample_rate_hz;
                    v.amp0 * (-age_s / TAU_S).exp() > SILENCE
                });
            }

            let _ = t_s; // keeps structure readable if we tweak later
        }
    }
}

fn midi_to_hz(midi: f32) -> f32 {
    440.0 * (2.0f32).powf((midi - 69.0) / 12.0)
}

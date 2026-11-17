// Nokia Smart Messaging ringing tones (OTT, Sound.FORMAT_TONE) and plain tones, rendered to square-wave PCM
// the way a phone buzzer plays them.

use alloc::vec::Vec;

pub const SAMPLE_RATE: u32 = 22050;

#[derive(Clone, Debug, PartialEq)]
pub struct Tone {
    // 0 is a pause.
    pub frequency: f32,
    pub duration_ms: u32,
    // How long of `duration_ms` the tone sounds (the style: natural notes leave a short gap).
    pub sounding_ms: u32,
    // 0..=15, the ringing tone volume.
    pub volume: u8,
}

const COMMAND_RINGING_TONE_PROGRAMMING: u32 = 0b0100101;
const COMMAND_UNICODE: u32 = 0b0100010;
const COMMAND_SOUND: u32 = 0b0011101;
const SONG_TYPE_BASIC: u32 = 0b001;

const INSTRUCTION_NOTE: u32 = 0b001;
const INSTRUCTION_SCALE: u32 = 0b010;
const INSTRUCTION_STYLE: u32 = 0b011;
const INSTRUCTION_TEMPO: u32 = 0b100;
const INSTRUCTION_VOLUME: u32 = 0b101;

const TEMPOS_BPM: [u32; 32] = [
    25, 28, 31, 35, 40, 45, 50, 56, 63, 70, 80, 90, 100, 112, 125, 140, 160, 180, 200, 225, 250, 285, 320, 355, 400, 450, 500, 565, 635, 715, 800,
    900,
];
const DEFAULT_TEMPO_BPM: u32 = 63;
const DEFAULT_SCALE: u32 = 1; // scale-2, A = 880 Hz
const DEFAULT_VOLUME: u8 = 7;
const STYLE_NATURAL: u32 = 0;
const STYLE_STACCATO: u32 = 2;

// 2^(n/12) for the notes C..B relative to A.
const SEMITONES_FROM_A: [f32; 12] = [
    0.594_603_6,                      // C
    0.629_960_5,                      // C#
    0.667_419_9,                      // D
    core::f32::consts::FRAC_1_SQRT_2, // D#
    0.749_153_5,                      // E
    0.793_700_5,                      // F
    0.840_896_4,                      // F#
    0.890_898_7,                      // G
    0.943_874_3,                      // G#
    1.0,                              // A
    1.059_463_1,                      // A#
    1.122_462,                        // B
];

struct BitReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl BitReader<'_> {
    fn read(&mut self, bits: usize) -> Option<u32> {
        let mut value = 0;
        for _ in 0..bits {
            let byte = *self.data.get(self.position / 8)?;
            let bit = (byte >> (7 - self.position % 8)) & 1;
            value = (value << 1) | bit as u32;
            self.position += 1;
        }
        Some(value)
    }

    fn align(&mut self) {
        self.position = self.position.div_ceil(8) * 8;
    }
}

// Parses a ringing tone: the basic song of the sound command (temporary songs and other commands are skipped).
pub fn parse_ott(data: &[u8]) -> Option<Vec<Tone>> {
    let mut reader = BitReader { data, position: 0 };
    let command_count = reader.read(8)?;

    for _ in 0..command_count {
        match reader.read(7)? {
            COMMAND_RINGING_TONE_PROGRAMMING => reader.align(),
            COMMAND_UNICODE => {}
            COMMAND_SOUND => return parse_song(&mut reader),
            _ => return None,
        }
    }
    None
}

fn parse_song(reader: &mut BitReader) -> Option<Vec<Tone>> {
    if reader.read(3)? != SONG_TYPE_BASIC {
        return None;
    }
    let title_length = reader.read(4)? as usize;
    reader.read(8 * title_length)?;

    let mut tones = Vec::new();
    let mut tempo = DEFAULT_TEMPO_BPM;
    let mut scale = DEFAULT_SCALE;
    let mut style = STYLE_NATURAL;
    let mut volume = DEFAULT_VOLUME;

    let pattern_count = reader.read(8)?;
    for _ in 0..pattern_count {
        let _pattern_header = reader.read(3)?;
        let _pattern_id = reader.read(2)?;
        let loops = reader.read(4)?;
        let instruction_count = reader.read(8)?;

        let pattern_start = tones.len();
        for _ in 0..instruction_count {
            match reader.read(3)? {
                INSTRUCTION_NOTE => {
                    let note = reader.read(4)?;
                    let duration = reader.read(3)?;
                    let specifier = reader.read(2)?;

                    // A whole note is four beats; durations 0..=5 are 1, 1/2 ... 1/32.
                    let whole_ms = 4 * 60_000 / tempo;
                    let base = whole_ms >> duration.min(5);
                    let duration_ms = match specifier {
                        1 => base * 3 / 2,
                        2 => base * 7 / 4,
                        3 => base * 2 / 3,
                        _ => base,
                    };
                    let sounding_ms = match style {
                        STYLE_NATURAL => duration_ms * 15 / 16,
                        STYLE_STACCATO => duration_ms / 2,
                        _ => duration_ms,
                    };
                    let frequency = match note {
                        1..=12 => 440.0 * (1u32 << scale) as f32 * SEMITONES_FROM_A[note as usize - 1],
                        _ => 0.0,
                    };

                    tones.push(Tone {
                        frequency,
                        duration_ms,
                        sounding_ms,
                        volume,
                    });
                }
                INSTRUCTION_SCALE => scale = reader.read(2)?,
                INSTRUCTION_STYLE => style = reader.read(2)?,
                INSTRUCTION_TEMPO => tempo = TEMPOS_BPM[reader.read(5)? as usize],
                INSTRUCTION_VOLUME => volume = reader.read(4)? as u8,
                _ => return None,
            }
        }

        // 15 repeats forever on a phone; once is enough for a sound effect.
        if loops != 15 {
            let pattern = tones[pattern_start..].to_vec();
            for _ in 0..loops {
                tones.extend_from_slice(&pattern);
            }
        }
    }

    Some(tones)
}

// A tone of `frequency` Hz (Sound(int freq, long duration)); 0 is silence.
pub fn plain_tone(frequency: i32, duration_ms: i64) -> Vec<Tone> {
    let duration_ms = duration_ms.clamp(0, 60_000) as u32;
    let mut tone = Tone {
        frequency: frequency.max(0) as f32,
        duration_ms,
        sounding_ms: duration_ms,
        volume: 15,
    };
    if tone.frequency > (SAMPLE_RATE / 2) as f32 {
        tone.frequency = 0.0;
    }
    [tone].to_vec()
}

pub fn duration_ms(tones: &[Tone]) -> u64 {
    tones.iter().map(|tone| tone.duration_ms as u64).sum()
}

// Mono 16-bit PCM at SAMPLE_RATE. `gain` is Sound.setGain's 0..=255.
pub fn render(tones: &[Tone], gain: i32) -> Vec<i16> {
    const FULL_AMPLITUDE: f32 = 0.75 * i16::MAX as f32;
    const RAMP_SAMPLES: usize = (SAMPLE_RATE / 500) as usize; // 2 ms against clicks

    let mut samples = Vec::with_capacity((duration_ms(tones) * SAMPLE_RATE as u64 / 1000) as usize);
    let gain = gain.clamp(0, 255) as f32 / 255.0;

    for tone in tones {
        let total = (tone.duration_ms as u64 * SAMPLE_RATE as u64 / 1000) as usize;
        let sounding = ((tone.sounding_ms as u64 * SAMPLE_RATE as u64 / 1000) as usize).min(total);
        let amplitude = FULL_AMPLITUDE * gain * tone.volume.min(15) as f32 / 15.0;

        if tone.frequency <= 0.0 || amplitude == 0.0 {
            samples.resize(samples.len() + total, 0);
            continue;
        }

        let step = tone.frequency / SAMPLE_RATE as f32;
        let mut phase = 0.0f32;
        for i in 0..sounding {
            let ramp = (i.min(sounding - 1 - i) as f32 / RAMP_SAMPLES as f32).min(1.0);
            let level = if phase < 0.5 { amplitude } else { -amplitude };
            samples.push((level * ramp) as i16);
            phase += step;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }
        samples.resize(samples.len() + total - sounding, 0);
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::{Tone, duration_ms, parse_ott, render};

    // C (scale 3) 1/8, G (scale 2) 1/16, C (scale 3) 1/8 at 125 bpm, natural style.
    const SHORT_TUNE: [u8; 16] = [
        0x02, 0x4a, 0x3a, 0x40, 0x04, 0x00, 0x11, 0x1c, 0xc5, 0x11, 0x62, 0x4c, 0x41, 0x44, 0x58, 0x00,
    ];

    #[test]
    fn parses_a_basic_song() {
        let tones = parse_ott(&SHORT_TUNE).unwrap();

        assert_eq!(tones.len(), 3);
        assert_eq!(
            tones
                .iter()
                .map(|tone| (tone.duration_ms, tone.sounding_ms, tone.volume))
                .collect::<alloc::vec::Vec<_>>(),
            [(240, 225, 7), (120, 112, 7), (240, 225, 7)]
        );
        assert!((tones[0].frequency - 1046.5).abs() < 0.5);
        assert!((tones[1].frequency - 783.99).abs() < 0.5);
        assert_eq!(duration_ms(&tones), 600);
    }

    #[test]
    fn rejects_other_data() {
        assert_eq!(parse_ott(&[]), None);
        assert_eq!(parse_ott(b"RIFF\0\0\0\0WAVE"), None);
    }

    #[test]
    fn renders_tones_and_pauses() {
        let tones = [
            Tone {
                frequency: 1000.0,
                duration_ms: 10,
                sounding_ms: 5,
                volume: 15,
            },
            Tone {
                frequency: 0.0,
                duration_ms: 10,
                sounding_ms: 10,
                volume: 15,
            },
        ];
        let samples = render(&tones, 255);

        assert_eq!(samples.len(), 2 * 220);
        assert!(samples[..110].iter().any(|&sample| sample > 10_000));
        assert!(samples[110..].iter().all(|&sample| sample == 0));
        assert!(render(&tones, 0).iter().all(|&sample| sample == 0));
    }
}

// MMAPI tones (Manager.playTone, ToneControl.setSequence) as MIDI events of the system audio.

use alloc::{vec, vec::Vec};

use wie_backend::{AudioEventData, AudioSequence, TimedAudioEvent};

// ToneControl
const VERSION: i8 = -2;
const TEMPO: i8 = -3;
const RESOLUTION: i8 = -4;
const BLOCK_START: i8 = -5;
const BLOCK_END: i8 = -6;
const PLAY_BLOCK: i8 = -7;
const SET_VOLUME: i8 = -8;
const REPEAT: i8 = -9;
const SILENCE: i8 = -1;

const DEFAULT_TEMPO: i32 = 30;
const DEFAULT_RESOLUTION: i32 = 64;
// Square wave: the sound of the phone tone generators.
const TONE_PROGRAM: u8 = 80;
// A sequence that plays its blocks over and over in itself is cut here.
const MAX_EVENTS: usize = 100_000;

struct Tones {
    events: Vec<TimedAudioEvent>,
    time: u64,
    velocity: u8,
}

impl Tones {
    fn new(volume: i32) -> Self {
        Self {
            events: vec![TimedAudioEvent {
                time: 0,
                data: AudioEventData::Midi(vec![0xc0, TONE_PROGRAM]),
            }],
            time: 0,
            velocity: velocity(volume),
        }
    }

    fn note(&mut self, note: u8, duration_ms: u64) {
        if self.events.len() >= MAX_EVENTS {
            return;
        }
        self.events.push(TimedAudioEvent {
            time: self.time,
            data: AudioEventData::Midi(vec![0x90, note, self.velocity]),
        });
        self.time += duration_ms;
        self.events.push(TimedAudioEvent {
            time: self.time,
            data: AudioEventData::Midi(vec![0x80, note, 0]),
        });
    }

    fn silence(&mut self, duration_ms: u64) {
        self.time += duration_ms;
    }

    fn into_sequence(self) -> AudioSequence {
        AudioSequence {
            duration: self.time,
            events: self.events,
        }
    }
}

fn velocity(volume: i32) -> u8 {
    (volume.clamp(0, 100) * 127 / 100) as u8
}

// Manager.playTone: one note (0..=127, C4 = 60) for duration_ms at volume 0..=100.
pub fn single_tone(note: u8, duration_ms: u64, volume: i32) -> AudioSequence {
    let mut tones = Tones::new(volume);
    tones.note(note, duration_ms);
    tones.into_sequence()
}

// ToneControl.setSequence: VERSION 1, optional TEMPO and RESOLUTION, block definitions, then the events to play.
// None for a sequence that is not well formed.
pub fn parse_tone_sequence(data: &[i8]) -> Option<AudioSequence> {
    if data.get(0..2) != Some(&[VERSION, 1][..]) {
        return None;
    }

    let mut tempo = DEFAULT_TEMPO;
    let mut resolution = DEFAULT_RESOLUTION;
    let mut index = 2;
    loop {
        match data.get(index..index + 2) {
            Some([TEMPO, value]) if (5..=127).contains(value) => tempo = *value as i32,
            Some([RESOLUTION, value]) if (1..=127).contains(value) => resolution = *value as i32,
            _ => break,
        }
        index += 2;
    }

    // Blocks: number -> the range of their events.
    let mut blocks: [Option<(usize, usize)>; 128] = [None; 128];
    while data.get(index) == Some(&BLOCK_START) {
        let number = *data.get(index + 1)?;
        if number < 0 {
            return None;
        }
        let start = index + 2;
        let mut end = start;
        while data.get(end..end + 2) != Some(&[BLOCK_END, number][..]) {
            if end >= data.len() {
                return None;
            }
            end += 2;
        }
        blocks[number as usize] = Some((start, end));
        index = end + 2;
    }

    // A note of `duration` units of 1/resolution of a whole note, at `tempo` * 4 quarter notes per minute.
    let duration_ms = |duration: i8| -> Option<u64> {
        if duration <= 0 {
            return None;
        }
        Some(duration as u64 * 240_000 / (tempo as u64 * 4 * resolution as u64))
    };

    let mut tones = Tones::new(100);
    play_events(data, index, data.len(), &blocks, &duration_ms, &mut tones, 0)?;

    Some(tones.into_sequence())
}

fn play_events(
    data: &[i8],
    start: usize,
    end: usize,
    blocks: &[Option<(usize, usize)>; 128],
    duration_ms: &dyn Fn(i8) -> Option<u64>,
    tones: &mut Tones,
    depth: usize,
) -> Option<()> {
    // Blocks may not play themselves; a sequence that does is not well formed.
    if depth > blocks.len() {
        return None;
    }

    let mut index = start;
    while index < end {
        let event = *data.get(index)?;
        let value = *data.get(index + 1)?;
        match event {
            PLAY_BLOCK => {
                let (block_start, block_end) = (*blocks.get(usize::try_from(value).ok()?)?)?;
                play_events(data, block_start, block_end, blocks, duration_ms, tones, depth + 1)?;
            }
            SET_VOLUME => tones.velocity = velocity(value as i32),
            REPEAT => {
                let note = *data.get(index + 2)?;
                let duration = duration_ms(*data.get(index + 3)?)?;
                for _ in 0..value.max(2) {
                    play_note(tones, note, duration)?;
                }
                index += 2;
            }
            _ => play_note(tones, event, duration_ms(value)?)?,
        }
        index += 2;
    }

    Some(())
}

fn play_note(tones: &mut Tones, note: i8, duration_ms: u64) -> Option<()> {
    match note {
        SILENCE => tones.silence(duration_ms),
        0.. => tones.note(note as u8, duration_ms),
        _ => return None,
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use wie_backend::AudioEventData;

    use super::{parse_tone_sequence, single_tone};

    fn notes(sequence: &wie_backend::AudioSequence) -> Vec<(u64, u8)> {
        sequence
            .events
            .iter()
            .filter_map(|event| match &event.data {
                AudioEventData::Midi(data) if data[0] == 0x90 => Some((event.time, data[1])),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tone_sequences_play_blocks_repeats_and_silences() {
        // Tempo 30 (120 bpm), resolution 64: a duration of 16 is a quarter note, 500 ms.
        let sequence = parse_tone_sequence(&[
            -2, 1, -3, 30, -4, 64, // version, tempo, resolution
            -5, 0, 60, 16, -1, 16, -6, 0, // block 0: C4, silence
            -7, 0, -8, 50, -9, 3, 62, 8, // play block 0, volume 50, D4 three times for 250 ms
        ])
        .unwrap();

        assert_eq!(notes(&sequence), [(0, 60), (1000, 62), (1250, 62), (1500, 62)]);
        assert_eq!(sequence.duration, 1750);

        assert!(parse_tone_sequence(&[-2, 1, -7, 0]).is_none());
        assert!(parse_tone_sequence(&[-2, 1, -5, 0, 60, 16, -7, 0, -6, 0, -7, 0]).is_none());
        assert!(parse_tone_sequence(&[60, 16]).is_none());
    }

    #[test]
    fn single_tones_last_their_duration() {
        let sequence = single_tone(69, 200, 100);

        assert_eq!(notes(&sequence), [(0, 69)]);
        assert_eq!(sequence.duration, 200);
    }
}

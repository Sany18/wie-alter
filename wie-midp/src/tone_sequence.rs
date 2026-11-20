// MMAPI tones (Manager.playTone, ToneControl.setSequence, iMelody data) as MIDI events of the system audio.

use alloc::{string::String, vec, vec::Vec};

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

// iMelody (BEGIN:IMELODY ... END:IMELODY): the BEAT and VOLUME headers and the MELODY line of notes ("*5#c3."),
// rests ("r2"), volume changes ("V+", "V-", "V9") and repeats ("(...@2)"); light and vibration marks are skipped.
// None for data that is not iMelody.
pub fn parse_imelody(data: &[u8]) -> Option<AudioSequence> {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines().map(str::trim);
    if !lines.next()?.eq_ignore_ascii_case("BEGIN:IMELODY") {
        return None;
    }

    let mut beat = 120;
    let mut volume = 7;
    let mut melody = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        match name.to_ascii_uppercase().as_str() {
            "BEAT" => beat = value.trim().parse::<u64>().ok().filter(|beat| (25..=900).contains(beat))?,
            "VOLUME" => volume = value.trim().trim_start_matches(['V', 'v']).parse::<i32>().unwrap_or(7),
            "MELODY" => melody = Some(value.trim().to_ascii_lowercase()),
            _ => {}
        }
    }

    let melody = melody?.into_bytes();
    let mut tones = Tones::new(imelody_volume(volume));
    let whole_note_ms = 240_000 / beat;
    imelody_events(&melody, whole_note_ms, &mut MelodyState { octave: 4, volume }, &mut tones, 0)?;

    Some(tones.into_sequence())
}

// iMelody volumes are 0..=15.
fn imelody_volume(volume: i32) -> i32 {
    volume.clamp(0, 15) * 100 / 15
}

// Plays the melody from `index` to its end or to the end of the repeat it is in; returns where it stopped.
// The octave and volume a melody sets last, also inside and after repeats.
struct MelodyState {
    octave: i32,
    volume: i32,
}

fn imelody_events(melody: &[u8], whole_note_ms: u64, state: &mut MelodyState, tones: &mut Tones, mut index: usize) -> Option<usize> {
    const MARKS: [&[u8]; 6] = [b"ledon", b"ledoff", b"vibeon", b"vibeoff", b"backon", b"backoff"];

    while index < melody.len() {
        let rest = &melody[index..];
        if let Some(mark) = MARKS.iter().find(|mark| rest.starts_with(mark)) {
            index += mark.len();
            continue;
        }
        match rest[0] {
            b' ' | b'\t' => index += 1,
            b'(' => {
                let start = index + 1;
                let end = imelody_events(melody, whole_note_ms, state, tones, start)?;
                // "@n)": n times in all, 0 for ever (taken as twice here, the sequence has an end).
                let count_start = end + 1;
                let count_end = count_start + melody[count_start..].iter().take_while(|c| c.is_ascii_digit()).count();
                let count: usize = core::str::from_utf8(&melody[count_start..count_end]).ok()?.parse().ok()?;
                for _ in 1..count.clamp(2, 16) {
                    imelody_events(melody, whole_note_ms, state, tones, start)?;
                }
                index = count_end;
                if melody.get(index) == Some(&b')') {
                    index += 1;
                }
            }
            b'@' => return Some(index),
            b')' => index += 1,
            b'*' => {
                state.octave = (*melody.get(index + 1)? as char).to_digit(10)? as i32;
                index += 2;
            }
            b'v' => {
                index += 1;
                match melody.get(index) {
                    Some(b'+') => {
                        state.volume = (state.volume + 1).min(15);
                        index += 1;
                    }
                    Some(b'-') => {
                        state.volume = (state.volume - 1).max(0);
                        index += 1;
                    }
                    _ => {
                        let digits = melody[index..].iter().take_while(|c| c.is_ascii_digit()).count();
                        state.volume = core::str::from_utf8(&melody[index..index + digits]).ok()?.parse().ok()?;
                        index += digits;
                    }
                }
                tones.velocity = velocity(imelody_volume(state.volume));
            }
            _ => {
                let mut shift = 0;
                match rest[0] {
                    b'#' => shift = 1,
                    b'&' => shift = -1,
                    _ => {}
                }
                if shift != 0 {
                    index += 1;
                }
                let name = *melody.get(index)?;
                let semitone = match name {
                    b'c' => Some(0),
                    b'd' => Some(2),
                    b'e' => Some(4),
                    b'f' => Some(5),
                    b'g' => Some(7),
                    b'a' => Some(9),
                    b'b' => Some(11),
                    b'r' => None,
                    _ => return None,
                };
                let duration = (*melody.get(index + 1)? as char).to_digit(10).filter(|duration| *duration <= 5)?;
                index += 2;
                let mut duration_ms = whole_note_ms >> duration;
                match melody.get(index) {
                    Some(b'.') => duration_ms = duration_ms * 3 / 2,
                    Some(b':') => duration_ms = duration_ms * 7 / 4,
                    Some(b';') => duration_ms = duration_ms * 2 / 3,
                    _ => index -= 1,
                }
                index += 1;
                match semitone {
                    // Octave *4 has a = 880 Hz.
                    Some(semitone) => tones.note((12 * (state.octave + 2) + semitone + shift).clamp(0, 127) as u8, duration_ms),
                    None => tones.silence(duration_ms),
                }
            }
        }
    }

    Some(index)
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use wie_backend::AudioEventData;

    use super::{parse_imelody, parse_tone_sequence, single_tone};

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

    #[test]
    fn imelody_plays_notes_rests_and_repeats() {
        // Beat 120: a quarter note (duration 2) is 500 ms.
        let data = b"BEGIN:IMELODY\r\nVERSION:1.2\r\nBEAT:120\r\nMELODY:*4a2 r2 *5(#c3@2) ledon &b2.\r\nEND:IMELODY\r\n";
        let sequence = parse_imelody(data).unwrap();

        assert_eq!(notes(&sequence), [(0, 81), (1000, 85), (1250, 85), (1500, 94)]);
        assert_eq!(sequence.duration, 2250);

        assert!(parse_imelody(b"MThd").is_none());
        assert!(parse_imelody(b"BEGIN:IMELODY\nMELODY:x2\nEND:IMELODY").is_none());
    }
}

use alloc::{vec, vec::Vec};

use crate::{AudioEventData, AudioSequence, TimedAudioEvent};

const DEFAULT_TEMPO: u64 = 500_000; // microseconds per quarter note: 120 bpm

enum Item {
    Midi(Vec<u8>),
    Tempo(u64),
}

/// A Standard MIDI File (format 0 or 1, the files of MIDP games) as one sequence of MIDI messages timed in milliseconds.
/// None when the data is not a MIDI file.
pub fn parse_standard_midi(data: &[u8]) -> Option<AudioSequence> {
    if data.len() < 14 || &data[0..4] != b"MThd" {
        return None;
    }
    let header_length = be32(&data[4..8])? as usize;
    let track_count = be16(&data[10..12])? as usize;
    let division = be16(&data[12..14])?;
    // ticks -> microseconds: by quarter note (and tempo), or SMPTE frames of a fixed length
    let smpte_tick_us = if division & 0x8000 != 0 {
        let fps = (256 - (division >> 8)) as u64;
        let ticks_per_frame = (division & 0xff).max(1) as u64;
        Some(1_000_000 / (fps.max(1) * ticks_per_frame))
    } else {
        None
    };
    let ticks_per_quarter = (division & 0x7fff).max(1) as u64;

    let mut items: Vec<(u64, usize, Item)> = Vec::new();
    let mut end_tick = 0;
    let mut position = 8 + header_length;
    let mut order = 0;
    for _ in 0..track_count {
        if position + 8 > data.len() {
            break;
        }
        let length = be32(&data[position + 4..position + 8])? as usize;
        let start = position + 8;
        let end = (start + length).min(data.len());
        if &data[position..position + 4] == b"MTrk" {
            end_tick = end_tick.max(parse_track(&data[start..end], &mut items, &mut order));
        }
        position = start + length;
    }
    if items.is_empty() {
        return None;
    }
    items.sort_by_key(|(tick, order, _)| (*tick, *order));

    let mut events = Vec::new();
    let (mut tempo, mut last_tick, mut time_us) = (DEFAULT_TEMPO, 0, 0u64);
    let tick_us = |ticks: u64, tempo: u64| smpte_tick_us.map_or(ticks * tempo / ticks_per_quarter, |us| ticks * us);
    for (tick, _, item) in items {
        time_us += tick_us(tick - last_tick, tempo);
        last_tick = tick;
        match item {
            Item::Tempo(value) => tempo = value,
            Item::Midi(message) => events.push(TimedAudioEvent {
                time: time_us / 1000,
                data: AudioEventData::Midi(message),
            }),
        }
    }
    time_us += tick_us(end_tick.max(last_tick) - last_tick, tempo);

    Some(AudioSequence {
        duration: time_us / 1000,
        events,
    })
}

/// The events of one track into `items` (tick, global order, item); returns the tick of the track's end.
fn parse_track(track: &[u8], items: &mut Vec<(u64, usize, Item)>, order: &mut usize) -> u64 {
    let (mut position, mut tick, mut status) = (0, 0u64, 0u8);
    while position < track.len() {
        let Some((delta, next)) = variable_length(track, position) else { break };
        tick += delta as u64;
        position = next;
        let Some(&first) = track.get(position) else { break };
        let event_status = if first & 0x80 != 0 {
            position += 1;
            first
        } else {
            status // running status: the data byte belongs to the previous status
        };
        match event_status {
            0xff => {
                let Some(&kind) = track.get(position) else { break };
                let Some((length, next)) = variable_length(track, position + 1) else {
                    break;
                };
                let body = track.get(next..next + length as usize).unwrap_or(&[]);
                position = next + length as usize;
                if kind == 0x51 && body.len() == 3 {
                    items.push((
                        tick,
                        *order,
                        Item::Tempo(((body[0] as u64) << 16) | ((body[1] as u64) << 8) | body[2] as u64),
                    ));
                    *order += 1;
                }
                if kind == 0x2f {
                    break;
                }
            }
            0xf0 | 0xf7 => {
                let Some((length, next)) = variable_length(track, position) else { break };
                let body = track.get(next..next + length as usize).unwrap_or(&[]);
                if event_status == 0xf0 {
                    let mut message = vec![0xf0];
                    message.extend_from_slice(body);
                    items.push((tick, *order, Item::Midi(message)));
                    *order += 1;
                }
                position = next + length as usize;
            }
            0x80..=0xef => {
                status = event_status;
                let size = if matches!(event_status & 0xf0, 0xc0 | 0xd0) { 1 } else { 2 };
                let Some(data) = track.get(position..position + size) else { break };
                let mut message = vec![event_status];
                message.extend_from_slice(data);
                position += size;
                items.push((tick, *order, Item::Midi(message)));
                *order += 1;
            }
            _ => break, // a data byte without a status before it: a broken track
        }
    }
    tick
}

fn variable_length(data: &[u8], mut position: usize) -> Option<(u32, usize)> {
    let mut value = 0u32;
    for _ in 0..4 {
        let byte = *data.get(position)?;
        position += 1;
        value = (value << 7) | (byte & 0x7f) as u32;
        if byte & 0x80 == 0 {
            return Some((value, position));
        }
    }
    None
}

fn be16(bytes: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.try_into().ok()?))
}

fn be32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::parse_standard_midi;
    use crate::AudioEventData;

    #[test]
    fn parses_tempo_running_status_and_tracks() {
        // format 1, two tracks, 96 ticks per quarter
        let mut file = vec![b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 1, 0, 2, 0, 96];
        // track 1: tempo 250000 us (240 bpm) at 0, end at 96
        let track1 = [0x00, 0xff, 0x51, 0x03, 0x03, 0xd0, 0x90, 0x60, 0xff, 0x2f, 0x00];
        // track 2: note on at 0, note on with running status at 96, note off at 192
        let track2 = [0x00, 0x90, 60, 100, 0x60, 64, 100, 0x60, 0x80, 60, 0, 0x00, 0xff, 0x2f, 0x00];
        for track in [&track1[..], &track2[..]] {
            file.extend_from_slice(b"MTrk");
            file.extend_from_slice(&(track.len() as u32).to_be_bytes());
            file.extend_from_slice(track);
        }

        let sequence = parse_standard_midi(&file).unwrap();
        let times: vec::Vec<u64> = sequence.events.iter().map(|e| e.time).collect();
        assert_eq!(times, [0, 250, 500], "a quarter note lasts 250 ms at 240 bpm");
        assert!(matches!(&sequence.events[1].data, AudioEventData::Midi(m) if m[..] == [0x90, 64, 100]));
        assert_eq!(sequence.duration, 500);
        assert!(parse_standard_midi(b"RIFF....WAVE").is_none());
    }
}

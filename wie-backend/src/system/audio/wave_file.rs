use alloc::vec;

use crate::{AudioEventData, AudioSequence, TimedAudioEvent};

/// A RIFF WAVE file with PCM samples (8-bit unsigned or 16-bit signed, mono or stereo) as one wave event.
/// None when the data is not such a file (compressed WAVE formats such as IMA ADPCM included).
pub fn parse_wave(data: &[u8]) -> Option<AudioSequence> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    let (mut format, mut samples) = (None, None);
    let mut position = 12;
    while position + 8 <= data.len() {
        let id = &data[position..position + 4];
        let length = u32::from_le_bytes(data[position + 4..position + 8].try_into().ok()?) as usize;
        let body = &data[position + 8..(position + 8 + length).min(data.len())];
        match id {
            b"fmt " if body.len() >= 16 => {
                let tag = u16::from_le_bytes([body[0], body[1]]);
                let channels = u16::from_le_bytes([body[2], body[3]]);
                let rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
                let bits = u16::from_le_bytes([body[14], body[15]]);
                format = Some((tag, channels, rate, bits));
            }
            b"data" => samples = Some(body),
            _ => {}
        }
        position += 8 + length + (length & 1);
    }
    let (tag, channels, sampling_rate, bits) = format?;
    let body = samples?;
    if tag != 1 || !(1..=2).contains(&channels) || sampling_rate == 0 {
        return None;
    }
    let samples: vec::Vec<i16> = match bits {
        8 => body.iter().map(|&b| ((b as i16) - 128) << 8).collect(),
        16 => body.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect(),
        _ => return None,
    };
    let duration = samples.len() as u64 * 1000 / (sampling_rate as u64 * channels as u64);

    Some(AudioSequence {
        duration,
        events: vec![TimedAudioEvent {
            time: 0,
            data: AudioEventData::Wave {
                channels: channels as u8,
                sampling_rate,
                samples,
            },
        }],
    })
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::parse_wave;
    use crate::AudioEventData;

    #[test]
    fn parses_8_bit_pcm() {
        let mut file: Vec<u8> = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        file.extend_from_slice(&16u32.to_le_bytes());
        file.extend_from_slice(&[1, 0, 1, 0]); // PCM, mono
        file.extend_from_slice(&8000u32.to_le_bytes());
        file.extend_from_slice(&8000u32.to_le_bytes());
        file.extend_from_slice(&[1, 0, 8, 0]);
        file.extend_from_slice(b"data");
        file.extend_from_slice(&4u32.to_le_bytes());
        file.extend_from_slice(&[128, 255, 0, 128]);

        let sequence = parse_wave(&file).unwrap();
        let AudioEventData::Wave {
            channels,
            sampling_rate,
            samples,
        } = &sequence.events[0].data
        else {
            panic!("a wave event")
        };
        assert_eq!((*channels, *sampling_rate), (1, 8000));
        assert_eq!(samples[..], [0, 127 << 8, -128 << 8, 0]);
        assert!(parse_wave(b"MThd").is_none());
    }
}

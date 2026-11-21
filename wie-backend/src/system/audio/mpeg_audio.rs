use alloc::vec;

use nanomp3::{Channels, decode_all};

use crate::{AudioEventData, AudioSequence, TimedAudioEvent};

/// MPEG audio (MP3) as one wave event of its decoded samples. None when the data is not MPEG audio.
pub fn parse_mpeg_audio(data: &[u8]) -> Option<AudioSequence> {
    if !nanomp3::detect(data) {
        return None;
    }
    let decoded = decode_all::<i16>(data);
    let channels = match decoded.channels? {
        Channels::Mono => 1,
        Channels::Stereo => 2,
    };
    if decoded.sample_rate == 0 || decoded.samples.is_empty() {
        return None;
    }
    let duration = decoded.samples.len() as u64 * 1000 / (decoded.sample_rate as u64 * channels as u64);

    Some(AudioSequence {
        duration,
        events: vec![TimedAudioEvent {
            time: 0,
            data: AudioEventData::Wave {
                channels,
                sampling_rate: decoded.sample_rate,
                samples: decoded.samples,
            },
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::parse_mpeg_audio;
    use crate::AudioEventData;

    #[test]
    fn decodes_mp3() {
        // 0.2 s of a 440 Hz sine, mono, 8000 Hz, 16 kbps; with no LAME tag the encoder delay and padding stay
        let sequence = parse_mpeg_audio(include_bytes!("tone_440hz.mp3")).unwrap();
        let AudioEventData::Wave {
            channels,
            sampling_rate,
            samples,
        } = &sequence.events[0].data
        else {
            panic!("not a wave event");
        };
        assert_eq!((*channels, *sampling_rate), (1, 8000));
        assert!((150..=400).contains(&sequence.duration), "{} ms", sequence.duration);
        assert!(samples.iter().any(|sample| sample.unsigned_abs() > 1000), "the tone is heard");
    }

    #[test]
    fn other_data_is_not_mpeg_audio() {
        assert!(parse_mpeg_audio(b"RIFF\0\0\0\0WAVEfmt ").is_none());
        assert!(parse_mpeg_audio(&[0; 64]).is_none());
    }
}

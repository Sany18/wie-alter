use alloc::{vec, vec::Vec};

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_backend::{AudioEventData, AudioSequence, TimedAudioEvent};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::ringtone::{self, SAMPLE_RATE, Tone};

const FORMAT_TONE: i32 = 1;
const FORMAT_WAV: i32 = 5;
const SOUND_PLAYING: i32 = 0;
const SOUND_STOPPED: i32 = 1;
const NO_AUDIO: i32 = -1;
const FOREVER: i64 = i64::MAX;
// A finite loop count plays the rendered tones that many times; keep the buffer bounded.
const MAX_RENDERED_MS: u64 = 60_000;

const CONSTANTS: [(&str, i32); 5] = [
    ("FORMAT_TONE", FORMAT_TONE),
    ("FORMAT_WAV", FORMAT_WAV),
    ("SOUND_PLAYING", SOUND_PLAYING),
    ("SOUND_STOPPED", SOUND_STOPPED),
    ("SOUND_UNINITIALIZED", 3),
];

// class com.nokia.mid.sound.Sound
// Ringing tones (FORMAT_TONE, Smart Messaging OTT) and plain tones play as square-wave PCM; WAV data stays silent.
pub struct Sound;

impl Sound {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "com/nokia/mid/sound/Sound",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::cl_init, MethodAccessFlags::STATIC),
                JavaMethodProto::new("<init>", "([BI)V", Self::init_with_data, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "(IJ)V", Self::init_with_tone, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("init", "([BI)V", Self::reinit_with_data, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("init", "(IJ)V", Self::reinit_with_tone, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("play", "(I)V", Self::play, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("stop", "()V", Self::stop, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("resume", "()V", Self::resume, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("release", "()V", Self::release, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getState", "()I", Self::get_state, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setGain", "(I)V", Self::set_gain, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getGain", "()I", Self::get_gain, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setSoundListener",
                    "(Lcom/nokia/mid/sound/SoundListener;)V",
                    Self::set_sound_listener,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "getSupportedFormats",
                    "()[I",
                    Self::get_supported_formats,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "getConcurrentSoundCount",
                    "(I)I",
                    Self::get_concurrent_sound_count,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
            ],
            fields: CONSTANTS
                .iter()
                .map(|(name, _)| JavaFieldProto::new(name, "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL))
                .chain([
                    JavaFieldProto::new("gain", "I", FieldAccessFlags::PRIVATE),
                    JavaFieldProto::new("data", "[B", FieldAccessFlags::PRIVATE),
                    JavaFieldProto::new("format", "I", FieldAccessFlags::PRIVATE),
                    JavaFieldProto::new("frequency", "I", FieldAccessFlags::PRIVATE),
                    JavaFieldProto::new("duration", "J", FieldAccessFlags::PRIVATE),
                    JavaFieldProto::new("audioHandle", "I", FieldAccessFlags::PRIVATE),
                    JavaFieldProto::new("playingUntil", "J", FieldAccessFlags::PRIVATE),
                ])
                .collect(),
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn cl_init(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::<clinit>()");

        for (name, value) in CONSTANTS {
            jvm.put_static_field("com/nokia/mid/sound/Sound", name, "I", value).await?;
        }
        Ok(())
    }

    async fn init_with_data(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<i8>>,
        format: i32,
    ) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::<init>({this:?}, {data:?}, {format})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "gain", "I", 255).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "audioHandle", "I", NO_AUDIO)
            .await?;
        jvm.invoke_virtual(&this, "com/nokia/mid/sound/Sound", "init", "([BI)V", (data, format))
            .await
    }

    async fn init_with_tone(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, frequency: i32, duration: i64) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::<init>({this:?}, {frequency}, {duration})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "gain", "I", 255).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "audioHandle", "I", NO_AUDIO)
            .await?;
        jvm.invoke_virtual(&this, "com/nokia/mid/sound/Sound", "init", "(IJ)V", (frequency, duration))
            .await
    }

    async fn reinit_with_data(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<i8>>,
        format: i32,
    ) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::init({this:?}, {data:?}, {format})");

        Self::release_audio(jvm, context, &mut this).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "data", "[B", data).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "format", "I", format).await
    }

    async fn reinit_with_tone(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        frequency: i32,
        duration: i64,
    ) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::init({this:?}, {frequency}, {duration})");

        Self::release_audio(jvm, context, &mut this).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "data", "[B", None).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "frequency", "I", frequency).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "duration", "J", duration).await
    }

    async fn play(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, loop_count: i32) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::play({this:?}, {loop_count})");

        Self::release_audio(jvm, context, &mut this).await?;

        let data: ClassInstanceRef<Array<i8>> = jvm.get_field(&this, "com/nokia/mid/sound/Sound", "data", "[B").await?;
        let tones = if data.is_null() {
            let frequency: i32 = jvm.get_field(&this, "com/nokia/mid/sound/Sound", "frequency", "I").await?;
            let duration: i64 = jvm.get_field(&this, "com/nokia/mid/sound/Sound", "duration", "J").await?;
            ringtone::plain_tone(frequency, duration)
        } else {
            let format: i32 = jvm.get_field(&this, "com/nokia/mid/sound/Sound", "format", "I").await?;
            let length = jvm.array_length(&data).await?;
            let bytes: Vec<i8> = jvm.load_array(&data, 0, length).await?;
            let bytes: Vec<u8> = bytes.into_iter().map(|byte| byte as u8).collect();

            match (format, ringtone::parse_ott(&bytes)) {
                (FORMAT_TONE, Some(tones)) => tones,
                _ => {
                    tracing::warn!("com.nokia.mid.sound.Sound::play: unsupported sound data (format {format}), silent");
                    return Ok(());
                }
            }
        };

        let once_ms = ringtone::duration_ms(&tones);
        if once_ms == 0 {
            return Ok(());
        }

        // 0 loops forever; otherwise the tones repeat loop_count times.
        let repeat = loop_count <= 0;
        let times = if repeat {
            1
        } else {
            (loop_count as u64).min(MAX_RENDERED_MS / once_ms).max(1)
        };
        let tones: Vec<Tone> = (0..times).flat_map(|_| tones.iter().cloned()).collect();

        let gain: i32 = jvm.get_field(&this, "com/nokia/mid/sound/Sound", "gain", "I").await?;
        let sequence = AudioSequence {
            duration: once_ms * times,
            events: vec![TimedAudioEvent {
                time: 0,
                data: AudioEventData::Wave {
                    channels: 1,
                    sampling_rate: SAMPLE_RATE,
                    samples: ringtone::render(&tones, gain),
                },
            }],
        };

        let system = context.system();
        let now = system.platform().now().raw() as i64;
        let handle = system.audio().load_sequence(sequence);
        let duration = system.audio().play(handle, repeat).unwrap() as i64;

        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "audioHandle", "I", handle as i32)
            .await?;
        let until = if repeat { FOREVER } else { now + duration };
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "playingUntil", "J", until).await
    }

    async fn stop(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::stop({this:?})");

        Self::release_audio(jvm, context, &mut this).await
    }

    async fn resume(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::resume({this:?})");

        Ok(())
    }

    async fn release(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::release({this:?})");

        Self::release_audio(jvm, context, &mut this).await
    }

    async fn get_state(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("com.nokia.mid.sound.Sound::getState({this:?})");

        let handle: i32 = jvm.get_field(&this, "com/nokia/mid/sound/Sound", "audioHandle", "I").await?;
        let until: i64 = jvm.get_field(&this, "com/nokia/mid/sound/Sound", "playingUntil", "J").await?;
        let now = context.system().platform().now().raw() as i64;

        Ok(if handle != NO_AUDIO && now < until {
            SOUND_PLAYING
        } else {
            SOUND_STOPPED
        })
    }

    async fn set_gain(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, gain: i32) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::setGain({this:?}, {gain})");

        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "gain", "I", gain.clamp(0, 255))
            .await
    }

    async fn get_gain(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("com.nokia.mid.sound.Sound::getGain({this:?})");

        jvm.get_field(&this, "com/nokia/mid/sound/Sound", "gain", "I").await
    }

    async fn set_sound_listener(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, listener: ClassInstanceRef<()>) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::setSoundListener({this:?}, {listener:?})");

        Ok(())
    }

    // Stops this sound and frees its rendered audio.
    async fn release_audio(jvm: &Jvm, context: &mut WieJvmContext, this: &mut ClassInstanceRef<Self>) -> JvmResult<()> {
        let handle: i32 = jvm.get_field(this, "com/nokia/mid/sound/Sound", "audioHandle", "I").await?;
        if handle != NO_AUDIO {
            let _ = context.system().audio().close(handle as u32);
            jvm.put_field(this, "com/nokia/mid/sound/Sound", "audioHandle", "I", NO_AUDIO).await?;
        }
        Ok(())
    }

    async fn get_supported_formats(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<ClassInstanceRef<Array<i32>>> {
        tracing::debug!("com.nokia.mid.sound.Sound::getSupportedFormats()");

        let mut formats = jvm.instantiate_array("I", 2).await?;
        jvm.store_array(&mut formats, 0, vec![FORMAT_TONE, FORMAT_WAV]).await?;
        Ok(formats.into())
    }

    async fn get_concurrent_sound_count(_: &Jvm, _: &mut WieJvmContext, format: i32) -> JvmResult<i32> {
        tracing::debug!("com.nokia.mid.sound.Sound::getConcurrentSoundCount({format})");

        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::{boxed::Box, sync::Arc, vec::Vec};
    use std::sync::Mutex;

    use jvm::{ClassInstanceRef, Jvm};

    use test_utils::{TestClock, TestPlatform, run_jvm_test_with_system};
    use wie_backend::{AudioCommand, AudioEventData};

    use super::{FORMAT_TONE, SOUND_PLAYING, SOUND_STOPPED, Sound};

    // C, G, C as 1/8, 1/16, 1/8 notes at 125 bpm: 600 ms.
    const RINGING_TONE: [i8; 16] = [
        0x02, 0x4a, 0x3a, 0x40, 0x04, 0x00, 0x11, 0x1c, -0x3b, 0x11, 0x62, 0x4c, 0x41, 0x44, 0x58, 0x00,
    ];

    async fn state(jvm: &Jvm, sound: &ClassInstanceRef<Sound>) -> jvm::Result<i32> {
        jvm.invoke_virtual(sound, "com/nokia/mid/sound/Sound", "getState", "()I", ()).await
    }

    #[test]
    fn plays_ringing_tones_and_plain_tones() {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let commands_clone = commands.clone();
        let clock = TestClock::new();
        let clock_clone = clock.clone();
        let platform = TestPlatform::with_clock(clock).with_audio_handler(move |command| commands_clone.lock().unwrap().push(command));

        run_jvm_test_with_system(
            Box::new([wie_midp::get_protos().into(), crate::get_protos().into()]),
            Box::new(platform),
            move |jvm, _| async move {
                let mut data = jvm.instantiate_array("B", RINGING_TONE.len()).await?;
                jvm.store_array(&mut data, 0, RINGING_TONE).await?;
                let sound: ClassInstanceRef<Sound> = jvm.new_class("com/nokia/mid/sound/Sound", "([BI)V", (data, FORMAT_TONE)).await?.into();
                assert_eq!(state(&jvm, &sound).await?, SOUND_STOPPED);

                let _: () = jvm.invoke_virtual(&sound, "com/nokia/mid/sound/Sound", "play", "(I)V", (1,)).await?;
                assert_eq!(state(&jvm, &sound).await?, SOUND_PLAYING);
                clock_clone.advance(600);
                assert_eq!(state(&jvm, &sound).await?, SOUND_STOPPED);

                let _: () = jvm.invoke_virtual(&sound, "com/nokia/mid/sound/Sound", "play", "(I)V", (0,)).await?;
                let _: () = jvm.invoke_virtual(&sound, "com/nokia/mid/sound/Sound", "stop", "()V", ()).await?;
                assert_eq!(state(&jvm, &sound).await?, SOUND_STOPPED);

                let tone: ClassInstanceRef<Sound> = jvm.new_class("com/nokia/mid/sound/Sound", "(IJ)V", (440, 100i64)).await?.into();
                let _: () = jvm.invoke_virtual(&tone, "com/nokia/mid/sound/Sound", "play", "(I)V", (2,)).await?;

                Ok(())
            },
        )
        .unwrap();

        let commands = commands.lock().unwrap();
        let plays: Vec<(u64, bool, usize)> = commands
            .iter()
            .filter_map(|command| match command {
                AudioCommand::Play { sequence, repeat, .. } => match &sequence.events[0].data {
                    AudioEventData::Wave { channels: 1, samples, .. } => Some((sequence.duration, *repeat, samples.len())),
                    _ => None,
                },
                AudioCommand::Stop { .. } => None,
            })
            .collect();
        assert_eq!(plays, [(600, false, 13230), (600, true, 13230), (200, false, 4410)]);
        assert!(commands.iter().any(|command| matches!(command, AudioCommand::Stop { .. })));
    }
}

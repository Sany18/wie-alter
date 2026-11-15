use alloc::vec;

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

const FORMAT_TONE: i32 = 1;
const FORMAT_WAV: i32 = 5;
const SOUND_STOPPED: i32 = 1;

const CONSTANTS: [(&str, i32); 5] = [
    ("FORMAT_TONE", FORMAT_TONE),
    ("FORMAT_WAV", FORMAT_WAV),
    ("SOUND_PLAYING", 0),
    ("SOUND_STOPPED", SOUND_STOPPED),
    ("SOUND_UNINITIALIZED", 3),
];

// class com.nokia.mid.sound.Sound
// Playback is not emulated yet: sounds stay silent and report the stopped state.
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
                .chain([JavaFieldProto::new("gain", "I", FieldAccessFlags::PRIVATE)])
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
        jvm.invoke_virtual(&this, "com/nokia/mid/sound/Sound", "init", "([BI)V", (data, format))
            .await
    }

    async fn init_with_tone(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, frequency: i32, duration: i64) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.sound.Sound::<init>({this:?}, {frequency}, {duration})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "com/nokia/mid/sound/Sound", "gain", "I", 255).await?;
        jvm.invoke_virtual(&this, "com/nokia/mid/sound/Sound", "init", "(IJ)V", (frequency, duration))
            .await
    }

    async fn reinit_with_data(
        _: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<i8>>,
        format: i32,
    ) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::init({this:?}, {data:?}, {format})");

        Ok(())
    }

    async fn reinit_with_tone(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, frequency: i32, duration: i64) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::init({this:?}, {frequency}, {duration})");

        Ok(())
    }

    async fn play(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, loop_count: i32) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::play({this:?}, {loop_count})");

        Ok(())
    }

    async fn stop(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::stop({this:?})");

        Ok(())
    }

    async fn resume(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::resume({this:?})");

        Ok(())
    }

    async fn release(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::release({this:?})");

        Ok(())
    }

    async fn get_state(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::warn!("stub com.nokia.mid.sound.Sound::getState({this:?})");

        Ok(SOUND_STOPPED)
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

use alloc::{boxed::Box, vec};

use jvm::{
    Array, ClassInstance, ClassInstanceRef, Jvm, Result,
    runtime::{JavaIoInputStream, JavaLangString},
};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::{
    io::InputStream,
    lang::{Object, String},
};

use wie_backend::AudioSequence;
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::{
    javax::microedition::media::{Control, PlayerListener},
    net::wie::{PlayerToneControl, PlayerVolumeControl},
};

// class net.wie.SmafPlayer
// Plays SMAF, Standard MIDI and PCM WAVE data through the system audio.
pub struct SmafPlayer;

impl SmafPlayer {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "net/wie/SmafPlayer",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["javax/microedition/media/Player"],
            methods: vec![
                JavaMethodProto::new("<init>", "(Ljava/io/InputStream;)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("<init>", "()V", Self::init_tone, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("realize", "()V", Self::realize, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("prefetch", "()V", Self::prefetch, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("start", "()V", Self::start, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("start", "(Z)V", Self::start_with_repeat, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("stop", "()V", Self::stop, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("deallocate", "()V", Self::deallocate, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("close", "()V", Self::close, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setMediaTime", "(J)J", Self::set_media_time, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getMediaTime", "()J", Self::get_media_time, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getState", "()I", Self::get_state, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getDuration", "()J", Self::get_duration, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getContentType",
                    "()Ljava/lang/String;",
                    Self::get_content_type,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("setLoopCount", "(I)V", Self::set_loop_count, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "addPlayerListener",
                    "(Ljavax/microedition/media/PlayerListener;)V",
                    Self::add_player_listener,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "removePlayerListener",
                    "(Ljavax/microedition/media/PlayerListener;)V",
                    Self::remove_player_listener,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "getControl",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
                    Self::get_control,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "setTimeBase",
                    "(Ljavax/microedition/media/TimeBase;)V",
                    Self::set_time_base,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "getTimeBase",
                    "()Ljavax/microedition/media/TimeBase;",
                    Self::get_time_base,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "getControls",
                    "()[Ljavax/microedition/media/Control;",
                    Self::get_controls,
                    MethodAccessFlags::PUBLIC,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("audioHandle", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("volumeControl", "Lnet/wie/PlayerVolumeControl;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("toneControl", "Lnet/wie/PlayerToneControl;", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, stream: ClassInstanceRef<InputStream>) -> Result<()> {
        tracing::debug!("net.wie.SmafPlayer::<init>({this:?}, {stream:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        let data = JavaIoInputStream::read_until_end(jvm, &stream).await?;
        // A MIDI or WAVE file by its header; anything else is taken as SMAF, as before.
        let audio_handle = if data.starts_with(b"MThd") || data.starts_with(b"RIFF") {
            match context.system().audio().load(&data) {
                Ok(handle) => handle,
                Err(_) => return Err(jvm.exception("javax/microedition/media/MediaException", "Unsupported media data").await),
            }
        } else {
            context.system().audio().load_smaf(&data).unwrap()
        };

        jvm.put_field(&mut this, "net/wie/SmafPlayer", "audioHandle", "I", audio_handle as i32)
            .await?;

        Ok(())
    }

    // The tone player of Manager.createPlayer("device://tone"): silent until its ToneControl gets a sequence.
    async fn init_tone(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::debug!("net.wie.SmafPlayer::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        let audio_handle = context.system().audio().load_sequence(AudioSequence { duration: 0, events: vec![] });
        jvm.put_field(&mut this, "net/wie/SmafPlayer", "audioHandle", "I", audio_handle as i32)
            .await?;

        let tone_control = jvm
            .new_class("net/wie/PlayerToneControl", "(Lnet/wie/SmafPlayer;)V", (this.clone(),))
            .await?;
        jvm.put_field(
            &mut this,
            "net/wie/SmafPlayer",
            "toneControl",
            "Lnet/wie/PlayerToneControl;",
            tone_control,
        )
        .await
    }

    async fn start(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        Self::start_with_repeat(jvm, context, this, false).await
    }

    async fn start_with_repeat(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, repeat: bool) -> Result<()> {
        tracing::debug!("net.wie.SmafPlayer::start({this:?}, {repeat})");

        let audio_handle: i32 = jvm.get_field(&this, "net/wie/SmafPlayer", "audioHandle", "I").await?;

        context.system().audio().play(audio_handle as u32, repeat).unwrap();

        Ok(())
    }

    async fn stop(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::debug!("net.wie.SmafPlayer::stop({this:?})");

        let audio_handle: i32 = jvm.get_field(&this, "net/wie/SmafPlayer", "audioHandle", "I").await?;

        let system = context.system();

        system.audio().stop(audio_handle as u32);

        Ok(())
    }

    async fn close(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::debug!("net.wie.SmafPlayer::close({this:?})");

        let audio_handle: i32 = jvm.get_field(&this, "net/wie/SmafPlayer", "audioHandle", "I").await?;

        let system = context.system();

        // Closing a closed player does nothing.
        let _ = system.audio().close(audio_handle as u32);

        Ok(())
    }

    // The player keeps its own clock: a time base the game sets is not followed.
    async fn set_time_base(
        _jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        time_base: ClassInstanceRef<Object>,
    ) -> Result<()> {
        tracing::warn!("stub net.wie.SmafPlayer::setTimeBase({this:?}, {time_base:?})");

        Ok(())
    }

    async fn get_time_base(_jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<Object>> {
        tracing::warn!("stub net.wie.SmafPlayer::getTimeBase({this:?})");

        Ok(None.into())
    }

    async fn realize(_jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::warn!("stub net.wie.SmafPlayer::realize({this:?})");

        Ok(())
    }

    async fn prefetch(_jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::warn!("stub net.wie.SmafPlayer::prefetch({this:?})");

        Ok(())
    }

    async fn deallocate(_jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::warn!("stub net.wie.SmafPlayer::deallocate({this:?})");

        Ok(())
    }

    async fn set_media_time(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, now: i64) -> Result<i64> {
        tracing::warn!("stub net.wie.SmafPlayer::setMediaTime({this:?}, {now})");

        Err(jvm.exception("javax/microedition/media/MediaException", "Seeking is not supported").await)
    }

    async fn get_media_time(_jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i64> {
        tracing::warn!("stub net.wie.SmafPlayer::getMediaTime({this:?})");

        Ok(-1)
    }

    async fn get_state(_jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        tracing::warn!("stub net.wie.SmafPlayer::getState({this:?})");

        Ok(300)
    }

    async fn get_duration(_jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i64> {
        tracing::warn!("stub net.wie.SmafPlayer::getDuration({this:?})");

        Ok(-1)
    }

    async fn get_content_type(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<String>> {
        tracing::debug!("net.wie.SmafPlayer::getContentType({this:?})");

        Ok(JavaLangString::from_rust_string(jvm, "application/vnd.smaf").await?.into())
    }

    async fn set_loop_count(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, count: i32) -> Result<()> {
        tracing::warn!("stub net.wie.SmafPlayer::setLoopCount({this:?}, {count})");

        if count == 0 || count < -1 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid loop count").await);
        }

        Ok(())
    }

    async fn add_player_listener(
        _jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        listener: ClassInstanceRef<PlayerListener>,
    ) -> Result<()> {
        tracing::warn!("stub net.wie.SmafPlayer::addPlayerListener({this:?}, {listener:?})");

        Ok(())
    }

    async fn remove_player_listener(
        _jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        listener: ClassInstanceRef<PlayerListener>,
    ) -> Result<()> {
        tracing::warn!("stub net.wie.SmafPlayer::removePlayerListener({this:?}, {listener:?})");

        Ok(())
    }

    async fn get_control(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        control_type: ClassInstanceRef<String>,
    ) -> Result<ClassInstanceRef<Control>> {
        tracing::debug!("net.wie.SmafPlayer::getControl({this:?}, {control_type:?})");

        if control_type.is_null() {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "Control type is null").await);
        }

        // MMAPI: a control of the package javax.microedition.media.control may be named without the package.
        let control_type = JavaLangString::to_rust_string(jvm, &control_type).await?;
        let control_type = control_type.strip_prefix("javax.microedition.media.control.").unwrap_or(&control_type);
        if control_type == "ToneControl" {
            // Only a tone player has one.
            let tone_control: ClassInstanceRef<PlayerToneControl> = jvm
                .get_field(&this, "net/wie/SmafPlayer", "toneControl", "Lnet/wie/PlayerToneControl;")
                .await?;
            let tone_control: Option<Box<dyn ClassInstance>> = tone_control.into();
            return Ok(tone_control.into());
        }
        if control_type != "VolumeControl" {
            return Ok(None.into());
        }

        let volume_control: Box<dyn ClassInstance> = Self::volume_control(jvm, this).await?.into();

        Ok(volume_control.into())
    }

    async fn get_controls(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<Array<Control>>> {
        tracing::debug!("net.wie.SmafPlayer::getControls({this:?})");

        let volume_control = Self::volume_control(jvm, this).await?;
        let mut controls = jvm.instantiate_array("Ljavax/microedition/media/Control;", 1).await?;
        jvm.store_array(&mut controls, 0, [volume_control]).await?;

        Ok(controls.into())
    }

    // The player's one VolumeControl, made on the first request.
    async fn volume_control(jvm: &Jvm, mut this: ClassInstanceRef<Self>) -> Result<ClassInstanceRef<PlayerVolumeControl>> {
        let volume_control: ClassInstanceRef<PlayerVolumeControl> = jvm
            .get_field(&this, "net/wie/SmafPlayer", "volumeControl", "Lnet/wie/PlayerVolumeControl;")
            .await?;
        if !volume_control.is_null() {
            return Ok(volume_control);
        }

        let volume_control: ClassInstanceRef<PlayerVolumeControl> = jvm.new_class("net/wie/PlayerVolumeControl", "()V", ()).await?.into();
        jvm.put_field(
            &mut this,
            "net/wie/SmafPlayer",
            "volumeControl",
            "Lnet/wie/PlayerVolumeControl;",
            volume_control.clone(),
        )
        .await?;

        Ok(volume_control)
    }
}

#[cfg(test)]
mod test {
    use alloc::boxed::Box;

    use jvm::{Array, ClassInstanceRef, JavaError, runtime::JavaLangString};
    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::{
        classes::javax::microedition::media::{Control, Player},
        get_protos,
    };

    #[test]
    fn test_player_controls() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            let data = jvm.instantiate_array("B", 0).await?;
            let stream = jvm.new_class("java/io/ByteArrayInputStream", "([B)V", (data,)).await?;
            let content_type = JavaLangString::from_rust_string(&jvm, "application/vnd.smaf").await?;
            let player: ClassInstanceRef<Player> = jvm
                .invoke_static(
                    "javax/microedition/media/Manager",
                    "createPlayer",
                    "(Ljava/io/InputStream;Ljava/lang/String;)Ljavax/microedition/media/Player;",
                    (stream, content_type),
                )
                .await?;
            let _: () = jvm
                .invoke_virtual(&player, "javax/microedition/media/Player", "realize", "()V", ())
                .await?;

            let controls: ClassInstanceRef<Array<Control>> = jvm
                .invoke_virtual(
                    &player,
                    "javax/microedition/media/Controllable",
                    "getControls",
                    "()[Ljavax/microedition/media/Control;",
                    (),
                )
                .await?;
            assert_eq!(jvm.array_length(&controls).await?, 1);

            let control_type = JavaLangString::from_rust_string(&jvm, "VolumeControl").await?;
            let control: ClassInstanceRef<Control> = jvm
                .invoke_virtual(
                    &player,
                    "javax/microedition/media/Player",
                    "getControl",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
                    (control_type,),
                )
                .await?;
            assert!(jvm.is_instance(&**control, "javax/microedition/media/control/VolumeControl"));
            let level: i32 = jvm
                .invoke_virtual(&control, "javax/microedition/media/control/VolumeControl", "setLevel", "(I)I", (150,))
                .await?;
            assert_eq!(level, 100);
            let _: () = jvm
                .invoke_virtual(&control, "javax/microedition/media/control/VolumeControl", "setMute", "(Z)V", (true,))
                .await?;

            let control_type = JavaLangString::from_rust_string(&jvm, "javax.microedition.media.control.VolumeControl").await?;
            let same_control: ClassInstanceRef<Control> = jvm
                .invoke_virtual(
                    &player,
                    "javax/microedition/media/Player",
                    "getControl",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
                    (control_type,),
                )
                .await?;
            let muted: bool = jvm
                .invoke_virtual(&same_control, "javax/microedition/media/control/VolumeControl", "isMuted", "()Z", ())
                .await?;
            assert!(muted);

            let control_type = JavaLangString::from_rust_string(&jvm, "ToneControl").await?;
            let control: ClassInstanceRef<Control> = jvm
                .invoke_virtual(
                    &player,
                    "javax/microedition/media/Player",
                    "getControl",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
                    (control_type,),
                )
                .await?;
            assert!(control.is_null());

            let JavaError::JavaException(exception) = jvm
                .invoke_virtual::<_, ClassInstanceRef<Control>>(
                    &player,
                    "javax/microedition/media/Controllable",
                    "getControl",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
                    (None,),
                )
                .await
                .unwrap_err();
            assert!(jvm.is_instance(&*exception, "java/lang/IllegalArgumentException"));

            let _: () = jvm
                .invoke_virtual(&player, "javax/microedition/media/Player", "setLoopCount", "(I)V", (-1,))
                .await?;
            let JavaError::JavaException(exception) = jvm
                .invoke_virtual::<_, ()>(&player, "javax/microedition/media/Player", "setLoopCount", "(I)V", (0,))
                .await
                .unwrap_err();
            assert!(jvm.is_instance(&*exception, "java/lang/IllegalArgumentException"));

            let JavaError::JavaException(exception) = jvm
                .invoke_virtual::<_, i64>(&player, "javax/microedition/media/Player", "setMediaTime", "(J)J", (0i64,))
                .await
                .unwrap_err();
            assert!(jvm.is_instance(&*exception, "javax/microedition/media/MediaException"));

            let _: () = jvm.invoke_virtual(&player, "javax/microedition/media/Player", "close", "()V", ()).await?;
            Ok(())
        })
    }
}

use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class net.wie.PlayerVolumeControl
// The VolumeControl of a player: keeps the level and the mute state the application sets, so that it reads them back.
// The system audio has no volume, so the sound itself does not change.
pub struct PlayerVolumeControl;

impl PlayerVolumeControl {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "net/wie/PlayerVolumeControl",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["javax/microedition/media/control/VolumeControl"],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setMute", "(Z)V", Self::set_mute, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isMuted", "()Z", Self::is_muted, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setLevel", "(I)I", Self::set_level, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getLevel", "()I", Self::get_level, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("level", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("muted", "Z", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> Result<()> {
        tracing::debug!("net.wie.PlayerVolumeControl::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "net/wie/PlayerVolumeControl", "level", "I", 100).await?;

        Ok(())
    }

    async fn set_mute(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, mute: bool) -> Result<()> {
        tracing::debug!("net.wie.PlayerVolumeControl::setMute({this:?}, {mute})");

        jvm.put_field(&mut this, "net/wie/PlayerVolumeControl", "muted", "Z", mute).await
    }

    async fn is_muted(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<bool> {
        tracing::debug!("net.wie.PlayerVolumeControl::isMuted({this:?})");

        jvm.get_field(&this, "net/wie/PlayerVolumeControl", "muted", "Z").await
    }

    async fn set_level(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, level: i32) -> Result<i32> {
        tracing::debug!("net.wie.PlayerVolumeControl::setLevel({this:?}, {level})");

        // MMAPI: a level below 0 is taken as 0 and above 100 as 100; the result is the level that was set.
        let level = level.clamp(0, 100);
        jvm.put_field(&mut this, "net/wie/PlayerVolumeControl", "level", "I", level).await?;

        Ok(level)
    }

    async fn get_level(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> Result<i32> {
        tracing::debug!("net.wie.PlayerVolumeControl::getLevel({this:?})");

        jvm.get_field(&this, "net/wie/PlayerVolumeControl", "level", "I").await
    }
}

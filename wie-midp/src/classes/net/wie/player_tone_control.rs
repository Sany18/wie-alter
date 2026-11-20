use alloc::vec;

use jvm::{Array, ClassInstanceRef, Jvm, Result};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::{classes::net::wie::SmafPlayer, tone_sequence::parse_tone_sequence};

// class net.wie.PlayerToneControl
// The ToneControl of a tone player (Manager.createPlayer("device://tone")): a new sequence replaces what it plays.
pub struct PlayerToneControl;

impl PlayerToneControl {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "net/wie/PlayerToneControl",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["javax/microedition/media/control/ToneControl"],
            methods: vec![
                JavaMethodProto::new("<init>", "(Lnet/wie/SmafPlayer;)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setSequence", "([B)V", Self::set_sequence, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![JavaFieldProto::new("player", "Lnet/wie/SmafPlayer;", FieldAccessFlags::PRIVATE)],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, player: ClassInstanceRef<SmafPlayer>) -> Result<()> {
        tracing::debug!("net.wie.PlayerToneControl::<init>({this:?}, {player:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "net/wie/PlayerToneControl", "player", "Lnet/wie/SmafPlayer;", player)
            .await
    }

    async fn set_sequence(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, sequence: ClassInstanceRef<Array<i8>>) -> Result<()> {
        tracing::debug!("net.wie.PlayerToneControl::setSequence({this:?}, {sequence:?})");

        if sequence.is_null() {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "sequence is null").await);
        }
        let length = jvm.array_length(&sequence).await?;
        let data: vec::Vec<i8> = jvm.load_array(&sequence, 0, length).await?;
        let Some(sequence) = parse_tone_sequence(&data) else {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid tone sequence").await);
        };

        let mut player: ClassInstanceRef<SmafPlayer> = jvm
            .get_field(&this, "net/wie/PlayerToneControl", "player", "Lnet/wie/SmafPlayer;")
            .await?;
        let old_handle: i32 = jvm.get_field(&player, "net/wie/SmafPlayer", "audioHandle", "I").await?;

        let system = context.system();
        // The old sequence is gone already when the player was closed.
        let _ = system.audio().close(old_handle as u32);
        let handle = system.audio().load_sequence(sequence);

        jvm.put_field(&mut player, "net/wie/SmafPlayer", "audioHandle", "I", handle as i32).await
    }
}

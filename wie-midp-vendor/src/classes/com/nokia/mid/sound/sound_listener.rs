use alloc::vec;

use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use wie_jvm_support::WieJavaClassProto;

// interface com.nokia.mid.sound.SoundListener
pub struct SoundListener;

impl SoundListener {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "com/nokia/mid/sound/SoundListener",
            parent_class: None,
            interfaces: vec![],
            methods: vec![JavaMethodProto::new_abstract(
                "soundStateChanged",
                "(Lcom/nokia/mid/sound/Sound;I)V",
                MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
            )],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }
}

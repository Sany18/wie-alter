use alloc::{string::String as RustString, vec};

use jvm::{Array, ClassInstanceRef, Jvm, Result, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::{io::InputStream, lang::String};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::{classes::javax::microedition::media::Player, tone_sequence::single_tone};

// The names games give Standard MIDI and WAVE data (net.wie.SmafPlayer tells the formats apart by their headers).
const MIDI_TYPES: [&str; 5] = ["audio/midi", "audio/mid", "audio/x-midi", "audio/x-mid", "audio/sp-midi"];
const WAVE_TYPES: [&str; 4] = ["audio/x-wav", "audio/wav", "audio/wave", "audio/x-wave"];
const TONE_DEVICE_LOCATOR: &str = "device://tone";

// class javax.microedition.media.Manager
pub struct Manager;

impl Manager {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "javax/microedition/media/Manager",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "createPlayer",
                    "(Ljava/io/InputStream;Ljava/lang/String;)Ljavax/microedition/media/Player;",
                    Self::create_player,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "createPlayer",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Player;",
                    Self::create_player_from_locator,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "getSupportedProtocols",
                    "(Ljava/lang/String;)[Ljava/lang/String;",
                    Self::get_supported_protocols,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "playTone",
                    "(III)V",
                    Self::play_tone,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "getSupportedContentTypes",
                    "(Ljava/lang/String;)[Ljava/lang/String;",
                    Self::get_supported_content_types,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
            ],
            fields: vec![JavaFieldProto::new(
                "toneHandle",
                "I",
                FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC,
            )],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL,
        }
    }

    async fn create_player(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        stream: ClassInstanceRef<InputStream>,
        r#type: ClassInstanceRef<String>,
    ) -> Result<ClassInstanceRef<Player>> {
        tracing::debug!("javax.microedition.media.Manager::createPlayer({stream:?}, {type:?})");

        // A game whose resource is missing passes the null of getResourceAsStream.
        if stream.is_null() {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "stream is null").await);
        }
        // Content types are case-insensitive and may carry parameters ("audio/X-wav", "audio/midi; rate=8000").
        // A null type asks the player to find out the format: it tells MIDI and WAVE by their headers.
        let type_string = if r#type.is_null() {
            RustString::new()
        } else {
            JavaLangString::to_rust_string(jvm, &r#type).await?
        };
        let type_string = type_string.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
        if type_string.is_empty()
            || type_string == "application/vnd.smaf"
            || MIDI_TYPES.contains(&type_string.as_str())
            || WAVE_TYPES.contains(&type_string.as_str())
        {
            Ok(jvm.new_class("net/wie/SmafPlayer", "(Ljava/io/InputStream;)V", (stream,)).await?.into())
        } else {
            Err(jvm.exception("javax/microedition/media/MediaException", "Unsupported media type").await)
        }
    }

    // Players are created from streams only, so no locator protocol is supported.
    async fn create_player_from_locator(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        locator: ClassInstanceRef<String>,
    ) -> Result<ClassInstanceRef<Player>> {
        tracing::debug!("javax.microedition.media.Manager::createPlayer({locator:?})");

        if locator.is_null() {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "locator is null").await);
        }
        if JavaLangString::to_rust_string(jvm, &locator).await? == TONE_DEVICE_LOCATOR {
            return Ok(jvm.new_class("net/wie/SmafPlayer", "()V", ()).await?.into());
        }

        Err(jvm.exception("javax/microedition/media/MediaException", "Unsupported locator").await)
    }

    async fn get_supported_protocols(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        content_type: ClassInstanceRef<String>,
    ) -> Result<ClassInstanceRef<Array<String>>> {
        tracing::debug!("javax.microedition.media.Manager::getSupportedProtocols({content_type:?})");

        Ok(jvm.instantiate_array("Ljava/lang/String;", 0).await?.into())
    }

    async fn play_tone(jvm: &Jvm, context: &mut WieJvmContext, note: i32, duration: i32, volume: i32) -> Result<()> {
        tracing::debug!("javax.microedition.media.Manager::playTone({note}, {duration}, {volume})");

        if !(0..=127).contains(&note) || duration <= 0 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid tone").await);
        }

        // One tone at a time: a new tone replaces the one before. toneHandle is the audio handle + 1, 0 for none.
        let previous: i32 = jvm.get_static_field("javax/microedition/media/Manager", "toneHandle", "I").await?;
        let system = context.system();
        if previous != 0 {
            let _ = system.audio().close((previous - 1) as u32);
        }
        let handle = system.audio().load_sequence(single_tone(note as u8, duration as u64, volume));
        let _ = system.audio().play(handle, false);

        jvm.put_static_field("javax/microedition/media/Manager", "toneHandle", "I", handle as i32 + 1)
            .await
    }

    async fn get_supported_content_types(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        protocol: ClassInstanceRef<String>,
    ) -> Result<ClassInstanceRef<Array<String>>> {
        tracing::debug!("javax.microedition.media.Manager::getSupportedContentTypes({protocol:?})");

        let supported = ["application/vnd.smaf", MIDI_TYPES[0], WAVE_TYPES[0]];
        let count = if protocol.is_null() { supported.len() } else { 0 };
        let mut types = jvm.instantiate_array("Ljava/lang/String;", count).await?;
        for (index, content_type) in supported.iter().take(count).enumerate() {
            let content_type = JavaLangString::from_rust_string(jvm, content_type).await?;
            jvm.store_array(&mut types, index, [content_type]).await?;
        }
        Ok(types.into())
    }
}

#[cfg(test)]
mod test {
    use alloc::boxed::Box;

    use jvm::{Array, ClassInstanceRef, JavaError, runtime::JavaLangString};
    use rustjava_runtime::classes::java::lang::String;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::{
        classes::javax::microedition::media::{Control, Player},
        get_protos,
    };

    #[test]
    fn supported_content_types_match_player_creation() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            let content_types: ClassInstanceRef<Array<String>> = jvm
                .invoke_static(
                    "javax/microedition/media/Manager",
                    "getSupportedContentTypes",
                    "(Ljava/lang/String;)[Ljava/lang/String;",
                    (None,),
                )
                .await?;
            assert_eq!(jvm.array_length(&content_types).await?, 3);
            let content_type: ClassInstanceRef<String> = jvm.load_array(&content_types, 0, 1).await?.remove(0);
            assert_eq!(JavaLangString::to_rust_string(&jvm, &content_type).await?, "application/vnd.smaf");
            for protocol in ["http", "file", "", "unknown"] {
                let protocol = JavaLangString::from_rust_string(&jvm, protocol).await?;
                let types: ClassInstanceRef<Array<String>> = jvm
                    .invoke_static(
                        "javax/microedition/media/Manager",
                        "getSupportedContentTypes",
                        "(Ljava/lang/String;)[Ljava/lang/String;",
                        (protocol,),
                    )
                    .await?;
                assert_eq!(jvm.array_length(&types).await?, 0);
            }
            let data = jvm.instantiate_array("B", 0).await?;
            let stream = jvm.new_class("java/io/ByteArrayInputStream", "([B)V", (data,)).await?;
            let player: ClassInstanceRef<Player> = jvm
                .invoke_static(
                    "javax/microedition/media/Manager",
                    "createPlayer",
                    "(Ljava/io/InputStream;Ljava/lang/String;)Ljavax/microedition/media/Player;",
                    (stream, content_type),
                )
                .await?;
            assert!(jvm.is_instance(&**player, "javax/microedition/media/Player"));
            Ok(())
        })
    }

    #[test]
    fn tone_player_plays_tone_sequences() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            let locator = JavaLangString::from_rust_string(&jvm, "device://tone").await?;
            let player: ClassInstanceRef<Player> = jvm
                .invoke_static(
                    "javax/microedition/media/Manager",
                    "createPlayer",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Player;",
                    (locator,),
                )
                .await?;
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
            assert!(jvm.is_instance(&**control, "javax/microedition/media/control/ToneControl"));

            let mut sequence = jvm.instantiate_array("B", 4).await?;
            jvm.store_array(&mut sequence, 0, [-2i8, 1, 60, 16]).await?;
            let _: () = jvm
                .invoke_virtual(
                    &control,
                    "javax/microedition/media/control/ToneControl",
                    "setSequence",
                    "([B)V",
                    (sequence,),
                )
                .await?;
            let _: () = jvm.invoke_virtual(&player, "javax/microedition/media/Player", "start", "()V", ()).await?;

            let mut sequence = jvm.instantiate_array("B", 2).await?;
            jvm.store_array(&mut sequence, 0, [60i8, 16]).await?;
            let JavaError::JavaException(exception) = jvm
                .invoke_virtual::<_, ()>(
                    &control,
                    "javax/microedition/media/control/ToneControl",
                    "setSequence",
                    "([B)V",
                    (sequence,),
                )
                .await
                .unwrap_err();
            assert!(jvm.is_instance(&*exception, "java/lang/IllegalArgumentException"));

            let _: () = jvm
                .invoke_static("javax/microedition/media/Manager", "playTone", "(III)V", (69, 100, 50))
                .await?;
            let _: () = jvm
                .invoke_static("javax/microedition/media/Manager", "playTone", "(III)V", (72, 100, 50))
                .await?;
            let JavaError::JavaException(exception) = jvm
                .invoke_static::<_, ()>("javax/microedition/media/Manager", "playTone", "(III)V", (128, 100, 50))
                .await
                .unwrap_err();
            assert!(jvm.is_instance(&*exception, "java/lang/IllegalArgumentException"));

            let locator = JavaLangString::from_rust_string(&jvm, "capture://audio").await?;
            let JavaError::JavaException(exception) = jvm
                .invoke_static::<_, ClassInstanceRef<Player>>(
                    "javax/microedition/media/Manager",
                    "createPlayer",
                    "(Ljava/lang/String;)Ljavax/microedition/media/Player;",
                    (locator,),
                )
                .await
                .unwrap_err();
            assert!(jvm.is_instance(&*exception, "javax/microedition/media/MediaException"));
            Ok(())
        })
    }
}

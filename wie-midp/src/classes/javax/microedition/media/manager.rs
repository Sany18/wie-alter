use alloc::{string::String as RustString, vec};

use jvm::{Array, ClassInstanceRef, Jvm, Result, runtime::JavaLangString};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::{io::InputStream, lang::String};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::javax::microedition::media::Player;

// The names games give Standard MIDI and WAVE data (net.wie.SmafPlayer tells the formats apart by their headers).
const MIDI_TYPES: [&str; 5] = ["audio/midi", "audio/mid", "audio/x-midi", "audio/x-mid", "audio/sp-midi"];
const WAVE_TYPES: [&str; 4] = ["audio/x-wav", "audio/wav", "audio/wave", "audio/x-wave"];

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
            fields: vec![],
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

    async fn play_tone(_jvm: &Jvm, _context: &mut WieJvmContext, note: i32, duration: i32, volume: i32) -> Result<()> {
        tracing::warn!("stub javax.microedition.media.Manager::playTone({note}, {duration}, {volume})");

        Ok(())
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

    use jvm::{Array, ClassInstanceRef, runtime::JavaLangString};
    use rustjava_runtime::classes::java::lang::String;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::{classes::javax::microedition::media::Player, get_protos};

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
}

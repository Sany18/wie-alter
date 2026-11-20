use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_midp::classes::{
    javax::microedition::lcdui::{Command, CommandListener},
    net::wie::NOKIA_KEY_CODES,
};

const KEY_CODES: [(&str, i32, i32); 9] = NOKIA_KEY_CODES;

// abstract class com.nokia.mid.ui.FullCanvas
pub struct FullCanvas;

impl FullCanvas {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "com/nokia/mid/ui/FullCanvas",
            parent_class: Some("javax/microedition/lcdui/Canvas"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::cl_init, MethodAccessFlags::STATIC),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "addCommand",
                    "(Ljavax/microedition/lcdui/Command;)V",
                    Self::add_command,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "setCommandListener",
                    "(Ljavax/microedition/lcdui/CommandListener;)V",
                    Self::set_command_listener,
                    MethodAccessFlags::PUBLIC,
                ),
            ],
            fields: KEY_CODES
                .iter()
                .map(|(name, _, _)| JavaFieldProto::new(name, "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL))
                .collect(),
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::ABSTRACT,
        }
    }

    async fn cl_init(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.ui.FullCanvas::<clinit>()");

        for (name, _, code) in KEY_CODES {
            jvm.put_static_field("com/nokia/mid/ui/FullCanvas", name, "I", code).await?;
        }
        Ok(())
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.ui.FullCanvas::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "javax/microedition/lcdui/Canvas", "<init>", "()V", ()).await?;
        jvm.invoke_special(&this, "javax/microedition/lcdui/Canvas", "setFullScreenMode", "(Z)V", (true,))
            .await
    }

    async fn add_command(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, command: ClassInstanceRef<Command>) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.ui.FullCanvas::addCommand({this:?}, {command:?})");

        Err(jvm
            .exception("java/lang/IllegalStateException", "FullCanvas does not support commands")
            .await)
    }

    async fn set_command_listener(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        listener: ClassInstanceRef<CommandListener>,
    ) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.ui.FullCanvas::setCommandListener({this:?}, {listener:?})");

        Err(jvm
            .exception("java/lang/IllegalStateException", "FullCanvas does not support commands")
            .await)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec};

    use jvm::{ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
    use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
    use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

    use test_utils::run_jvm_test;
    use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
    use wie_midp::classes::{
        javax::microedition::lcdui::Graphics,
        net::wie::{KeyboardEventType, MIDPKeyCode},
    };

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<()>) -> JvmResult<()> {
        jvm.invoke_special(&this, "com/nokia/mid/ui/FullCanvas", "<init>", "()V", ()).await
    }

    async fn paint(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<()>, _: ClassInstanceRef<Graphics>) -> JvmResult<()> {
        Ok(())
    }

    async fn key_pressed(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>, code: i32) -> JvmResult<()> {
        jvm.put_field(&mut this, "test/FullCanvas", "pressed", "I", code).await
    }

    #[test]
    fn full_canvas_initializes_and_translates_key_events() {
        let canvas_proto = WieJavaClassProto {
            name: "test/FullCanvas",
            parent_class: Some("com/nokia/mid/ui/FullCanvas"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paint", "(Ljavax/microedition/lcdui/Graphics;)V", paint, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new("keyPressed", "(I)V", key_pressed, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![JavaFieldProto::new("pressed", "I", FieldAccessFlags::PUBLIC)],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test(
            Box::new([wie_midp::get_protos().into(), crate::get_protos().into(), [canvas_proto].into()]),
            |jvm| async move {
                let canvas = jvm.new_class("test/FullCanvas", "()V", ()).await?;
                let title = JavaLangString::from_rust_string(&jvm, "Fullscreen canvas").await?;
                let _: () = jvm
                    .invoke_virtual(
                        &canvas,
                        "javax/microedition/lcdui/Displayable",
                        "setTitle",
                        "(Ljava/lang/String;)V",
                        (title,),
                    )
                    .await?;
                assert_eq!(
                    jvm.invoke_virtual::<_, i32>(&canvas, "javax/microedition/lcdui/Canvas", "getHeight", "()I", ())
                        .await?,
                    240
                );

                let display = jvm.new_class("javax/microedition/lcdui/Display", "()V", ()).await?;
                let _: () = jvm
                    .invoke_virtual(
                        &display,
                        "javax/microedition/lcdui/Display",
                        "setCurrent",
                        "(Ljavax/microedition/lcdui/Displayable;)V",
                        (canvas.clone(),),
                    )
                    .await?;
                let _: () = jvm
                    .invoke_virtual(
                        &display,
                        "javax/microedition/lcdui/Display",
                        "handleKeyEvent",
                        "(II)V",
                        (KeyboardEventType::KeyPressed as i32, MIDPKeyCode::UP as i32),
                    )
                    .await?;
                assert_eq!(jvm.get_field::<i32>(&canvas, "test/FullCanvas", "pressed", "I").await?, -1);
                // the soft keys have no game action, the arrows have theirs
                for (code, action) in [(-6, 0), (-7, 0), (-2, 6), (-5, 8)] {
                    let got: i32 = jvm
                        .invoke_virtual(&canvas, "javax/microedition/lcdui/Canvas", "getGameAction", "(I)I", (code,))
                        .await?;
                    assert_eq!(got, action, "game action of {code}");
                }
                Ok(())
            },
        )
        .unwrap();
    }
}

use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::{
    javax::microedition::lcdui::{Graphics, Image},
    net::wie::{KeyboardEventType, MIDPKeyCode},
};

// The bits of getKeyStates: 1 << the game action of the key.
const KEY_STATES: [(&str, i32); 9] = [
    ("UP_PRESSED", 1 << 1),
    ("DOWN_PRESSED", 1 << 6),
    ("LEFT_PRESSED", 1 << 2),
    ("RIGHT_PRESSED", 1 << 5),
    ("FIRE_PRESSED", 1 << 8),
    ("GAME_A_PRESSED", 1 << 9),
    ("GAME_B_PRESSED", 1 << 10),
    ("GAME_C_PRESSED", 1 << 11),
    ("GAME_D_PRESSED", 1 << 12),
];

// The getKeyStates bit of one of our key codes; 0 for a key with no game action.
fn key_state_bit(code: i32) -> i32 {
    let action = match MIDPKeyCode::from_raw(code) {
        Some(MIDPKeyCode::UP) => 1,
        Some(MIDPKeyCode::DOWN) => 6,
        Some(MIDPKeyCode::LEFT) => 2,
        Some(MIDPKeyCode::RIGHT) => 5,
        Some(MIDPKeyCode::FIRE) => 8,
        Some(MIDPKeyCode::KEY_NUM1) => 9,
        Some(MIDPKeyCode::KEY_NUM3) => 10,
        Some(MIDPKeyCode::KEY_NUM7) => 11,
        Some(MIDPKeyCode::KEY_NUM9) => 12,
        _ => return 0,
    };
    1 << action
}

// class javax.microedition.lcdui.game.GameCanvas
pub struct GameCanvas;

impl GameCanvas {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "javax/microedition/lcdui/game/GameCanvas",
            parent_class: Some("javax/microedition/lcdui/Canvas"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(Z)V", Self::init, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "getGraphics",
                    "()Ljavax/microedition/lcdui/Graphics;",
                    Self::get_graphics,
                    MethodAccessFlags::PROTECTED,
                ),
                JavaMethodProto::new("<clinit>", "()V", Self::cl_init, MethodAccessFlags::STATIC),
                JavaMethodProto::new("flushGraphics", "()V", Self::flush_graphics, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("flushGraphics", "(IIII)V", Self::flush_graphics_area, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getKeyStates", "()I", Self::get_key_states, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("handleKeyEvent", "(II)V", Self::handle_key_event, MethodAccessFlags::PROTECTED),
                JavaMethodProto::new(
                    "paint",
                    "(Ljavax/microedition/lcdui/Graphics;)V",
                    Self::paint,
                    MethodAccessFlags::PROTECTED,
                ),
            ],
            fields: [
                JavaFieldProto::new("offscreenImage", "Ljavax/microedition/lcdui/Image;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("suppressKeyEvents", "Z", FieldAccessFlags::PRIVATE),
                // the game action keys held now, and those pressed since the last getKeyStates
                JavaFieldProto::new("keyStates", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("latchedKeyStates", "I", FieldAccessFlags::PRIVATE),
            ]
            .into_iter()
            .chain(
                KEY_STATES
                    .iter()
                    .map(|(name, _)| JavaFieldProto::new(name, "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL)),
            )
            .collect(),
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, suppress_key_events: bool) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.GameCanvas::<init>({this:?}, {suppress_key_events})");

        let _: () = jvm.invoke_special(&this, "javax/microedition/lcdui/Canvas", "<init>", "()V", ()).await?;
        jvm.put_field(
            &mut this,
            "javax/microedition/lcdui/game/GameCanvas",
            "suppressKeyEvents",
            "Z",
            suppress_key_events,
        )
        .await?;

        // The buffer must cover the full screen even when the reported Canvas area is smaller.
        let screen = context.system().platform().screen();
        let (width, height) = (screen.width() as i32, screen.height() as i32);

        let image: ClassInstanceRef<Image> = jvm
            .invoke_static(
                "javax/microedition/lcdui/Image",
                "createImage",
                "(II)Ljavax/microedition/lcdui/Image;",
                (width, height),
            )
            .await?;

        jvm.put_field(
            &mut this,
            "javax/microedition/lcdui/game/GameCanvas",
            "offscreenImage",
            "Ljavax/microedition/lcdui/Image;",
            image,
        )
        .await?;

        Ok(())
    }

    async fn cl_init(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.GameCanvas::<clinit>()");

        for (name, value) in KEY_STATES {
            jvm.put_static_field("javax/microedition/lcdui/game/GameCanvas", name, "I", value).await?;
        }
        Ok(())
    }

    async fn handle_key_event(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        event_type: i32,
        code: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.GameCanvas::handleKeyEvent({this:?}, {event_type}, {code})");

        let bit = key_state_bit(code);
        if bit != 0 {
            let class = "javax/microedition/lcdui/game/GameCanvas";
            let states: i32 = jvm.get_field(&this, class, "keyStates", "I").await?;
            if event_type == KeyboardEventType::KeyPressed as i32 {
                let latched: i32 = jvm.get_field(&this, class, "latchedKeyStates", "I").await?;
                jvm.put_field(&mut this, class, "keyStates", "I", states | bit).await?;
                jvm.put_field(&mut this, class, "latchedKeyStates", "I", latched | bit).await?;
            } else if event_type == KeyboardEventType::KeyReleased as i32 {
                jvm.put_field(&mut this, class, "keyStates", "I", states & !bit).await?;
            }
            // with suppressKeyEvents, the game action keys reach the app only through getKeyStates
            if jvm.get_field(&this, class, "suppressKeyEvents", "Z").await? {
                return Ok(());
            }
        }

        jvm.invoke_special(&this, "javax/microedition/lcdui/Canvas", "handleKeyEvent", "(II)V", (event_type, code))
            .await
    }

    async fn get_key_states(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.GameCanvas::getKeyStates({this:?})");

        let class = "javax/microedition/lcdui/game/GameCanvas";
        let states: i32 = jvm.get_field(&this, class, "keyStates", "I").await?;
        let latched: i32 = jvm.get_field(&this, class, "latchedKeyStates", "I").await?;
        jvm.put_field(&mut this, class, "latchedKeyStates", "I", 0).await?;

        Ok(states | latched)
    }

    async fn get_graphics(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Graphics>> {
        tracing::debug!("javax.microedition.lcdui.game.GameCanvas::getGraphics({this:?})");

        let offscreen_image: ClassInstanceRef<Image> = jvm
            .get_field(
                &this,
                "javax/microedition/lcdui/game/GameCanvas",
                "offscreenImage",
                "Ljavax/microedition/lcdui/Image;",
            )
            .await?;
        let graphics = jvm
            .new_class(
                "javax/microedition/lcdui/Graphics",
                "(Ljavax/microedition/lcdui/Image;)V",
                (offscreen_image,),
            )
            .await?;

        Ok(graphics.into())
    }

    async fn flush_graphics(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.GameCanvas::flushGraphics({this:?})");

        let _: () = jvm.invoke_virtual(&this, "javax/microedition/lcdui/Canvas", "repaint", "()V", ()).await?;

        Ok(())
    }

    async fn flush_graphics_area(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.GameCanvas::flushGraphics({this:?}, {x}, {y}, {width}, {height})");

        if width <= 0 || height <= 0 {
            return Ok(());
        }
        jvm.invoke_virtual(&this, "javax/microedition/lcdui/Canvas", "repaint", "(IIII)V", (x, y, width, height))
            .await
    }

    async fn paint(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, g: ClassInstanceRef<Graphics>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.GameCanvas::paint({this:?}, {g:?})");

        let offscreen_image: ClassInstanceRef<Image> = jvm
            .get_field(
                &this,
                "javax/microedition/lcdui/game/GameCanvas",
                "offscreenImage",
                "Ljavax/microedition/lcdui/Image;",
            )
            .await?;

        let _: () = jvm
            .invoke_virtual(
                &g,
                "javax/microedition/lcdui/Graphics",
                "drawImage",
                "(Ljavax/microedition/lcdui/Image;III)V",
                (offscreen_image, 0, 0, 0),
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec};

    use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
    use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
    use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
    use test_utils::run_jvm_test;
    use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
    use wie_util::Result;

    use crate::{
        classes::{
            javax::microedition::lcdui::Graphics,
            net::wie::{KeyboardEventType, MIDPKeyCode},
        },
        get_protos,
    };

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<()>) -> JvmResult<()> {
        jvm.invoke_special(&this, "javax/microedition/lcdui/game/GameCanvas", "<init>", "(Z)V", (true,))
            .await
    }

    async fn paint(_: &Jvm, _: &mut WieJvmContext, _: ClassInstanceRef<()>, _: ClassInstanceRef<Graphics>) -> JvmResult<()> {
        Ok(())
    }

    async fn key_pressed(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<()>, code: i32) -> JvmResult<()> {
        jvm.put_field(&mut this, "test/KeyStatesCanvas", "pressed", "I", code).await
    }

    #[test]
    fn key_states_latch_and_suppress_game_keys() -> Result<()> {
        let canvas_proto = WieJavaClassProto {
            name: "test/KeyStatesCanvas",
            parent_class: Some("javax/microedition/lcdui/game/GameCanvas"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("paint", "(Ljavax/microedition/lcdui/Graphics;)V", paint, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("keyPressed", "(I)V", key_pressed, MethodAccessFlags::PROTECTED),
            ],
            fields: vec![JavaFieldProto::new("pressed", "I", FieldAccessFlags::PUBLIC)],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test(Box::new([get_protos().into(), [canvas_proto].into()]), |jvm| async move {
            let canvas = jvm.new_class("test/KeyStatesCanvas", "()V", ()).await?;
            let class = "javax/microedition/lcdui/game/GameCanvas";
            let key = async |event_type: KeyboardEventType, code: MIDPKeyCode| -> JvmResult<()> {
                jvm.invoke_virtual(
                    &canvas,
                    "javax/microedition/lcdui/Displayable",
                    "handleKeyEvent",
                    "(II)V",
                    (event_type as i32, code as i32),
                )
                .await
            };
            let down: i32 = jvm.get_static_field(class, "DOWN_PRESSED", "I").await?;
            let fire: i32 = jvm.get_static_field(class, "FIRE_PRESSED", "I").await?;

            // held
            key(KeyboardEventType::KeyPressed, MIDPKeyCode::DOWN).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&canvas, class, "getKeyStates", "()I", ()).await?, down);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&canvas, class, "getKeyStates", "()I", ()).await?, down);
            key(KeyboardEventType::KeyReleased, MIDPKeyCode::DOWN).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&canvas, class, "getKeyStates", "()I", ()).await?, 0);
            // pressed and released between two calls
            key(KeyboardEventType::KeyPressed, MIDPKeyCode::FIRE).await?;
            key(KeyboardEventType::KeyReleased, MIDPKeyCode::FIRE).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&canvas, class, "getKeyStates", "()I", ()).await?, fire);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&canvas, class, "getKeyStates", "()I", ()).await?, 0);

            // suppressed: the game keys do not reach keyPressed, the others do
            assert_eq!(jvm.get_field::<i32>(&canvas, "test/KeyStatesCanvas", "pressed", "I").await?, 0);
            key(KeyboardEventType::KeyPressed, MIDPKeyCode::KEY_NUM5).await?;
            assert_eq!(
                jvm.get_field::<i32>(&canvas, "test/KeyStatesCanvas", "pressed", "I").await?,
                MIDPKeyCode::KEY_NUM5 as i32
            );

            Ok(())
        })
    }
}

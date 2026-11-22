use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class javax.microedition.lcdui.game.Layer
pub struct Layer;

impl Layer {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "javax/microedition/lcdui/game/Layer",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(II)V", Self::init, MethodAccessFlags::empty()),
                JavaMethodProto::new("setPosition", "(II)V", Self::set_position, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("move", "(II)V", Self::r#move, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getX", "()I", Self::get_x, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new("getY", "()I", Self::get_y, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new("getWidth", "()I", Self::get_width, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new("getHeight", "()I", Self::get_height, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new("setVisible", "(Z)V", Self::set_visible, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isVisible", "()Z", Self::is_visible, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                // sets the size a subclass reports (the tiled area of a TiledLayer, the frame of a Sprite)
                JavaMethodProto::new("setSize", "(II)V", Self::set_size, MethodAccessFlags::empty()),
                JavaMethodProto::new_abstract(
                    "paint",
                    "(Ljavax/microedition/lcdui/Graphics;)V",
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("x", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("y", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("width", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("height", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("visible", "Z", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::ABSTRACT,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, width: i32, height: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::<init>({this:?}, {width}, {height})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(&mut this, "javax/microedition/lcdui/game/Layer", "visible", "Z", true)
            .await?;
        jvm.invoke_virtual(&this, "javax/microedition/lcdui/game/Layer", "setSize", "(II)V", (width, height))
            .await
    }

    async fn set_position(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, x: i32, y: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::setPosition({this:?}, {x}, {y})");

        jvm.put_field(&mut this, "javax/microedition/lcdui/game/Layer", "x", "I", x).await?;
        jvm.put_field(&mut this, "javax/microedition/lcdui/game/Layer", "y", "I", y).await
    }

    async fn r#move(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, dx: i32, dy: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::move({this:?}, {dx}, {dy})");

        let x: i32 = jvm.get_field(&this, "javax/microedition/lcdui/game/Layer", "x", "I").await?;
        let y: i32 = jvm.get_field(&this, "javax/microedition/lcdui/game/Layer", "y", "I").await?;
        jvm.invoke_virtual(
            &this,
            "javax/microedition/lcdui/game/Layer",
            "setPosition",
            "(II)V",
            (x.wrapping_add(dx), y.wrapping_add(dy)),
        )
        .await
    }

    async fn get_x(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::getX({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/game/Layer", "x", "I").await
    }

    async fn get_y(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::getY({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/game/Layer", "y", "I").await
    }

    async fn get_width(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::getWidth({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/game/Layer", "width", "I").await
    }

    async fn get_height(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::getHeight({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/game/Layer", "height", "I").await
    }

    async fn set_visible(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, visible: bool) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::setVisible({this:?}, {visible})");

        jvm.put_field(&mut this, "javax/microedition/lcdui/game/Layer", "visible", "Z", visible)
            .await
    }

    async fn is_visible(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::isVisible({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/game/Layer", "visible", "Z").await
    }

    async fn set_size(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, width: i32, height: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Layer::setSize({this:?}, {width}, {height})");

        jvm.put_field(&mut this, "javax/microedition/lcdui/game/Layer", "width", "I", width)
            .await?;
        jvm.put_field(&mut this, "javax/microedition/lcdui/game/Layer", "height", "I", height)
            .await
    }
}

use alloc::vec;

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_midp::classes::javax::microedition::lcdui::{Graphics, Image};

use crate::classes::com::nokia::mid::ui::DirectGraphics;

// class com.nokia.mid.ui.DirectUtils
pub struct DirectUtils;

impl DirectUtils {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "com/nokia/mid/ui/DirectUtils",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "getDirectGraphics",
                    "(Ljavax/microedition/lcdui/Graphics;)Lcom/nokia/mid/ui/DirectGraphics;",
                    Self::get_direct_graphics,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "createImage",
                    "(III)Ljavax/microedition/lcdui/Image;",
                    Self::create_image,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "createImage",
                    "([BII)Ljavax/microedition/lcdui/Image;",
                    Self::create_image_from_data,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
            ],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn get_direct_graphics(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        graphics: ClassInstanceRef<Graphics>,
    ) -> JvmResult<ClassInstanceRef<DirectGraphics>> {
        tracing::debug!("com.nokia.mid.ui.DirectUtils::getDirectGraphics({graphics:?})");

        Ok(jvm
            .new_class("net/wie/DirectGraphicsImpl", "(Ljavax/microedition/lcdui/Graphics;)V", (graphics,))
            .await?
            .into())
    }

    // Mutable image whose every pixel starts as the given ARGB color, so it can be transparent.
    async fn create_image(jvm: &Jvm, _: &mut WieJvmContext, width: i32, height: i32, argb: i32) -> JvmResult<ClassInstanceRef<Image>> {
        tracing::debug!("com.nokia.mid.ui.DirectUtils::createImage({width}, {height}, {argb:#x})");

        if width <= 0 || height <= 0 {
            return Err(jvm
                .exception("java/lang/IllegalArgumentException", "image dimensions must be positive")
                .await);
        }

        let count = (width * height) as usize;
        let mut rgb = jvm.instantiate_array("I", count).await?;
        jvm.store_array(&mut rgb, 0, vec![argb; count]).await?;

        let mut image: ClassInstanceRef<Image> = jvm
            .invoke_static(
                "javax/microedition/lcdui/Image",
                "createRGBImage",
                "([IIIZ)Ljavax/microedition/lcdui/Image;",
                (rgb, width, height, true),
            )
            .await?;
        jvm.put_field(&mut image, "javax/microedition/lcdui/Image", "mutable", "Z", true).await?;

        Ok(image)
    }

    // Mutable image decoded from image file data.
    async fn create_image_from_data(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        data: ClassInstanceRef<Array<i8>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<ClassInstanceRef<Image>> {
        tracing::debug!("com.nokia.mid.ui.DirectUtils::createImage({data:?}, {offset}, {length})");

        let mut image: ClassInstanceRef<Image> = jvm
            .invoke_static(
                "javax/microedition/lcdui/Image",
                "createImage",
                "([BII)Ljavax/microedition/lcdui/Image;",
                (data, offset, length),
            )
            .await?;
        jvm.put_field(&mut image, "javax/microedition/lcdui/Image", "mutable", "Z", true).await?;

        Ok(image)
    }
}

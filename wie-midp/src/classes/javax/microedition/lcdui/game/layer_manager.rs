use alloc::vec;

use jvm::{ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::util::Vector;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::javax::microedition::lcdui::Graphics;

const CLASS: &str = "javax/microedition/lcdui/game/LayerManager";
const LAYER: &str = "javax/microedition/lcdui/game/Layer";
const GRAPHICS: &str = "javax/microedition/lcdui/Graphics";

// class javax.microedition.lcdui.game.LayerManager
pub struct LayerManager;

impl LayerManager {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: CLASS,
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "append",
                    "(Ljavax/microedition/lcdui/game/Layer;)V",
                    Self::append,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "insert",
                    "(Ljavax/microedition/lcdui/game/Layer;I)V",
                    Self::insert,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "remove",
                    "(Ljavax/microedition/lcdui/game/Layer;)V",
                    Self::remove,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "getLayerAt",
                    "(I)Ljavax/microedition/lcdui/game/Layer;",
                    Self::get_layer_at,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getSize", "()I", Self::get_size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setViewWindow", "(IIII)V", Self::set_view_window, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "paint",
                    "(Ljavax/microedition/lcdui/Graphics;II)V",
                    Self::paint,
                    MethodAccessFlags::PUBLIC,
                ),
            ],
            fields: vec![
                // index 0 is the layer nearest the viewer
                JavaFieldProto::new("layers", "Ljava/util/Vector;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("viewX", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("viewY", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("viewWidth", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("viewHeight", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        let layers = jvm.new_class("java/util/Vector", "()V", ()).await?;
        jvm.put_field(&mut this, CLASS, "layers", "Ljava/util/Vector;", layers).await?;

        jvm.invoke_virtual(&this, CLASS, "setViewWindow", "(IIII)V", (0, 0, i32::MAX, i32::MAX))
            .await
    }

    async fn layers(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Vector>> {
        jvm.get_field(this, CLASS, "layers", "Ljava/util/Vector;").await
    }

    async fn append(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, layer: ClassInstanceRef<()>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::append({this:?}, {layer:?})");

        let layers = Self::layers(jvm, &this).await?;
        let _: bool = jvm
            .invoke_virtual(&layers, "java/util/Vector", "removeElement", "(Ljava/lang/Object;)Z", (layer.clone(),))
            .await?;
        jvm.invoke_virtual(&layers, "java/util/Vector", "addElement", "(Ljava/lang/Object;)V", (layer,))
            .await
    }

    async fn insert(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, layer: ClassInstanceRef<()>, index: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::insert({this:?}, {layer:?}, {index})");

        let layers = Self::layers(jvm, &this).await?;
        let _: bool = jvm
            .invoke_virtual(&layers, "java/util/Vector", "removeElement", "(Ljava/lang/Object;)Z", (layer.clone(),))
            .await?;
        jvm.invoke_virtual(&layers, "java/util/Vector", "insertElementAt", "(Ljava/lang/Object;I)V", (layer, index))
            .await
    }

    async fn remove(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, layer: ClassInstanceRef<()>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::remove({this:?}, {layer:?})");

        let layers = Self::layers(jvm, &this).await?;
        let _: bool = jvm
            .invoke_virtual(&layers, "java/util/Vector", "removeElement", "(Ljava/lang/Object;)Z", (layer,))
            .await?;

        Ok(())
    }

    async fn get_layer_at(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<ClassInstanceRef<()>> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::getLayerAt({this:?}, {index})");

        let layers = Self::layers(jvm, &this).await?;
        jvm.invoke_virtual(&layers, "java/util/Vector", "elementAt", "(I)Ljava/lang/Object;", (index,))
            .await
    }

    async fn get_size(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::getSize({this:?})");

        let layers = Self::layers(jvm, &this).await?;
        jvm.invoke_virtual(&layers, "java/util/Vector", "size", "()I", ()).await
    }

    async fn set_view_window(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::setViewWindow({this:?}, {x}, {y}, {width}, {height})");

        jvm.put_field(&mut this, CLASS, "viewX", "I", x).await?;
        jvm.put_field(&mut this, CLASS, "viewY", "I", y).await?;
        jvm.put_field(&mut this, CLASS, "viewWidth", "I", width).await?;
        jvm.put_field(&mut this, CLASS, "viewHeight", "I", height).await
    }

    async fn paint(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        graphics: ClassInstanceRef<Graphics>,
        x: i32,
        y: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.LayerManager::paint({this:?}, {graphics:?}, {x}, {y})");

        let view_x: i32 = jvm.get_field(&this, CLASS, "viewX", "I").await?;
        let view_y: i32 = jvm.get_field(&this, CLASS, "viewY", "I").await?;
        let view_width: i32 = jvm.get_field(&this, CLASS, "viewWidth", "I").await?;
        let view_height: i32 = jvm.get_field(&this, CLASS, "viewHeight", "I").await?;

        let clip_x: i32 = jvm.invoke_virtual(&graphics, GRAPHICS, "getClipX", "()I", ()).await?;
        let clip_y: i32 = jvm.invoke_virtual(&graphics, GRAPHICS, "getClipY", "()I", ()).await?;
        let clip_width: i32 = jvm.invoke_virtual(&graphics, GRAPHICS, "getClipWidth", "()I", ()).await?;
        let clip_height: i32 = jvm.invoke_virtual(&graphics, GRAPHICS, "getClipHeight", "()I", ()).await?;

        // The view window shows at (x, y), clipped to its size; its origin is (viewX, viewY) of the layers.
        let _: () = jvm
            .invoke_virtual(&graphics, GRAPHICS, "clipRect", "(IIII)V", (x, y, view_width, view_height))
            .await?;
        let _: () = jvm
            .invoke_virtual(&graphics, GRAPHICS, "translate", "(II)V", (x - view_x, y - view_y))
            .await?;

        let layers = Self::layers(jvm, &this).await?;
        let count: i32 = jvm.invoke_virtual(&layers, "java/util/Vector", "size", "()I", ()).await?;
        for index in (0..count).rev() {
            let layer: ClassInstanceRef<()> = jvm
                .invoke_virtual(&layers, "java/util/Vector", "elementAt", "(I)Ljava/lang/Object;", (index,))
                .await?;
            let visible: bool = jvm.invoke_virtual(&layer, LAYER, "isVisible", "()Z", ()).await?;
            if visible {
                let _: () = jvm
                    .invoke_virtual(&layer, LAYER, "paint", "(Ljavax/microedition/lcdui/Graphics;)V", (graphics.clone(),))
                    .await?;
            }
        }

        let _: () = jvm
            .invoke_virtual(&graphics, GRAPHICS, "translate", "(II)V", (view_x - x, view_y - y))
            .await?;
        jvm.invoke_virtual(&graphics, GRAPHICS, "setClip", "(IIII)V", (clip_x, clip_y, clip_width, clip_height))
            .await
    }
}

#[cfg(test)]
mod test {
    use alloc::boxed::Box;

    use jvm::{ClassInstance, ClassInstanceRef, JavaValue};

    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::{classes::javax::microedition::lcdui::Image, get_protos};

    const RED: i32 = 0xffff0000u32 as i32;
    const BLUE: i32 = 0xff0000ffu32 as i32;
    const WHITE: i32 = -1;

    #[test]
    fn tiled_layer_paints_through_the_view_window() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            // two 2x2 tiles side by side: 1 red, 2 blue
            let mut rgb = jvm.instantiate_array("I", 8).await?;
            jvm.store_array(&mut rgb, 0, [RED, RED, BLUE, BLUE, RED, RED, BLUE, BLUE]).await?;
            let tiles: ClassInstanceRef<Image> = jvm
                .invoke_static(
                    "javax/microedition/lcdui/Image",
                    "createRGBImage",
                    "([IIIZ)Ljavax/microedition/lcdui/Image;",
                    (rgb, 4, 2, false),
                )
                .await?;

            // three cells: tile 1, empty, an animated tile showing tile 2
            let layer = jvm
                .new_class(
                    "javax/microedition/lcdui/game/TiledLayer",
                    "(IILjavax/microedition/lcdui/Image;II)V",
                    (3, 1, tiles, 2, 2),
                )
                .await?;
            let class = "javax/microedition/lcdui/game/TiledLayer";
            let animated: i32 = jvm.invoke_virtual(&layer, class, "createAnimatedTile", "(I)I", (2,)).await?;
            assert_eq!(animated, -1);
            let _: () = jvm.invoke_virtual(&layer, class, "setCell", "(III)V", (0, 0, 1)).await?;
            let _: () = jvm.invoke_virtual(&layer, class, "setCell", "(III)V", (2, 0, animated)).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&layer, class, "getCell", "(II)I", (2, 0)).await?, -1);
            let width: i32 = jvm
                .invoke_virtual(&layer, "javax/microedition/lcdui/game/Layer", "getWidth", "()I", ())
                .await?;
            assert_eq!(width, 6);
            let _: () = jvm
                .invoke_virtual(&layer, "javax/microedition/lcdui/game/Layer", "setPosition", "(II)V", (1, 0))
                .await?;

            let manager = jvm.new_class("javax/microedition/lcdui/game/LayerManager", "()V", ()).await?;
            let manager_class = "javax/microedition/lcdui/game/LayerManager";
            let _: () = jvm
                .invoke_virtual(
                    &manager,
                    manager_class,
                    "append",
                    "(Ljavax/microedition/lcdui/game/Layer;)V",
                    (layer.clone(),),
                )
                .await?;
            // layer x 2..7 seen at screen x 10..15; the third cell (layer x 5..7) is cut off at screen x 13
            let _: () = jvm
                .invoke_virtual(&manager, manager_class, "setViewWindow", "(IIII)V", (2, 0, 4, 2))
                .await?;

            let screen: ClassInstanceRef<Image> = jvm
                .invoke_static(
                    "javax/microedition/lcdui/Image",
                    "createImage",
                    "(II)Ljavax/microedition/lcdui/Image;",
                    (20, 4),
                )
                .await?;
            let graphics: Box<dyn ClassInstance> = jvm
                .invoke_virtual(
                    &screen,
                    "javax/microedition/lcdui/Image",
                    "getGraphics",
                    "()Ljavax/microedition/lcdui/Graphics;",
                    (),
                )
                .await?;
            let paint = async |expected: [i32; 6]| -> jvm::Result<()> {
                let _: () = jvm
                    .invoke_virtual(
                        &manager,
                        manager_class,
                        "paint",
                        "(Ljavax/microedition/lcdui/Graphics;II)V",
                        [graphics.clone().into(), 10.into(), 0.into()] as [JavaValue; 3],
                    )
                    .await?;
                let row = jvm.instantiate_array("I", 6).await?;
                let _: () = jvm
                    .invoke_virtual(
                        &screen,
                        "javax/microedition/lcdui/Image",
                        "getRGB",
                        "([IIIIIII)V",
                        (row.clone(), 0, 6, 9, 1, 6, 1),
                    )
                    .await?;
                assert_eq!(jvm.load_array::<i32>(&row, 0, 6).await?, expected);
                Ok(())
            };
            // screen x 9..15: outside, cell 0 (half of it in view), empty cell 1, cell 2 up to the view's edge, outside
            paint([WHITE, RED, WHITE, WHITE, BLUE, WHITE]).await?;

            // the animated tile now shows tile 1; the view and clip are restored after each paint
            let _: () = jvm.invoke_virtual(&layer, class, "setAnimatedTile", "(II)V", (animated, 1)).await?;
            paint([WHITE, RED, WHITE, WHITE, RED, WHITE]).await?;
            let translate_x: i32 = jvm
                .invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "getTranslateX", "()I", ())
                .await?;
            assert_eq!(translate_x, 0);

            Ok(())
        })
    }
}

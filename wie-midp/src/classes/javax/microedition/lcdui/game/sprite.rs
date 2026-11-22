use alloc::{vec, vec::Vec};

use jvm::{Array, ClassInstanceRef, JavaValue, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::javax::microedition::lcdui::{Graphics, Image, game::TiledLayer};

const CLASS: &str = "javax/microedition/lcdui/game/Sprite";
const LAYER: &str = "javax/microedition/lcdui/game/Layer";
const TILED_LAYER: &str = "javax/microedition/lcdui/game/TiledLayer";
const IMAGE: &str = "javax/microedition/lcdui/Image";
// Graphics.TOP | Graphics.LEFT
const TOP_LEFT: i32 = 0x10 | 0x04;

/// Where pixel (x, y) of a width x height frame lands once the frame is drawn with `transform`.
fn transform_point(transform: i32, x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    match transform {
        1 => (x, height - 1 - y),             // TRANS_MIRROR_ROT180
        2 => (width - 1 - x, y),              // TRANS_MIRROR
        3 => (width - 1 - x, height - 1 - y), // TRANS_ROT180
        4 => (y, x),                          // TRANS_MIRROR_ROT270
        5 => (height - 1 - y, x),             // TRANS_ROT90
        6 => (y, width - 1 - x),              // TRANS_ROT270
        7 => (height - 1 - y, width - 1 - x), // TRANS_MIRROR_ROT90
        _ => (x, y),                          // TRANS_NONE
    }
}

/// A rectangle of a width x height frame once the frame is drawn with `transform`.
fn transform_rect(transform: i32, rect: Rect, width: i32, height: i32) -> Rect {
    if rect.width <= 0 || rect.height <= 0 {
        return Rect::default();
    }
    let (x1, y1) = transform_point(transform, rect.x, rect.y, width, height);
    let (x2, y2) = transform_point(transform, rect.x + rect.width - 1, rect.y + rect.height - 1, width, height);

    Rect {
        x: x1.min(x2),
        y: y1.min(y2),
        width: (x1 - x2).abs() + 1,
        height: (y1 - y2).abs() + 1,
    }
}

#[derive(Clone, Copy, Default)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Rect {
    fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);

        (right > x && bottom > y).then_some(Rect {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

/// ARGB pixels placed on the screen, for pixel level collisions.
struct Pixels {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    argb: Vec<u32>,
}

impl Pixels {
    async fn of_image(jvm: &Jvm, image: &ClassInstanceRef<Image>, x: i32, y: i32) -> JvmResult<Self> {
        let source = Image::image(jvm, image).await?;
        let (width, height) = (source.width() as i32, source.height() as i32);
        let (_, _, argb) = Image::transform_region(&*source, 0, 0, width, height, 0).unwrap_or_default();

        Ok(Self { x, y, width, height, argb })
    }

    fn opaque(&self, x: i32, y: i32) -> bool {
        let (x, y) = (x - self.x, y - self.y);
        x >= 0 && y >= 0 && x < self.width && y < self.height && self.argb[(y * self.width + x) as usize] >> 24 != 0
    }
}

/// Whether a pixel in `area` is opaque in `a` and in `b` moved by `b_offset`.
fn any_opaque_in_both(area: &Rect, a: &Pixels, b: &Pixels, b_offset: (i32, i32)) -> bool {
    (area.y..area.y + area.height).any(|y| (area.x..area.x + area.width).any(|x| a.opaque(x, y) && b.opaque(x - b_offset.0, y - b_offset.1)))
}

// class javax.microedition.lcdui.game.Sprite
pub struct Sprite;

impl Sprite {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: CLASS,
            parent_class: Some(LAYER),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "(Ljavax/microedition/lcdui/Image;)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljavax/microedition/lcdui/Image;II)V",
                    Self::init_with_frame_size,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "setImage",
                    "(Ljavax/microedition/lcdui/Image;II)V",
                    Self::set_image,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("defineReferencePixel", "(II)V", Self::define_reference_pixel, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setRefPixelPosition", "(II)V", Self::set_ref_pixel_position, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getRefPixelX", "()I", Self::get_ref_pixel_x, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getRefPixelY", "()I", Self::get_ref_pixel_y, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFrame", "(I)V", Self::set_frame, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getFrame", "()I", Self::get_frame, MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL),
                JavaMethodProto::new("getRawFrameCount", "()I", Self::get_raw_frame_count, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getFrameSequenceLength",
                    "()I",
                    Self::get_frame_sequence_length,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("nextFrame", "()V", Self::next_frame, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("prevFrame", "()V", Self::prev_frame, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFrameSequence", "([I)V", Self::set_frame_sequence, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setTransform", "(I)V", Self::set_transform, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "defineCollisionRectangle",
                    "(IIII)V",
                    Self::define_collision_rectangle,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "collidesWith",
                    "(Ljavax/microedition/lcdui/game/Sprite;Z)Z",
                    Self::collides_with_sprite,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new(
                    "collidesWith",
                    "(Ljavax/microedition/lcdui/game/TiledLayer;Z)Z",
                    Self::collides_with_tiled_layer,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new(
                    "collidesWith",
                    "(Ljavax/microedition/lcdui/Image;IIZ)Z",
                    Self::collides_with_image,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
                JavaMethodProto::new(
                    "paint",
                    "(Ljavax/microedition/lcdui/Graphics;)V",
                    Self::paint,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::FINAL,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("image", "Ljavax/microedition/lcdui/Image;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("frameWidth", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("frameHeight", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("rawFrameCount", "I", FieldAccessFlags::PRIVATE),
                // raw frame of each sequence position
                JavaFieldProto::new("frameSequence", "[I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("sequenceIndex", "I", FieldAccessFlags::PRIVATE),
                // the reference pixel and the collision rectangle are kept in untransformed frame coordinates
                JavaFieldProto::new("refX", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("refY", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("collisionX", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("collisionY", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("collisionWidth", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("collisionHeight", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("transform", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, image: ClassInstanceRef<Image>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::<init>({this:?}, {image:?})");

        let width: i32 = jvm.invoke_virtual(&image, IMAGE, "getWidth", "()I", ()).await?;
        let height: i32 = jvm.invoke_virtual(&image, IMAGE, "getHeight", "()I", ()).await?;
        jvm.invoke_special(&this, CLASS, "<init>", "(Ljavax/microedition/lcdui/Image;II)V", (image, width, height))
            .await
    }

    async fn init_with_frame_size(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        image: ClassInstanceRef<Image>,
        frame_width: i32,
        frame_height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::<init>({this:?}, {image:?}, {frame_width}, {frame_height})");

        let _: () = jvm.invoke_special(&this, LAYER, "<init>", "(II)V", (frame_width, frame_height)).await?;
        jvm.invoke_virtual(
            &this,
            CLASS,
            "setImage",
            "(Ljavax/microedition/lcdui/Image;II)V",
            (image, frame_width, frame_height),
        )
        .await
    }

    async fn set_image(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        image: ClassInstanceRef<Image>,
        frame_width: i32,
        frame_height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::setImage({this:?}, {image:?}, {frame_width}, {frame_height})");

        let image_width: i32 = jvm.invoke_virtual(&image, IMAGE, "getWidth", "()I", ()).await?;
        let image_height: i32 = jvm.invoke_virtual(&image, IMAGE, "getHeight", "()I", ()).await?;
        if frame_width <= 0 || frame_height <= 0 || image_width % frame_width != 0 || image_height % frame_height != 0 {
            return Err(jvm
                .exception("java/lang/IllegalArgumentException", "image is not a whole number of frames")
                .await);
        }
        let raw_frame_count = (image_width / frame_width) * (image_height / frame_height);

        // The reference pixel stays where it is on the screen.
        let ref_x: i32 = jvm.invoke_virtual(&this, CLASS, "getRefPixelX", "()I", ()).await?;
        let ref_y: i32 = jvm.invoke_virtual(&this, CLASS, "getRefPixelY", "()I", ()).await?;

        // With fewer frames than before, the frame sequence goes back to the default one; with another frame size, the
        // collision rectangle covers the whole frame again.
        let old_raw_frame_count: i32 = jvm.get_field(&this, CLASS, "rawFrameCount", "I").await?;
        let old_frame_width: i32 = jvm.get_field(&this, CLASS, "frameWidth", "I").await?;
        let old_frame_height: i32 = jvm.get_field(&this, CLASS, "frameHeight", "I").await?;
        jvm.put_field(&mut this, CLASS, "image", "Ljavax/microedition/lcdui/Image;", image)
            .await?;
        jvm.put_field(&mut this, CLASS, "frameWidth", "I", frame_width).await?;
        jvm.put_field(&mut this, CLASS, "frameHeight", "I", frame_height).await?;
        jvm.put_field(&mut this, CLASS, "rawFrameCount", "I", raw_frame_count).await?;
        if raw_frame_count < old_raw_frame_count || old_raw_frame_count == 0 {
            let _: () = jvm
                .invoke_virtual(&this, CLASS, "setFrameSequence", "([I)V", (ClassInstanceRef::<Array<i32>>::new(None),))
                .await?;
        }
        if (frame_width, frame_height) != (old_frame_width, old_frame_height) {
            let _: () = jvm
                .invoke_virtual(&this, CLASS, "defineCollisionRectangle", "(IIII)V", (0, 0, frame_width, frame_height))
                .await?;
        }

        let transform: i32 = jvm.get_field(&this, CLASS, "transform", "I").await?;
        let (width, height) = if transform >= 4 {
            (frame_height, frame_width)
        } else {
            (frame_width, frame_height)
        };
        let _: () = jvm.invoke_virtual(&this, LAYER, "setSize", "(II)V", (width, height)).await?;
        jvm.invoke_virtual(&this, CLASS, "setRefPixelPosition", "(II)V", (ref_x, ref_y)).await
    }

    /// The reference pixel's offset from the top left corner of the drawn (transformed) frame.
    async fn transformed_ref_pixel(jvm: &Jvm, this: &ClassInstanceRef<Self>, transform: i32) -> JvmResult<(i32, i32)> {
        let ref_x: i32 = jvm.get_field(this, CLASS, "refX", "I").await?;
        let ref_y: i32 = jvm.get_field(this, CLASS, "refY", "I").await?;
        let frame_width: i32 = jvm.get_field(this, CLASS, "frameWidth", "I").await?;
        let frame_height: i32 = jvm.get_field(this, CLASS, "frameHeight", "I").await?;

        Ok(transform_point(transform, ref_x, ref_y, frame_width, frame_height))
    }

    async fn define_reference_pixel(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, x: i32, y: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::defineReferencePixel({this:?}, {x}, {y})");

        jvm.put_field(&mut this, CLASS, "refX", "I", x).await?;
        jvm.put_field(&mut this, CLASS, "refY", "I", y).await
    }

    async fn set_ref_pixel_position(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, x: i32, y: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::setRefPixelPosition({this:?}, {x}, {y})");

        let transform: i32 = jvm.get_field(&this, CLASS, "transform", "I").await?;
        let (offset_x, offset_y) = Self::transformed_ref_pixel(jvm, &this, transform).await?;
        jvm.invoke_virtual(&this, LAYER, "setPosition", "(II)V", (x - offset_x, y - offset_y))
            .await
    }

    async fn get_ref_pixel_x(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::getRefPixelX({this:?})");

        let transform: i32 = jvm.get_field(&this, CLASS, "transform", "I").await?;
        let (offset_x, _) = Self::transformed_ref_pixel(jvm, &this, transform).await?;
        let x: i32 = jvm.invoke_virtual(&this, LAYER, "getX", "()I", ()).await?;
        Ok(x + offset_x)
    }

    async fn get_ref_pixel_y(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::getRefPixelY({this:?})");

        let transform: i32 = jvm.get_field(&this, CLASS, "transform", "I").await?;
        let (_, offset_y) = Self::transformed_ref_pixel(jvm, &this, transform).await?;
        let y: i32 = jvm.invoke_virtual(&this, LAYER, "getY", "()I", ()).await?;
        Ok(y + offset_y)
    }

    async fn set_frame(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, sequence_index: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::setFrame({this:?}, {sequence_index})");

        let sequence: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "frameSequence", "[I").await?;
        let length = jvm.array_length(&sequence).await? as i32;
        if !(0..length).contains(&sequence_index) {
            return Err(jvm
                .exception("java/lang/IndexOutOfBoundsException", "frame is outside the frame sequence")
                .await);
        }
        jvm.put_field(&mut this, CLASS, "sequenceIndex", "I", sequence_index).await
    }

    async fn get_frame(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::getFrame({this:?})");

        jvm.get_field(&this, CLASS, "sequenceIndex", "I").await
    }

    async fn get_raw_frame_count(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::getRawFrameCount({this:?})");

        jvm.get_field(&this, CLASS, "rawFrameCount", "I").await
    }

    async fn get_frame_sequence_length(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::getFrameSequenceLength({this:?})");

        let sequence: ClassInstanceRef<Array<i32>> = jvm.get_field(&this, CLASS, "frameSequence", "[I").await?;
        Ok(jvm.array_length(&sequence).await? as i32)
    }

    async fn step_frame(jvm: &Jvm, this: &mut ClassInstanceRef<Self>, step: i32) -> JvmResult<()> {
        let sequence: ClassInstanceRef<Array<i32>> = jvm.get_field(this, CLASS, "frameSequence", "[I").await?;
        let length = jvm.array_length(&sequence).await? as i32;
        let index: i32 = jvm.get_field(this, CLASS, "sequenceIndex", "I").await?;
        jvm.put_field(this, CLASS, "sequenceIndex", "I", (index + step).rem_euclid(length)).await
    }

    async fn next_frame(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::nextFrame({this:?})");

        Self::step_frame(jvm, &mut this, 1).await
    }

    async fn prev_frame(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::prevFrame({this:?})");

        Self::step_frame(jvm, &mut this, -1).await
    }

    async fn set_frame_sequence(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        sequence: ClassInstanceRef<Array<i32>>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::setFrameSequence({this:?}, {sequence:?})");

        // null is the default sequence, every raw frame in order
        let frames: Vec<i32> = if sequence.is_null() {
            let raw_frame_count: i32 = jvm.get_field(&this, CLASS, "rawFrameCount", "I").await?;
            (0..raw_frame_count).collect()
        } else {
            let length = jvm.array_length(&sequence).await?;
            jvm.load_array(&sequence, 0, length).await?
        };

        let mut copy = jvm.instantiate_array("I", frames.len()).await?;
        jvm.store_array(&mut copy, 0, frames).await?;
        jvm.put_field(&mut this, CLASS, "frameSequence", "[I", copy).await?;
        jvm.put_field(&mut this, CLASS, "sequenceIndex", "I", 0).await
    }

    async fn set_transform(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, transform: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::setTransform({this:?}, {transform})");

        if !(0..8).contains(&transform) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid transform").await);
        }

        // The transform turns the frame around the reference pixel, which stays where it is on the screen.
        let ref_x: i32 = jvm.invoke_virtual(&this, CLASS, "getRefPixelX", "()I", ()).await?;
        let ref_y: i32 = jvm.invoke_virtual(&this, CLASS, "getRefPixelY", "()I", ()).await?;
        jvm.put_field(&mut this, CLASS, "transform", "I", transform).await?;

        let frame_width: i32 = jvm.get_field(&this, CLASS, "frameWidth", "I").await?;
        let frame_height: i32 = jvm.get_field(&this, CLASS, "frameHeight", "I").await?;
        let (width, height) = if transform >= 4 {
            (frame_height, frame_width)
        } else {
            (frame_width, frame_height)
        };
        let _: () = jvm.invoke_virtual(&this, LAYER, "setSize", "(II)V", (width, height)).await?;
        jvm.invoke_virtual(&this, CLASS, "setRefPixelPosition", "(II)V", (ref_x, ref_y)).await
    }

    async fn define_collision_rectangle(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::defineCollisionRectangle({this:?}, {x}, {y}, {width}, {height})");

        if width < 0 || height < 0 {
            return Err(jvm
                .exception("java/lang/IllegalArgumentException", "negative collision rectangle size")
                .await);
        }
        jvm.put_field(&mut this, CLASS, "collisionX", "I", x).await?;
        jvm.put_field(&mut this, CLASS, "collisionY", "I", y).await?;
        jvm.put_field(&mut this, CLASS, "collisionWidth", "I", width).await?;
        jvm.put_field(&mut this, CLASS, "collisionHeight", "I", height).await
    }

    /// The collision rectangle on the screen.
    async fn collision_bounds(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<Rect> {
        let rect = Rect {
            x: jvm.get_field(this, CLASS, "collisionX", "I").await?,
            y: jvm.get_field(this, CLASS, "collisionY", "I").await?,
            width: jvm.get_field(this, CLASS, "collisionWidth", "I").await?,
            height: jvm.get_field(this, CLASS, "collisionHeight", "I").await?,
        };
        let transform: i32 = jvm.get_field(this, CLASS, "transform", "I").await?;
        let frame_width: i32 = jvm.get_field(this, CLASS, "frameWidth", "I").await?;
        let frame_height: i32 = jvm.get_field(this, CLASS, "frameHeight", "I").await?;
        let x: i32 = jvm.invoke_virtual(this, LAYER, "getX", "()I", ()).await?;
        let y: i32 = jvm.invoke_virtual(this, LAYER, "getY", "()I", ()).await?;

        let rect = transform_rect(transform, rect, frame_width, frame_height);
        Ok(Rect {
            x: x + rect.x,
            y: y + rect.y,
            ..rect
        })
    }

    /// The current frame as drawn on the screen.
    async fn frame_pixels(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<Pixels> {
        let image: ClassInstanceRef<Image> = jvm.get_field(this, CLASS, "image", "Ljavax/microedition/lcdui/Image;").await?;
        let (source_x, source_y, frame_width, frame_height) = Self::frame_region(jvm, this).await?;
        let transform: i32 = jvm.get_field(this, CLASS, "transform", "I").await?;
        let x: i32 = jvm.invoke_virtual(this, LAYER, "getX", "()I", ()).await?;
        let y: i32 = jvm.invoke_virtual(this, LAYER, "getY", "()I", ()).await?;

        let source = Image::image(jvm, &image).await?;
        let (width, height, argb) = Image::transform_region(&*source, source_x, source_y, frame_width, frame_height, transform).unwrap_or_default();
        Ok(Pixels { x, y, width, height, argb })
    }

    /// The source region of the current frame in the image.
    async fn frame_region(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<(i32, i32, i32, i32)> {
        let image: ClassInstanceRef<Image> = jvm.get_field(this, CLASS, "image", "Ljavax/microedition/lcdui/Image;").await?;
        let frame_width: i32 = jvm.get_field(this, CLASS, "frameWidth", "I").await?;
        let frame_height: i32 = jvm.get_field(this, CLASS, "frameHeight", "I").await?;
        let sequence: ClassInstanceRef<Array<i32>> = jvm.get_field(this, CLASS, "frameSequence", "[I").await?;
        let index: i32 = jvm.get_field(this, CLASS, "sequenceIndex", "I").await?;
        let frame: i32 = jvm.load_array(&sequence, index as usize, 1).await?[0];
        let image_width: i32 = jvm.invoke_virtual(&image, IMAGE, "getWidth", "()I", ()).await?;
        let frames_per_row = image_width / frame_width;

        Ok((
            frame % frames_per_row * frame_width,
            frame / frames_per_row * frame_height,
            frame_width,
            frame_height,
        ))
    }

    async fn collides_with_sprite(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        other: ClassInstanceRef<Sprite>,
        pixel_level: bool,
    ) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::collidesWith({this:?}, {other:?}, {pixel_level})");

        let visible: bool = jvm.invoke_virtual(&this, LAYER, "isVisible", "()Z", ()).await?;
        let other_visible: bool = jvm.invoke_virtual(&other, LAYER, "isVisible", "()Z", ()).await?;
        if !visible || !other_visible {
            return Ok(false);
        }
        let bounds = Self::collision_bounds(jvm, &this).await?;
        let other_bounds = Self::collision_bounds(jvm, &other).await?;
        let Some(area) = bounds.intersect(&other_bounds) else {
            return Ok(false);
        };
        if !pixel_level {
            return Ok(true);
        }

        let pixels = Self::frame_pixels(jvm, &this).await?;
        let other_pixels = Self::frame_pixels(jvm, &other).await?;
        Ok(any_opaque_in_both(&area, &pixels, &other_pixels, (0, 0)))
    }

    async fn collides_with_tiled_layer(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        layer: ClassInstanceRef<TiledLayer>,
        pixel_level: bool,
    ) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::collidesWith({this:?}, {layer:?}, {pixel_level})");

        let visible: bool = jvm.invoke_virtual(&this, LAYER, "isVisible", "()Z", ()).await?;
        let layer_visible: bool = jvm.invoke_virtual(&layer, LAYER, "isVisible", "()Z", ()).await?;
        if !visible || !layer_visible {
            return Ok(false);
        }
        let layer_bounds = Rect {
            x: jvm.invoke_virtual(&layer, LAYER, "getX", "()I", ()).await?,
            y: jvm.invoke_virtual(&layer, LAYER, "getY", "()I", ()).await?,
            width: jvm.invoke_virtual(&layer, LAYER, "getWidth", "()I", ()).await?,
            height: jvm.invoke_virtual(&layer, LAYER, "getHeight", "()I", ()).await?,
        };
        let Some(area) = Self::collision_bounds(jvm, &this).await?.intersect(&layer_bounds) else {
            return Ok(false);
        };

        let cell_width: i32 = jvm.invoke_virtual(&layer, TILED_LAYER, "getCellWidth", "()I", ()).await?;
        let cell_height: i32 = jvm.invoke_virtual(&layer, TILED_LAYER, "getCellHeight", "()I", ()).await?;
        let (pixels, tiles) = if pixel_level {
            let tiles: ClassInstanceRef<Image> = jvm.get_field(&layer, TILED_LAYER, "image", "Ljavax/microedition/lcdui/Image;").await?;
            (
                Some(Self::frame_pixels(jvm, &this).await?),
                Some(Pixels::of_image(jvm, &tiles, 0, 0).await?),
            )
        } else {
            (None, None)
        };

        let first_column = (area.x - layer_bounds.x) / cell_width;
        let last_column = (area.x + area.width - 1 - layer_bounds.x) / cell_width;
        let first_row = (area.y - layer_bounds.y) / cell_height;
        let last_row = (area.y + area.height - 1 - layer_bounds.y) / cell_height;
        for row in first_row..=last_row {
            for column in first_column..=last_column {
                let mut tile: i32 = jvm.invoke_virtual(&layer, TILED_LAYER, "getCell", "(II)I", (column, row)).await?;
                if tile < 0 {
                    tile = jvm.invoke_virtual(&layer, TILED_LAYER, "getAnimatedTile", "(I)I", (tile,)).await?;
                }
                if tile == 0 {
                    continue;
                }
                let (Some(pixels), Some(tiles)) = (&pixels, &tiles) else {
                    return Ok(true);
                };

                // the tile image placed so that this tile covers its cell
                let cell = Rect {
                    x: layer_bounds.x + column * cell_width,
                    y: layer_bounds.y + row * cell_height,
                    width: cell_width,
                    height: cell_height,
                };
                let tiles_per_row = tiles.width / cell_width;
                let tile_offset = (
                    cell.x - (tile - 1) % tiles_per_row * cell_width,
                    cell.y - (tile - 1) / tiles_per_row * cell_height,
                );
                if let Some(cell_area) = cell.intersect(&area)
                    && any_opaque_in_both(&cell_area, pixels, tiles, tile_offset)
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn collides_with_image(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        image: ClassInstanceRef<Image>,
        x: i32,
        y: i32,
        pixel_level: bool,
    ) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::collidesWith({this:?}, {image:?}, {x}, {y}, {pixel_level})");

        let visible: bool = jvm.invoke_virtual(&this, LAYER, "isVisible", "()Z", ()).await?;
        if !visible {
            return Ok(false);
        }
        let image_bounds = Rect {
            x,
            y,
            width: jvm.invoke_virtual(&image, IMAGE, "getWidth", "()I", ()).await?,
            height: jvm.invoke_virtual(&image, IMAGE, "getHeight", "()I", ()).await?,
        };
        let Some(area) = Self::collision_bounds(jvm, &this).await?.intersect(&image_bounds) else {
            return Ok(false);
        };
        if !pixel_level {
            return Ok(true);
        }

        let pixels = Self::frame_pixels(jvm, &this).await?;
        let image_pixels = Pixels::of_image(jvm, &image, x, y).await?;
        Ok(any_opaque_in_both(&area, &pixels, &image_pixels, (0, 0)))
    }

    async fn paint(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, graphics: ClassInstanceRef<Graphics>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.game.Sprite::paint({this:?}, {graphics:?})");

        let visible: bool = jvm.invoke_virtual(&this, LAYER, "isVisible", "()Z", ()).await?;
        if !visible {
            return Ok(());
        }
        let image: ClassInstanceRef<Image> = jvm.get_field(&this, CLASS, "image", "Ljavax/microedition/lcdui/Image;").await?;
        let (source_x, source_y, frame_width, frame_height) = Self::frame_region(jvm, &this).await?;
        let transform: i32 = jvm.get_field(&this, CLASS, "transform", "I").await?;
        let x: i32 = jvm.invoke_virtual(&this, LAYER, "getX", "()I", ()).await?;
        let y: i32 = jvm.invoke_virtual(&this, LAYER, "getY", "()I", ()).await?;

        jvm.invoke_virtual(
            &graphics,
            "javax/microedition/lcdui/Graphics",
            "drawRegion",
            "(Ljavax/microedition/lcdui/Image;IIIIIIII)V",
            [
                image.into(),
                source_x.into(),
                source_y.into(),
                frame_width.into(),
                frame_height.into(),
                transform.into(),
                x.into(),
                y.into(),
                TOP_LEFT.into(),
            ] as [JavaValue; 9],
        )
        .await
    }
}

#[cfg(test)]
mod test {
    use alloc::boxed::Box;

    use jvm::{ClassInstance, ClassInstanceRef, JavaValue, Jvm};
    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::{classes::javax::microedition::lcdui::Image, get_protos};

    const RED: i32 = 0xffff0000u32 as i32;
    const BLUE: i32 = 0xff0000ffu32 as i32;
    const WHITE: i32 = -1;
    const CLEAR: i32 = 0;
    const SPRITE: &str = "javax/microedition/lcdui/game/Sprite";
    const LAYER: &str = "javax/microedition/lcdui/game/Layer";

    async fn new_sprite(jvm: &Jvm, frames: &ClassInstanceRef<Image>) -> jvm::Result<Box<dyn ClassInstance>> {
        jvm.new_class(SPRITE, "(Ljavax/microedition/lcdui/Image;II)V", (frames.clone(), 2, 2))
            .await
    }

    #[test]
    fn sprite_frames_transform_and_collisions() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            // two 2x2 frames side by side: 0 has a red left column and a clear right one, 1 is blue
            let mut rgb = jvm.instantiate_array("I", 8).await?;
            jvm.store_array(&mut rgb, 0, [RED, CLEAR, BLUE, BLUE, RED, CLEAR, BLUE, BLUE]).await?;
            let frames: ClassInstanceRef<Image> = jvm
                .invoke_static(
                    "javax/microedition/lcdui/Image",
                    "createRGBImage",
                    "([IIIZ)Ljavax/microedition/lcdui/Image;",
                    (rgb, 4, 2, true),
                )
                .await?;

            let sprite = new_sprite(&jvm, &frames).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&sprite, SPRITE, "getRawFrameCount", "()I", ()).await?, 2);
            let _: () = jvm.invoke_virtual(&sprite, SPRITE, "setRefPixelPosition", "(II)V", (5, 0)).await?;

            let screen: ClassInstanceRef<Image> = jvm
                .invoke_static(
                    "javax/microedition/lcdui/Image",
                    "createImage",
                    "(II)Ljavax/microedition/lcdui/Image;",
                    (8, 2),
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
            let paint = async |expected: [i32; 4]| -> jvm::Result<()> {
                let graphics_class = "javax/microedition/lcdui/Graphics";
                let _: () = jvm.invoke_virtual(&graphics, graphics_class, "setColor", "(I)V", (WHITE,)).await?;
                let _: () = jvm.invoke_virtual(&graphics, graphics_class, "fillRect", "(IIII)V", (0, 0, 8, 2)).await?;
                let _: () = jvm
                    .invoke_virtual(
                        &sprite,
                        SPRITE,
                        "paint",
                        "(Ljavax/microedition/lcdui/Graphics;)V",
                        [graphics.clone().into()] as [JavaValue; 1],
                    )
                    .await?;
                let row = jvm.instantiate_array("I", 4).await?;
                let _: () = jvm
                    .invoke_virtual(
                        &screen,
                        "javax/microedition/lcdui/Image",
                        "getRGB",
                        "([IIIIIII)V",
                        (row.clone(), 0, 4, 3, 0, 4, 1),
                    )
                    .await?;
                assert_eq!(jvm.load_array::<i32>(&row, 0, 4).await?, expected);
                Ok(())
            };
            // screen x 3..7: frame 0 with the reference pixel (its top left) at x 5
            paint([WHITE, WHITE, RED, WHITE]).await?;

            // mirrored around the reference pixel: the red column stays at x 5, the frame now starts at x 4
            let _: () = jvm.invoke_virtual(&sprite, SPRITE, "setTransform", "(I)V", (2,)).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&sprite, LAYER, "getX", "()I", ()).await?, 4);
            assert_eq!(jvm.invoke_virtual::<_, i32>(&sprite, SPRITE, "getRefPixelX", "()I", ()).await?, 5);
            paint([WHITE, WHITE, RED, WHITE]).await?;

            // another sprite at x 3..4, opaque only at x 3: the frames overlap at x 4, where the first one is clear
            let other = new_sprite(&jvm, &frames).await?;
            let _: () = jvm.invoke_virtual(&other, LAYER, "setPosition", "(II)V", (3, 0)).await?;
            let collides = async |pixel_level: bool| -> jvm::Result<bool> {
                jvm.invoke_virtual(
                    &sprite,
                    SPRITE,
                    "collidesWith",
                    "(Ljavax/microedition/lcdui/game/Sprite;Z)Z",
                    [other.clone().into(), pixel_level.into()] as [JavaValue; 2],
                )
                .await
            };
            assert!(collides(false).await?);
            assert!(!collides(true).await?);

            // the blue frame is opaque at x 4 too, where the other sprite is clear
            let _: () = jvm.invoke_virtual(&sprite, SPRITE, "nextFrame", "()V", ()).await?;
            assert_eq!(jvm.invoke_virtual::<_, i32>(&sprite, SPRITE, "getFrame", "()I", ()).await?, 1);
            paint([WHITE, BLUE, BLUE, WHITE]).await?;
            assert!(!collides(true).await?);
            // the other sprite's red column moved onto the blue frame
            let _: () = jvm.invoke_virtual(&other, LAYER, "setPosition", "(II)V", (4, 0)).await?;
            assert!(collides(true).await?);

            // a tiled layer at x 4..5 showing frame 0 as its tile: opaque at x 4
            let layer = jvm
                .new_class(
                    "javax/microedition/lcdui/game/TiledLayer",
                    "(IILjavax/microedition/lcdui/Image;II)V",
                    (1, 1, frames.clone(), 2, 2),
                )
                .await?;
            let _: () = jvm
                .invoke_virtual(&layer, "javax/microedition/lcdui/game/TiledLayer", "setCell", "(III)V", (0, 0, 1))
                .await?;
            let _: () = jvm.invoke_virtual(&layer, LAYER, "setPosition", "(II)V", (4, 0)).await?;
            let collides_with_layer: bool = jvm
                .invoke_virtual(
                    &sprite,
                    SPRITE,
                    "collidesWith",
                    "(Ljavax/microedition/lcdui/game/TiledLayer;Z)Z",
                    [layer.into(), true.into()] as [JavaValue; 2],
                )
                .await?;
            assert!(collides_with_layer);

            Ok(())
        })
    }
}

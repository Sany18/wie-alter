use alloc::vec;

use jvm::{Array, ClassInstanceRef, JavaValue, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_midp::classes::javax::microedition::lcdui::{Graphics, Image};

const FLIP_HORIZONTAL: i32 = 0x2000;
const FLIP_VERTICAL: i32 = 0x4000;
const TYPE_INT_888_RGB: i32 = 888;
const TYPE_INT_8888_ARGB: i32 = 8888;

const CONSTANTS: [(&str, i32); 18] = [
    ("FLIP_HORIZONTAL", FLIP_HORIZONTAL),
    ("FLIP_VERTICAL", FLIP_VERTICAL),
    ("ROTATE_90", 90),
    ("ROTATE_180", 180),
    ("ROTATE_270", 270),
    ("TYPE_BYTE_1_GRAY", 1),
    ("TYPE_BYTE_1_GRAY_VERTICAL", -1),
    ("TYPE_BYTE_2_GRAY", 2),
    ("TYPE_BYTE_4_GRAY", 4),
    ("TYPE_BYTE_8_GRAY", 8),
    ("TYPE_BYTE_332_RGB", 332),
    ("TYPE_USHORT_4444_ARGB", 4444),
    ("TYPE_USHORT_444_RGB", 444),
    ("TYPE_USHORT_555_RGB", 555),
    ("TYPE_USHORT_1555_ARGB", 1555),
    ("TYPE_USHORT_565_RGB", 565),
    ("TYPE_INT_888_RGB", TYPE_INT_888_RGB),
    ("TYPE_INT_8888_ARGB", TYPE_INT_8888_ARGB),
];

// 2x2 screen-space matrices (y grows downwards) of the MIDP Sprite transforms, indexed by TRANS_* value.
const MIDP_TRANSFORMS: [[i32; 4]; 8] = [
    [1, 0, 0, 1],   // TRANS_NONE
    [1, 0, 0, -1],  // TRANS_MIRROR_ROT180
    [-1, 0, 0, 1],  // TRANS_MIRROR
    [-1, 0, 0, -1], // TRANS_ROT180
    [0, 1, 1, 0],   // TRANS_MIRROR_ROT270
    [0, -1, 1, 0],  // TRANS_ROT90
    [0, 1, -1, 0],  // TRANS_ROT270
    [0, -1, -1, 0], // TRANS_MIRROR_ROT90
];

const fn multiply(a: [i32; 4], b: [i32; 4]) -> [i32; 4] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
    ]
}

// Nokia manipulation: flips first, then a counter-clockwise rotation. Returns the equivalent MIDP transform.
fn manipulation_to_transform(manipulation: i32) -> Option<i32> {
    let mut matrix = [1, 0, 0, 1];
    if manipulation & FLIP_HORIZONTAL != 0 {
        matrix = multiply([-1, 0, 0, 1], matrix);
    }
    if manipulation & FLIP_VERTICAL != 0 {
        matrix = multiply([1, 0, 0, -1], matrix);
    }
    let rotation = match manipulation & !(FLIP_HORIZONTAL | FLIP_VERTICAL) {
        0 => [1, 0, 0, 1],
        90 => [0, 1, -1, 0],
        180 => [-1, 0, 0, -1],
        270 => [0, -1, 1, 0],
        _ => return None,
    };
    matrix = multiply(rotation, matrix);

    MIDP_TRANSFORMS.iter().position(|x| *x == matrix).map(|x| x as i32)
}

// interface com.nokia.mid.ui.DirectGraphics
pub struct DirectGraphics;

impl DirectGraphics {
    pub fn as_proto() -> WieJavaClassProto {
        let abstract_method =
            |name, descriptor| JavaMethodProto::new_abstract(name, descriptor, MethodAccessFlags::PUBLIC | MethodAccessFlags::ABSTRACT);

        WieJavaClassProto {
            name: "com/nokia/mid/ui/DirectGraphics",
            parent_class: None,
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::cl_init, MethodAccessFlags::STATIC),
                abstract_method("drawImage", "(Ljavax/microedition/lcdui/Image;IIII)V"),
                abstract_method("setARGBColor", "(I)V"),
                abstract_method("getAlphaComponent", "()I"),
                abstract_method("getNativePixelFormat", "()I"),
                abstract_method("drawTriangle", "(IIIIIII)V"),
                abstract_method("fillTriangle", "(IIIIIII)V"),
                abstract_method("drawPolygon", "([II[IIII)V"),
                abstract_method("fillPolygon", "([II[IIII)V"),
                abstract_method("drawPixels", "([IZIIIIIIII)V"),
                abstract_method("drawPixels", "([B[BIIIIIIII)V"),
                abstract_method("drawPixels", "([SZIIIIIIII)V"),
                abstract_method("getPixels", "([IIIIIIII)V"),
                abstract_method("getPixels", "([B[BIIIIIII)V"),
                abstract_method("getPixels", "([SIIIIIII)V"),
            ],
            fields: CONSTANTS
                .iter()
                .map(|(name, _)| JavaFieldProto::new(name, "I", FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL))
                .collect(),
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::INTERFACE | ClassAccessFlags::ABSTRACT,
        }
    }

    async fn cl_init(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::debug!("com.nokia.mid.ui.DirectGraphics::<clinit>()");

        for (name, value) in CONSTANTS {
            jvm.put_static_field("com/nokia/mid/ui/DirectGraphics", name, "I", value).await?;
        }
        Ok(())
    }
}

// class net.wie.DirectGraphicsImpl: DirectGraphics view over a MIDP Graphics.
pub struct DirectGraphicsImpl;

#[allow(clippy::too_many_arguments)]
impl DirectGraphicsImpl {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "net/wie/DirectGraphicsImpl",
            parent_class: Some("java/lang/Object"),
            interfaces: vec!["com/nokia/mid/ui/DirectGraphics"],
            methods: vec![
                JavaMethodProto::new("<init>", "(Ljavax/microedition/lcdui/Graphics;)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "drawImage",
                    "(Ljavax/microedition/lcdui/Image;IIII)V",
                    Self::draw_image,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("setARGBColor", "(I)V", Self::set_argb_color, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getAlphaComponent", "()I", Self::get_alpha_component, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getNativePixelFormat", "()I", Self::get_native_pixel_format, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("drawTriangle", "(IIIIIII)V", Self::draw_triangle, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("fillTriangle", "(IIIIIII)V", Self::fill_triangle, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("drawPolygon", "([II[IIII)V", Self::draw_polygon, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("fillPolygon", "([II[IIII)V", Self::fill_polygon, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("drawPixels", "([IZIIIIIIII)V", Self::draw_pixels, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![
                JavaFieldProto::new("graphics", "Ljavax/microedition/lcdui/Graphics;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("alpha", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, graphics: ClassInstanceRef<Graphics>) -> JvmResult<()> {
        tracing::debug!("net.wie.DirectGraphicsImpl::<init>({this:?}, {graphics:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;
        jvm.put_field(
            &mut this,
            "net/wie/DirectGraphicsImpl",
            "graphics",
            "Ljavax/microedition/lcdui/Graphics;",
            graphics,
        )
        .await?;
        jvm.put_field(&mut this, "net/wie/DirectGraphicsImpl", "alpha", "I", 0xff).await
    }

    async fn graphics(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<Graphics>> {
        jvm.get_field(this, "net/wie/DirectGraphicsImpl", "graphics", "Ljavax/microedition/lcdui/Graphics;")
            .await
    }

    async fn draw_image(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        img: ClassInstanceRef<Image>,
        x: i32,
        y: i32,
        anchor: i32,
        manipulation: i32,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.DirectGraphicsImpl::drawImage({this:?}, {img:?}, {x}, {y}, {anchor}, {manipulation:#x})");

        if img.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "img is null").await);
        }
        let Some(transform) = manipulation_to_transform(manipulation) else {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid manipulation").await);
        };

        let graphics = Self::graphics(jvm, &this).await?;
        let width: i32 = jvm.invoke_virtual(&img, "javax/microedition/lcdui/Image", "getWidth", "()I", ()).await?;
        let height: i32 = jvm.invoke_virtual(&img, "javax/microedition/lcdui/Image", "getHeight", "()I", ()).await?;
        jvm.invoke_virtual(
            &graphics,
            "javax/microedition/lcdui/Graphics",
            "drawRegion",
            "(Ljavax/microedition/lcdui/Image;IIIIIIII)V",
            [
                img.into(),
                JavaValue::Int(0),
                JavaValue::Int(0),
                JavaValue::Int(width),
                JavaValue::Int(height),
                JavaValue::Int(transform),
                JavaValue::Int(x),
                JavaValue::Int(y),
                JavaValue::Int(anchor),
            ],
        )
        .await
    }

    async fn set_argb_color(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, argb: i32) -> JvmResult<()> {
        tracing::debug!("net.wie.DirectGraphicsImpl::setARGBColor({this:?}, {argb:#x})");

        let graphics = Self::graphics(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "setColor", "(I)V", (argb & 0xffffff,))
            .await?;
        jvm.put_field(&mut this, "net/wie/DirectGraphicsImpl", "alpha", "I", (argb >> 24) & 0xff)
            .await
    }

    async fn get_alpha_component(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("net.wie.DirectGraphicsImpl::getAlphaComponent({this:?})");

        jvm.get_field(&this, "net/wie/DirectGraphicsImpl", "alpha", "I").await
    }

    async fn get_native_pixel_format(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("net.wie.DirectGraphicsImpl::getNativePixelFormat({this:?})");

        Ok(TYPE_INT_8888_ARGB)
    }

    // Draws with a temporary color, keeping the Graphics color the game set.
    async fn with_color<F>(jvm: &Jvm, this: &ClassInstanceRef<Self>, argb: i32, draw: F) -> JvmResult<()>
    where
        F: AsyncFnOnce(&ClassInstanceRef<Graphics>) -> JvmResult<()>,
    {
        let graphics = Self::graphics(jvm, this).await?;
        let color: i32 = jvm
            .invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "getColor", "()I", ())
            .await?;
        let _: () = jvm
            .invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "setColor", "(I)V", (argb & 0xffffff,))
            .await?;
        draw(&graphics).await?;
        jvm.invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "setColor", "(I)V", (color,))
            .await
    }

    async fn draw_lines(jvm: &Jvm, graphics: &ClassInstanceRef<Graphics>, points: &[(i32, i32)]) -> JvmResult<()> {
        for (index, (x1, y1)) in points.iter().enumerate() {
            let (x2, y2) = points[(index + 1) % points.len()];
            let _: () = jvm
                .invoke_virtual(graphics, "javax/microedition/lcdui/Graphics", "drawLine", "(IIII)V", (*x1, *y1, x2, y2))
                .await?;
        }
        Ok(())
    }

    async fn draw_triangle(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        argb: i32,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.DirectGraphicsImpl::drawTriangle({this:?}, {x1}, {y1}, {x2}, {y2}, {x3}, {y3}, {argb:#x})");

        Self::with_color(jvm, &this, argb, async |graphics| {
            Self::draw_lines(jvm, graphics, &[(x1, y1), (x2, y2), (x3, y3)]).await
        })
        .await
    }

    async fn fill_triangle(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        argb: i32,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.DirectGraphicsImpl::fillTriangle({this:?}, {x1}, {y1}, {x2}, {y2}, {x3}, {y3}, {argb:#x})");

        Self::with_color(jvm, &this, argb, async |graphics| {
            jvm.invoke_virtual(
                graphics,
                "javax/microedition/lcdui/Graphics",
                "fillTriangle",
                "(IIIIII)V",
                (x1, y1, x2, y2, x3, y3),
            )
            .await
        })
        .await
    }

    async fn polygon_points(
        jvm: &Jvm,
        x_points: &ClassInstanceRef<Array<i32>>,
        x_offset: i32,
        y_points: &ClassInstanceRef<Array<i32>>,
        y_offset: i32,
        count: i32,
    ) -> JvmResult<vec::Vec<(i32, i32)>> {
        let xs: vec::Vec<i32> = jvm.load_array(x_points, x_offset as _, count as _).await?;
        let ys: vec::Vec<i32> = jvm.load_array(y_points, y_offset as _, count as _).await?;
        Ok(xs.into_iter().zip(ys).collect())
    }

    async fn draw_polygon(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        x_points: ClassInstanceRef<Array<i32>>,
        x_offset: i32,
        y_points: ClassInstanceRef<Array<i32>>,
        y_offset: i32,
        count: i32,
        argb: i32,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.DirectGraphicsImpl::drawPolygon({this:?}, {x_points:?}, {x_offset}, {y_points:?}, {y_offset}, {count}, {argb:#x})");

        let points = Self::polygon_points(jvm, &x_points, x_offset, &y_points, y_offset, count).await?;
        Self::with_color(jvm, &this, argb, async |graphics| Self::draw_lines(jvm, graphics, &points).await).await
    }

    // Fan triangulation; exact for convex polygons, which is what games draw in practice.
    async fn fill_polygon(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        x_points: ClassInstanceRef<Array<i32>>,
        x_offset: i32,
        y_points: ClassInstanceRef<Array<i32>>,
        y_offset: i32,
        count: i32,
        argb: i32,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.DirectGraphicsImpl::fillPolygon({this:?}, {x_points:?}, {x_offset}, {y_points:?}, {y_offset}, {count}, {argb:#x})");

        let points = Self::polygon_points(jvm, &x_points, x_offset, &y_points, y_offset, count).await?;
        Self::with_color(jvm, &this, argb, async |graphics| {
            for window in points.windows(2).skip(1) {
                let ((x1, y1), (x2, y2), (x3, y3)) = (points[0], window[0], window[1]);
                let _: () = jvm
                    .invoke_virtual(
                        graphics,
                        "javax/microedition/lcdui/Graphics",
                        "fillTriangle",
                        "(IIIIII)V",
                        (x1, y1, x2, y2, x3, y3),
                    )
                    .await?;
            }
            Ok(())
        })
        .await
    }

    async fn draw_pixels(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        pixels: ClassInstanceRef<Array<i32>>,
        transparency: bool,
        offset: i32,
        scan_length: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        manipulation: i32,
        format: i32,
    ) -> JvmResult<()> {
        tracing::debug!(
            "net.wie.DirectGraphicsImpl::drawPixels({this:?}, {pixels:?}, {transparency}, {offset}, {scan_length}, {x}, {y}, {width}, {height}, {manipulation:#x}, {format})"
        );

        if manipulation != 0 || (format != TYPE_INT_8888_ARGB && format != TYPE_INT_888_RGB) {
            tracing::warn!("stub net.wie.DirectGraphicsImpl::drawPixels with manipulation {manipulation:#x}, format {format}");
            return Ok(());
        }

        let graphics = Self::graphics(jvm, &this).await?;
        jvm.invoke_virtual(
            &graphics,
            "javax/microedition/lcdui/Graphics",
            "drawRGB",
            "([IIIIIIIZ)V",
            (
                pixels,
                offset,
                scan_length,
                x,
                y,
                width,
                height,
                transparency && format == TYPE_INT_8888_ARGB,
            ),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::manipulation_to_transform;

    #[test]
    fn manipulation_maps_to_midp_transform() {
        assert_eq!(manipulation_to_transform(0), Some(0)); // TRANS_NONE
        assert_eq!(manipulation_to_transform(0x2000), Some(2)); // FLIP_HORIZONTAL -> TRANS_MIRROR
        assert_eq!(manipulation_to_transform(0x4000), Some(1)); // FLIP_VERTICAL -> TRANS_MIRROR_ROT180
        assert_eq!(manipulation_to_transform(0x6000), Some(3)); // both flips -> TRANS_ROT180
        assert_eq!(manipulation_to_transform(90), Some(6)); // counter-clockwise 90 -> TRANS_ROT270
        assert_eq!(manipulation_to_transform(270), Some(5)); // counter-clockwise 270 -> TRANS_ROT90
        assert_eq!(manipulation_to_transform(45), None);
    }
}

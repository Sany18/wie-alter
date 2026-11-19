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
const TYPE_USHORT_4444_ARGB: i32 = 4444;

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
    ("TYPE_USHORT_4444_ARGB", TYPE_USHORT_4444_ARGB),
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
                JavaMethodProto::new("drawPixels", "([B[BIIIIIIII)V", Self::draw_pixels_byte, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("drawPixels", "([SZIIIIIIII)V", Self::draw_pixels_short, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPixels", "([IIIIIIII)V", Self::get_pixels, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPixels", "([B[BIIIIIII)V", Self::get_pixels_byte, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getPixels", "([SIIIIIII)V", Self::get_pixels_short, MethodAccessFlags::PUBLIC),
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

        // The format of the Nokia phones the applications were made for: they pass it with short pixel arrays.
        Ok(TYPE_USHORT_4444_ARGB)
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

        if format != TYPE_INT_8888_ARGB && format != TYPE_INT_888_RGB {
            tracing::warn!("stub net.wie.DirectGraphicsImpl::drawPixels with format {format}");
            return Ok(());
        }
        if pixels.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "pixels is null").await);
        }
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let rows = Self::load_rows::<i32>(jvm, &pixels, offset, scan_length, width, height).await?;
        let argb = rows
            .into_iter()
            .map(|pixel| {
                if format == TYPE_INT_888_RGB {
                    pixel | 0xff00_0000u32 as i32
                } else {
                    pixel
                }
            })
            .collect();
        Self::draw_argb(
            jvm,
            &this,
            argb,
            x,
            y,
            width,
            height,
            manipulation,
            transparency && format == TYPE_INT_8888_ARGB,
        )
        .await
    }

    async fn draw_pixels_short(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        pixels: ClassInstanceRef<Array<i16>>,
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

        if pixels.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "pixels is null").await);
        }
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        if !matches!(format, 4444 | 444 | 555 | 1555 | 565) {
            tracing::warn!("stub net.wie.DirectGraphicsImpl::drawPixels(short[]) with format {format}");
            return Ok(());
        }
        let rows = Self::load_rows::<i16>(jvm, &pixels, offset, scan_length, width, height).await?;
        let argb = rows.into_iter().map(|pixel| short_to_argb(pixel as u16, format)).collect();
        let alpha = transparency && matches!(format, 4444 | 1555);
        Self::draw_argb(jvm, &this, argb, x, y, width, height, manipulation, alpha).await
    }

    async fn draw_pixels_byte(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        pixels: ClassInstanceRef<Array<i8>>,
        mask: ClassInstanceRef<Array<i8>>,
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
            "net.wie.DirectGraphicsImpl::drawPixels({this:?}, {pixels:?}, {mask:?}, {offset}, {scan_length}, {x}, {y}, {width}, {height}, {manipulation:#x}, {format})"
        );

        if pixels.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "pixels is null").await);
        }
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let Some(bits) = byte_format_bits(format) else {
            tracing::warn!("stub net.wie.DirectGraphicsImpl::drawPixels(byte[]) with format {format}");
            return Ok(());
        };
        // Packed formats: scan_length is in pixels, rows start at a bit offset (offset counts pixels too).
        let all: vec::Vec<i8> = jvm.load_array(&pixels, 0, jvm.array_length(&pixels).await?).await?;
        let mask: Option<vec::Vec<i8>> = if mask.is_null() {
            None
        } else {
            Some(jvm.load_array(&mask, 0, jvm.array_length(&mask).await?).await?)
        };
        let mut argb = vec::Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            for column in 0..width {
                let index = i64::from(offset) + i64::from(row) * i64::from(scan_length) + i64::from(column);
                if index < 0 {
                    return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "negative pixel index").await);
                }
                let Some(value) = packed(&all, index as usize, bits) else {
                    return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "pixels is too small").await);
                };
                let opaque = match &mask {
                    // The mask is one bit per pixel, laid out like the pixels; a set bit is an opaque pixel.
                    Some(mask) => packed(mask, index as usize, 1).unwrap_or(1) != 0,
                    None => true,
                };
                let rgb = byte_to_rgb(value, format);
                argb.push(if opaque { rgb | 0xff00_0000u32 as i32 } else { rgb & 0xffffff });
            }
        }
        Self::draw_argb(jvm, &this, argb, x, y, width, height, manipulation, mask.is_some()).await
    }

    async fn get_pixels(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        mut pixels: ClassInstanceRef<Array<i32>>,
        offset: i32,
        scan_length: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: i32,
    ) -> JvmResult<()> {
        tracing::debug!(
            "net.wie.DirectGraphicsImpl::getPixels({this:?}, {pixels:?}, {offset}, {scan_length}, {x}, {y}, {width}, {height}, {format})"
        );

        if pixels.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "pixels is null").await);
        }
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let argb = Self::read_argb(jvm, &this, x, y, width, height).await?;
        let values: vec::Vec<i32> = argb
            .into_iter()
            .map(|pixel| if format == TYPE_INT_888_RGB { pixel & 0xffffff } else { pixel })
            .collect();
        Self::store_rows(jvm, &mut pixels, offset, scan_length, width, height, values).await
    }

    async fn get_pixels_short(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        mut pixels: ClassInstanceRef<Array<i16>>,
        offset: i32,
        scan_length: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: i32,
    ) -> JvmResult<()> {
        tracing::debug!(
            "net.wie.DirectGraphicsImpl::getPixels({this:?}, {pixels:?}, {offset}, {scan_length}, {x}, {y}, {width}, {height}, {format})"
        );

        if pixels.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "pixels is null").await);
        }
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        if !matches!(format, 4444 | 444 | 555 | 1555 | 565) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "unsupported format").await);
        }
        let argb = Self::read_argb(jvm, &this, x, y, width, height).await?;
        let values = argb.into_iter().map(|pixel| argb_to_short(pixel, format) as i16).collect();
        Self::store_rows(jvm, &mut pixels, offset, scan_length, width, height, values).await
    }

    async fn get_pixels_byte(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        mut pixels: ClassInstanceRef<Array<i8>>,
        mut mask: ClassInstanceRef<Array<i8>>,
        offset: i32,
        scan_length: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: i32,
    ) -> JvmResult<()> {
        tracing::debug!(
            "net.wie.DirectGraphicsImpl::getPixels({this:?}, {pixels:?}, {mask:?}, {offset}, {scan_length}, {x}, {y}, {width}, {height}, {format})"
        );

        if pixels.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "pixels is null").await);
        }
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let Some(bits) = byte_format_bits(format) else {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "unsupported format").await);
        };
        let argb = Self::read_argb(jvm, &this, x, y, width, height).await?;
        let length = jvm.array_length(&pixels).await?;
        let mut all: vec::Vec<i8> = jvm.load_array(&pixels, 0, length).await?;
        let mut all_mask: Option<vec::Vec<i8>> = if mask.is_null() {
            None
        } else {
            Some(jvm.load_array(&mask, 0, jvm.array_length(&mask).await?).await?)
        };
        for row in 0..height {
            for column in 0..width {
                let index = i64::from(offset) + i64::from(row) * i64::from(scan_length) + i64::from(column);
                let pixel = argb[(row * width + column) as usize];
                if index < 0 || !set_packed(&mut all, index as usize, bits, rgb_to_byte(pixel, format)) {
                    return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "pixels is too small").await);
                }
                if let Some(mask) = &mut all_mask {
                    set_packed(mask, index as usize, 1, u8::from((pixel as u32 >> 24) >= 0x80));
                }
            }
        }
        jvm.store_array(&mut pixels, 0, all).await?;
        if let Some(all_mask) = all_mask {
            jvm.store_array(&mut mask, 0, all_mask).await?;
        }
        Ok(())
    }

    // width x height values of an array, rows `scan_length` apart from `offset` (negative scan lengths go upwards).
    async fn load_rows<T>(
        jvm: &Jvm,
        array: &ClassInstanceRef<Array<T>>,
        offset: i32,
        scan_length: i32,
        width: i32,
        height: i32,
    ) -> JvmResult<vec::Vec<T>>
    where
        T: From<JavaValue> + Into<JavaValue> + Clone + Send + Sync,
    {
        let length = jvm.array_length(array).await? as i64;
        let mut values = vec::Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            let start = i64::from(offset) + i64::from(row) * i64::from(scan_length);
            if start < 0 || start + i64::from(width) > length {
                return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "pixels is too small").await);
            }
            values.extend(jvm.load_array::<T>(array, start as _, width as _).await?);
        }
        Ok(values)
    }

    async fn store_rows<T>(
        jvm: &Jvm,
        array: &mut ClassInstanceRef<Array<T>>,
        offset: i32,
        scan_length: i32,
        width: i32,
        height: i32,
        values: vec::Vec<T>,
    ) -> JvmResult<()>
    where
        T: From<JavaValue> + Into<JavaValue> + Clone + Send + Sync,
    {
        let length = jvm.array_length(array).await? as i64;
        for row in 0..height {
            let start = i64::from(offset) + i64::from(row) * i64::from(scan_length);
            if start < 0 || start + i64::from(width) > length {
                return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "pixels is too small").await);
            }
            let from = (row * width) as usize;
            jvm.store_array(array, start as _, values[from..from + width as usize].iter().cloned())
                .await?;
        }
        Ok(())
    }

    // ARGB pixels of the Graphics target (its image; Canvas graphics draw into one too), in the translated coordinates.
    async fn read_argb(jvm: &Jvm, this: &ClassInstanceRef<Self>, x: i32, y: i32, width: i32, height: i32) -> JvmResult<vec::Vec<i32>> {
        let graphics = Self::graphics(jvm, this).await?;
        let image: ClassInstanceRef<Image> = jvm
            .get_field(&graphics, "javax/microedition/lcdui/Graphics", "img", "Ljavax/microedition/lcdui/Image;")
            .await?;
        if image.is_null() {
            return Ok(vec![0; (width * height) as usize]);
        }
        let translate_x: i32 = jvm.get_field(&graphics, "javax/microedition/lcdui/Graphics", "translateX", "I").await?;
        let translate_y: i32 = jvm.get_field(&graphics, "javax/microedition/lcdui/Graphics", "translateY", "I").await?;
        let image_width: i32 = jvm
            .invoke_virtual(&image, "javax/microedition/lcdui/Image", "getWidth", "()I", ())
            .await?;
        let image_height: i32 = jvm
            .invoke_virtual(&image, "javax/microedition/lcdui/Image", "getHeight", "()I", ())
            .await?;
        // getRGB throws outside the image; a phone reads the part inside and leaves the rest transparent black.
        let (left, top) = (x + translate_x, y + translate_y);
        let (x0, y0) = (left.max(0), top.max(0));
        let (x1, y1) = ((left + width).min(image_width), (top + height).min(image_height));
        let mut argb = vec![0; (width * height) as usize];
        if x1 <= x0 || y1 <= y0 {
            return Ok(argb);
        }
        let (inner_width, inner_height) = (x1 - x0, y1 - y0);
        let buffer = jvm.instantiate_array("I", (inner_width * inner_height) as usize).await?;
        let _: () = jvm
            .invoke_virtual(
                &image,
                "javax/microedition/lcdui/Image",
                "getRGB",
                "([IIIIIII)V",
                (buffer.clone(), 0, inner_width, x0, y0, inner_width, inner_height),
            )
            .await?;
        let buffer: ClassInstanceRef<Array<i32>> = buffer.into();
        let inner: vec::Vec<i32> = jvm.load_array(&buffer, 0, (inner_width * inner_height) as usize).await?;
        for row in 0..inner_height {
            for column in 0..inner_width {
                let target = ((y0 - top + row) * width + (x0 - left + column)) as usize;
                argb[target] = inner[(row * inner_width + column) as usize];
            }
        }
        Ok(argb)
    }

    // Draws width x height ARGB pixels at (x, y) after the Nokia manipulation (flips, then a counter-clockwise rotation).
    async fn draw_argb(
        jvm: &Jvm,
        this: &ClassInstanceRef<Self>,
        argb: vec::Vec<i32>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        manipulation: i32,
        process_alpha: bool,
    ) -> JvmResult<()> {
        let Some(transform) = manipulation_to_transform(manipulation) else {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid manipulation").await);
        };
        let (out_width, out_height, out) = transform_pixels(&argb, width, height, MIDP_TRANSFORMS[transform as usize]);
        let mut array = jvm.instantiate_array("I", out.len()).await?;
        jvm.store_array(&mut array, 0, out).await?;
        let graphics = Self::graphics(jvm, this).await?;
        jvm.invoke_virtual(
            &graphics,
            "javax/microedition/lcdui/Graphics",
            "drawRGB",
            "([IIIIIIIZ)V",
            (array, 0, out_width, x, y, out_width, out_height, process_alpha),
        )
        .await
    }
}

// The pixels moved by a 2x2 screen-space matrix, then shifted back to start at (0, 0).
fn transform_pixels(pixels: &[i32], width: i32, height: i32, matrix: [i32; 4]) -> (i32, i32, vec::Vec<i32>) {
    if matrix == [1, 0, 0, 1] {
        return (width, height, pixels.to_vec());
    }
    let map = |x: i32, y: i32| (matrix[0] * x + matrix[1] * y, matrix[2] * x + matrix[3] * y);
    let corners = [map(0, 0), map(width - 1, 0), map(0, height - 1), map(width - 1, height - 1)];
    let min_x = corners.iter().map(|c| c.0).min().unwrap_or(0);
    let min_y = corners.iter().map(|c| c.1).min().unwrap_or(0);
    let (out_width, out_height) = if matrix[1] != 0 { (height, width) } else { (width, height) };
    let mut out = vec![0; pixels.len()];
    for y in 0..height {
        for x in 0..width {
            let (tx, ty) = map(x, y);
            out[((ty - min_y) * out_width + (tx - min_x)) as usize] = pixels[(y * width + x) as usize];
        }
    }
    (out_width, out_height, out)
}

const fn expand(value: u32, bits: u32) -> u32 {
    if bits == 0 { 0 } else { value * 255 / ((1 << bits) - 1) }
}

const fn shrink(value: u32, bits: u32) -> u32 {
    (value * ((1 << bits) - 1) + 127) / 255
}

// (alpha bits, red, green, blue) of the 16-bit formats.
const fn short_layout(format: i32) -> (u32, u32, u32, u32) {
    match format {
        4444 => (4, 4, 4, 4),
        444 => (0, 4, 4, 4),
        555 => (0, 5, 5, 5),
        1555 => (1, 5, 5, 5),
        _ => (0, 5, 6, 5), // 565
    }
}

fn short_to_argb(value: u16, format: i32) -> i32 {
    let (a, r, g, b) = short_layout(format);
    let value = value as u32;
    let blue = expand(value & ((1 << b) - 1), b);
    let green = expand((value >> b) & ((1 << g) - 1), g);
    let red = expand((value >> (b + g)) & ((1 << r) - 1), r);
    let alpha = if a == 0 {
        255
    } else {
        expand((value >> (b + g + r)) & ((1 << a) - 1), a)
    };
    ((alpha << 24) | (red << 16) | (green << 8) | blue) as i32
}

fn argb_to_short(pixel: i32, format: i32) -> u16 {
    let (a, r, g, b) = short_layout(format);
    let pixel = pixel as u32;
    let alpha = shrink(pixel >> 24, a);
    let red = shrink((pixel >> 16) & 0xff, r);
    let green = shrink((pixel >> 8) & 0xff, g);
    let blue = shrink(pixel & 0xff, b);
    ((alpha << (r + g + b)) | (red << (g + b)) | (green << b) | blue) as u16
}

// Bits per pixel of the byte formats this implements (vertical 1-bit layout is not).
const fn byte_format_bits(format: i32) -> Option<u32> {
    match format {
        1 => Some(1),
        2 => Some(2),
        4 => Some(4),
        8 | 332 => Some(8),
        _ => None,
    }
}

// The `index`-th value of `bits` bits, most significant bits first.
fn packed(data: &[i8], index: usize, bits: u32) -> Option<u8> {
    let bit = index * bits as usize;
    let byte = *data.get(bit / 8)? as u8;
    let shift = 8 - bits as usize - bit % 8;
    Some((byte >> shift) & ((1u16 << bits) - 1) as u8)
}

fn set_packed(data: &mut [i8], index: usize, bits: u32, value: u8) -> bool {
    let bit = index * bits as usize;
    let Some(byte) = data.get_mut(bit / 8) else {
        return false;
    };
    let shift = 8 - bits as usize - bit % 8;
    let mask = (((1u16 << bits) - 1) as u8) << shift;
    *byte = ((*byte as u8 & !mask) | ((value << shift) & mask)) as i8;
    true
}

// Gray formats: 1-bit "on" is black (Nokia's monochrome displays), more bits go from black (0) to white.
fn byte_to_rgb(value: u8, format: i32) -> i32 {
    let value = value as u32;
    match format {
        332 => ((expand(value >> 5, 3) << 16) | (expand((value >> 2) & 7, 3) << 8) | expand(value & 3, 2)) as i32,
        1 => {
            if value != 0 {
                0
            } else {
                0xffffff
            }
        }
        _ => {
            let gray = expand(value, byte_format_bits(format).unwrap_or(8));
            ((gray << 16) | (gray << 8) | gray) as i32
        }
    }
}

fn rgb_to_byte(pixel: i32, format: i32) -> u8 {
    let pixel = pixel as u32;
    let (red, green, blue) = ((pixel >> 16) & 0xff, (pixel >> 8) & 0xff, pixel & 0xff);
    match format {
        332 => ((shrink(red, 3) << 5) | (shrink(green, 3) << 2) | shrink(blue, 2)) as u8,
        _ => {
            let gray = (red * 299 + green * 587 + blue * 114) / 1000;
            if format == 1 {
                u8::from(gray < 128)
            } else {
                shrink(gray, byte_format_bits(format).unwrap_or(8)) as u8
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{argb_to_short, manipulation_to_transform, packed, set_packed, short_to_argb, transform_pixels};

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

    #[test]
    fn sixteen_bit_formats_round_trip() {
        assert_eq!(short_to_argb(0xf800, 565), 0xffff0000u32 as i32);
        assert_eq!(short_to_argb(0x0f00, 4444), 0x00ff0000);
        assert_eq!(argb_to_short(0xff00ff00u32 as i32, 565), 0x07e0);
        assert_eq!(argb_to_short(0x80ffffffu32 as i32, 1555), 0xffff);
        for value in [0u16, 0x1234, 0x7fff, 0xabcd] {
            assert_eq!(argb_to_short(short_to_argb(value, 4444), 4444), value);
        }
    }

    #[test]
    fn packed_pixels() {
        let mut data = [0i8; 2];
        assert!(set_packed(&mut data, 1, 1, 1));
        assert_eq!(data[0] as u8, 0b0100_0000);
        assert_eq!(packed(&data, 1, 1), Some(1));
        assert!(set_packed(&mut data, 3, 4, 0xa));
        assert_eq!(packed(&data, 3, 4), Some(0xa));
        assert_eq!(packed(&data, 16, 1), None);
    }

    #[test]
    fn rotation_moves_pixels() {
        // 2x1 [a, b] rotated by the ROT90 matrix becomes 1x2.
        let (w, h, out) = transform_pixels(&[1, 2], 2, 1, [0, -1, 1, 0]);
        assert_eq!((w, h), (1, 2));
        assert_eq!(out, vec![1, 2]);
    }
}

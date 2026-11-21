use alloc::{string::String as RustString, vec};

use jvm::{Array, ClassInstanceRef, JavaChar, Jvm, Result as JvmResult, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_backend::canvas::{FONT_SIZE, TextStyle};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

// class javax.microedition.lcdui.Font
pub struct Font;

impl Font {
    pub const HEIGHT: i32 = 12; // of the default font

    // The text style of a font (the default one for null): SIZE_SMALL (8), SIZE_MEDIUM (0) and SIZE_LARGE (16) in
    // points, STYLE_BOLD (1) and STYLE_UNDERLINED (4). Italic is drawn upright, and every face is the one font.
    pub async fn text_style(jvm: &Jvm, font: &ClassInstanceRef<Self>) -> JvmResult<TextStyle> {
        if font.is_null() {
            return Ok(TextStyle::default());
        }
        let style: i32 = jvm.get_field(font, "javax/microedition/lcdui/Font", "style", "I").await?;
        let size: i32 = jvm.get_field(font, "javax/microedition/lcdui/Font", "size", "I").await?;

        Ok(TextStyle {
            size: match size {
                8 => 8.0,
                16 => 13.0,
                _ => FONT_SIZE,
            },
            bold: style & 1 != 0,
            underlined: style & 4 != 0,
        })
    }

    async fn string_width_of(jvm: &Jvm, context: &mut WieJvmContext, this: &ClassInstanceRef<Self>, string: &str) -> JvmResult<i32> {
        let style = Self::text_style(jvm, this).await?;

        Ok(style.string_width(context.system().platform().font(), string))
    }

    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "javax/microedition/lcdui/Font",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::cl_init, MethodAccessFlags::STATIC),
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::empty()),
                JavaMethodProto::new("getHeight", "()I", Self::get_height, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getBaselinePosition", "()I", Self::get_baseline_position, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("stringWidth", "(Ljava/lang/String;)I", Self::string_width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "substringWidth",
                    "(Ljava/lang/String;II)I",
                    Self::substring_width,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("charWidth", "(C)I", Self::char_width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("charsWidth", "([CII)I", Self::chars_width, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getFont",
                    "(III)Ljavax/microedition/lcdui/Font;",
                    Self::get_font,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new(
                    "getFont",
                    "(I)Ljavax/microedition/lcdui/Font;",
                    Self::get_font_by_specifier,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
                JavaMethodProto::new("getFace", "()I", Self::get_face, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getStyle", "()I", Self::get_style, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getSize", "()I", Self::get_size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isPlain", "()Z", Self::is_plain, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isBold", "()Z", Self::is_bold, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isItalic", "()Z", Self::is_italic, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("isUnderlined", "()Z", Self::is_underlined, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getDefaultFont",
                    "()Ljavax/microedition/lcdui/Font;",
                    Self::get_default_font,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("face", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("style", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("size", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new(
                    "FACE_SYSTEM",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "FACE_MONOSPACE",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "FACE_PROPORTIONAL",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "STYLE_PLAIN",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "STYLE_BOLD",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "STYLE_ITALIC",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "STYLE_UNDERLINED",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "SIZE_SMALL",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "SIZE_MEDIUM",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new(
                    "SIZE_LARGE",
                    "I",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
            ],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL,
        }
    }

    async fn cl_init(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.Font::<clinit>");

        jvm.put_static_field("javax/microedition/lcdui/Font", "FACE_SYSTEM", "I", 0).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "FACE_MONOSPACE", "I", 32).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "FACE_PROPORTIONAL", "I", 64)
            .await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "STYLE_PLAIN", "I", 0).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "STYLE_BOLD", "I", 1).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "STYLE_ITALIC", "I", 2).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "STYLE_UNDERLINED", "I", 4).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "SIZE_MEDIUM", "I", 0).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "SIZE_SMALL", "I", 8).await?;
        jvm.put_static_field("javax/microedition/lcdui/Font", "SIZE_LARGE", "I", 16).await?;

        Ok(())
    }

    async fn init(_: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Font>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.Font::<init>({this:?})");

        Ok(())
    }

    async fn get_height(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::getHeight({this:?})");

        Ok(Self::text_style(jvm, &this).await?.height())
    }

    async fn get_baseline_position(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::getBaselinePosition({this:?})");

        Ok(Self::text_style(jvm, &this).await?.baseline())
    }

    async fn get_default_font(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<ClassInstanceRef<Self>> {
        tracing::debug!("javax.microedition.lcdui.Font::getDefaultFont");

        Ok(jvm.new_class("javax/microedition/lcdui/Font", "()V", []).await?.into())
    }

    async fn get_font(jvm: &Jvm, _: &mut WieJvmContext, face: i32, style: i32, size: i32) -> JvmResult<ClassInstanceRef<Font>> {
        tracing::debug!("javax.microedition.lcdui.Font::getFont({face}, {style}, {size})");

        let mut font = jvm.new_class("javax/microedition/lcdui/Font", "()V", []).await?;
        jvm.put_field(&mut font, "javax/microedition/lcdui/Font", "face", "I", face).await?;
        jvm.put_field(&mut font, "javax/microedition/lcdui/Font", "style", "I", style).await?;
        jvm.put_field(&mut font, "javax/microedition/lcdui/Font", "size", "I", size).await?;

        Ok(font.into())
    }

    async fn get_font_by_specifier(jvm: &Jvm, _: &mut WieJvmContext, font_specifier: i32) -> JvmResult<ClassInstanceRef<Font>> {
        tracing::debug!("javax.microedition.lcdui.Font::getFont({font_specifier})");

        // FONT_STATIC_TEXT (0) and FONT_INPUT_TEXT (1) are the default font
        jvm.invoke_static("javax/microedition/lcdui/Font", "getDefaultFont", "()Ljavax/microedition/lcdui/Font;", ())
            .await
    }

    async fn get_face(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::getFace({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/Font", "face", "I").await
    }

    async fn get_style(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::getStyle({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/Font", "style", "I").await
    }

    async fn get_size(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::getSize({this:?})");

        jvm.get_field(&this, "javax/microedition/lcdui/Font", "size", "I").await
    }

    async fn is_plain(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.Font::isPlain({this:?})");

        Ok(jvm.get_field::<i32>(&this, "javax/microedition/lcdui/Font", "style", "I").await? == 0)
    }

    async fn is_bold(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.Font::isBold({this:?})");

        Ok(jvm.get_field::<i32>(&this, "javax/microedition/lcdui/Font", "style", "I").await? & 1 != 0)
    }

    async fn is_italic(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.Font::isItalic({this:?})");

        Ok(jvm.get_field::<i32>(&this, "javax/microedition/lcdui/Font", "style", "I").await? & 2 != 0)
    }

    async fn is_underlined(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.Font::isUnderlined({this:?})");

        Ok(jvm.get_field::<i32>(&this, "javax/microedition/lcdui/Font", "style", "I").await? & 4 != 0)
    }

    async fn string_width(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, string: ClassInstanceRef<String>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::stringWidth({this:?}, {string:?})");

        let string = JavaLangString::to_rust_string(jvm, &string).await?;

        Self::string_width_of(jvm, context, &this, &string).await
    }

    async fn substring_width(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        string: ClassInstanceRef<String>,
        offset: i32,
        len: i32,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::substringWidth({this:?}, {string:?}, {offset}, {len})");

        let string = JavaLangString::to_rust_string(jvm, &string).await?;
        let substring = string.chars().skip(offset as usize).take(len as usize).collect::<RustString>();

        Self::string_width_of(jvm, context, &this, &substring).await
    }

    async fn char_width(jvm: &Jvm, context: &mut WieJvmContext, this: ClassInstanceRef<Self>, char: JavaChar) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::charWidth({this:?}, {char})");

        let string = RustString::from_utf16_lossy(&[char]);

        Self::string_width_of(jvm, context, &this, &string).await
    }

    async fn chars_width(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        chars: ClassInstanceRef<Array<JavaChar>>,
        offset: i32,
        len: i32,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.Font::charsWidth({this:?}, {chars:?}, {offset}, {len})");

        let chars = jvm.load_array(&chars, offset as _, len as _).await?;
        let string = RustString::from_utf16_lossy(&chars);

        Self::string_width_of(jvm, context, &this, &string).await
    }
}

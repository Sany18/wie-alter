use alloc::{string::String as RustString, vec, vec::Vec};

use jvm::{Array, ClassInstanceRef, JavaChar, Jvm, Result as JvmResult, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

const CLASS: &str = "javax/microedition/lcdui/TextField";
const CONSTRAINT_MASK: i32 = 0xffff;
const MAX_CONSTRAINT: i32 = 5; // ANY, EMAILADDR, NUMERIC, PHONENUMBER, URL, DECIMAL

// class javax.microedition.lcdui.TextField
// The text, its maximum size and the input constraints, with the editing methods of MIDP. TextBox keeps its content in a
// TextField of its own.
pub struct TextField;

impl TextField {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: CLASS,
            parent_class: Some("javax/microedition/lcdui/Item"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/String;II)V",
                    Self::init,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getString", "()Ljava/lang/String;", Self::get_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setString", "(Ljava/lang/String;)V", Self::set_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getChars", "([C)I", Self::get_chars, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setChars", "([CII)V", Self::set_chars, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("insert", "(Ljava/lang/String;I)V", Self::insert, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("insert", "([CIII)V", Self::insert_chars, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("delete", "(II)V", Self::delete, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getMaxSize", "()I", Self::get_max_size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setMaxSize", "(I)I", Self::set_max_size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("size", "()I", Self::size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getCaretPosition", "()I", Self::get_caret_position, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getConstraints", "()I", Self::get_constraints, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setConstraints", "(I)V", Self::set_constraints, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setInitialInputMode",
                    "(Ljava/lang/String;)V",
                    Self::set_initial_input_mode,
                    MethodAccessFlags::PUBLIC,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("text", "Ljava/lang/String;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("maxSize", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("constraints", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        label: ClassInstanceRef<String>,
        text: ClassInstanceRef<String>,
        max_size: i32,
        constraints: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::<init>({this:?}, {label:?}, {text:?}, {max_size}, {constraints})");

        let _: () = jvm.invoke_special(&this, "javax/microedition/lcdui/Item", "<init>", "()V", ()).await?;
        let _: () = jvm
            .invoke_virtual(&this, "javax/microedition/lcdui/Item", "setLabel", "(Ljava/lang/String;)V", (label,))
            .await?;
        if max_size <= 0 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "maxSize must be positive").await);
        }
        Self::check_constraints(jvm, constraints).await?;
        jvm.put_field(&mut this, CLASS, "maxSize", "I", max_size).await?;
        jvm.put_field(&mut this, CLASS, "constraints", "I", constraints).await?;

        let text = if text.is_null() {
            Vec::new()
        } else {
            JavaLangString::to_rust_string(jvm, &text).await?.encode_utf16().collect()
        };
        Self::store(jvm, &mut this, text).await
    }

    async fn get_string(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<String>> {
        tracing::debug!("javax.microedition.lcdui.TextField::getString({this:?})");

        jvm.get_field(&this, CLASS, "text", "Ljava/lang/String;").await
    }

    async fn set_string(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, text: ClassInstanceRef<String>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::setString({this:?}, {text:?})");

        let text = if text.is_null() {
            Vec::new()
        } else {
            JavaLangString::to_rust_string(jvm, &text).await?.encode_utf16().collect()
        };
        Self::store(jvm, &mut this, text).await
    }

    async fn get_chars(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        mut data: ClassInstanceRef<Array<JavaChar>>,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextField::getChars({this:?}, {data:?})");

        let text = Self::text(jvm, &this).await?;
        if data.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "data is null").await);
        }
        if jvm.array_length(&data).await? < text.len() {
            return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "data is too short").await);
        }
        let length = text.len() as i32;
        jvm.store_array(&mut data, 0, text).await?;

        Ok(length)
    }

    async fn set_chars(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<JavaChar>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::setChars({this:?}, {data:?}, {offset}, {length})");

        let text = if data.is_null() {
            Vec::new()
        } else {
            Self::load_chars(jvm, &data, offset, length).await?
        };
        Self::store(jvm, &mut this, text).await
    }

    async fn insert(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        src: ClassInstanceRef<String>,
        position: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::insert({this:?}, {src:?}, {position})");

        if src.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "src is null").await);
        }
        let src: Vec<u16> = JavaLangString::to_rust_string(jvm, &src).await?.encode_utf16().collect();
        Self::insert_at(jvm, &mut this, src, position).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_chars(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<JavaChar>>,
        offset: i32,
        length: i32,
        position: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::insert({this:?}, {data:?}, {offset}, {length}, {position})");

        if data.is_null() {
            return Err(jvm.exception("java/lang/NullPointerException", "data is null").await);
        }
        let src = Self::load_chars(jvm, &data, offset, length).await?;
        Self::insert_at(jvm, &mut this, src, position).await
    }

    async fn delete(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, offset: i32, length: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::delete({this:?}, {offset}, {length})");

        let mut text = Self::text(jvm, &this).await?;
        if offset < 0 || length < 0 || offset as usize + length as usize > text.len() {
            return Err(jvm.exception("java/lang/StringIndexOutOfBoundsException", "invalid range").await);
        }
        text.drain(offset as usize..(offset + length) as usize);
        Self::store(jvm, &mut this, text).await
    }

    async fn get_max_size(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextField::getMaxSize({this:?})");

        jvm.get_field(&this, CLASS, "maxSize", "I").await
    }

    async fn set_max_size(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, max_size: i32) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextField::setMaxSize({this:?}, {max_size})");

        if max_size <= 0 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "maxSize must be positive").await);
        }
        jvm.put_field(&mut this, CLASS, "maxSize", "I", max_size).await?;
        // The contents are truncated to the new maximum.
        let mut text = Self::text(jvm, &this).await?;
        if text.len() > max_size as usize {
            text.truncate(max_size as usize);
            Self::store(jvm, &mut this, text).await?;
        }

        Ok(max_size)
    }

    async fn size(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextField::size({this:?})");

        Ok(Self::text(jvm, &this).await?.len() as i32)
    }

    async fn get_caret_position(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextField::getCaretPosition({this:?})");

        // There is no on-screen editing: the caret stays after the text.
        Ok(Self::text(jvm, &this).await?.len() as i32)
    }

    async fn get_constraints(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextField::getConstraints({this:?})");

        jvm.get_field(&this, CLASS, "constraints", "I").await
    }

    async fn set_constraints(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, constraints: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::setConstraints({this:?}, {constraints})");

        Self::check_constraints(jvm, constraints).await?;
        jvm.put_field(&mut this, CLASS, "constraints", "I", constraints).await
    }

    async fn set_initial_input_mode(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, mode: ClassInstanceRef<String>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextField::setInitialInputMode({this:?}, {mode:?})");

        let _ = jvm;
        Ok(())
    }

    async fn check_constraints(jvm: &Jvm, constraints: i32) -> JvmResult<()> {
        if (constraints & CONSTRAINT_MASK) > MAX_CONSTRAINT || constraints < 0 {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid constraints").await);
        }
        Ok(())
    }

    async fn load_chars(jvm: &Jvm, data: &ClassInstanceRef<Array<JavaChar>>, offset: i32, length: i32) -> JvmResult<Vec<u16>> {
        let array_length = jvm.array_length(data).await?;
        if offset < 0 || length < 0 || offset as usize + length as usize > array_length {
            return Err(jvm.exception("java/lang/ArrayIndexOutOfBoundsException", "invalid range").await);
        }
        jvm.load_array(data, offset as usize, length as usize).await
    }

    async fn text(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<Vec<u16>> {
        let text: ClassInstanceRef<String> = jvm.get_field(this, CLASS, "text", "Ljava/lang/String;").await?;
        Ok(JavaLangString::to_rust_string(jvm, &text).await?.encode_utf16().collect())
    }

    async fn insert_at(jvm: &Jvm, this: &mut ClassInstanceRef<Self>, src: Vec<u16>, position: i32) -> JvmResult<()> {
        let mut text = Self::text(jvm, this).await?;
        let position = position.clamp(0, text.len() as i32) as usize;
        text.splice(position..position, src);
        Self::store(jvm, this, text).await
    }

    async fn store(jvm: &Jvm, this: &mut ClassInstanceRef<Self>, text: Vec<u16>) -> JvmResult<()> {
        let max_size: i32 = jvm.get_field(this, CLASS, "maxSize", "I").await?;
        if text.len() > max_size as usize {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "text exceeds the maximum size").await);
        }
        let text = RustString::from_utf16_lossy(&text);
        let text = JavaLangString::from_rust_string(jvm, &text).await?;
        jvm.put_field(this, CLASS, "text", "Ljava/lang/String;", text).await
    }
}

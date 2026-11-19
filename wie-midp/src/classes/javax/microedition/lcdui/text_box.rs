use alloc::vec;

use jvm::{Array, ClassInstanceRef, JavaChar, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::javax::microedition::lcdui::TextField;

const CLASS: &str = "javax/microedition/lcdui/TextBox";
const FIELD: &str = "javax/microedition/lcdui/TextField";

// class javax.microedition.lcdui.TextBox
// The text is kept by a TextField of the TextBox, which has the same editing methods.
pub struct TextBox;

impl TextBox {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: CLASS,
            parent_class: Some("javax/microedition/lcdui/Screen"),
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
            fields: vec![JavaFieldProto::new(
                "content",
                "Ljavax/microedition/lcdui/TextField;",
                FieldAccessFlags::PRIVATE,
            )],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        title: ClassInstanceRef<String>,
        text: ClassInstanceRef<String>,
        max_size: i32,
        constraints: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::<init>({this:?}, {title:?}, {text:?}, {max_size}, {constraints})");

        let _: () = jvm.invoke_special(&this, "javax/microedition/lcdui/Screen", "<init>", "()V", ()).await?;
        let _: () = jvm
            .invoke_virtual(
                &this,
                "javax/microedition/lcdui/Displayable",
                "setTitle",
                "(Ljava/lang/String;)V",
                (title,),
            )
            .await?;
        let null: ClassInstanceRef<String> = None.into();
        let content = jvm
            .new_class(FIELD, "(Ljava/lang/String;Ljava/lang/String;II)V", (null, text, max_size, constraints))
            .await?;
        jvm.put_field(&mut this, CLASS, "content", "Ljavax/microedition/lcdui/TextField;", content)
            .await
    }

    async fn content(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<TextField>> {
        jvm.get_field(this, CLASS, "content", "Ljavax/microedition/lcdui/TextField;").await
    }

    async fn get_string(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<String>> {
        tracing::debug!("javax.microedition.lcdui.TextBox::getString({this:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "getString", "()Ljava/lang/String;", ()).await
    }

    async fn set_string(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, text: ClassInstanceRef<String>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::setString({this:?}, {text:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "setString", "(Ljava/lang/String;)V", (text,)).await
    }

    async fn get_chars(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<JavaChar>>,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextBox::getChars({this:?}, {data:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "getChars", "([C)I", (data,)).await
    }

    async fn set_chars(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<JavaChar>>,
        offset: i32,
        length: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::setChars({this:?}, {data:?}, {offset}, {length})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "setChars", "([CII)V", (data, offset, length)).await
    }

    async fn insert(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        src: ClassInstanceRef<String>,
        position: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::insert({this:?}, {src:?}, {position})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "insert", "(Ljava/lang/String;I)V", (src, position))
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_chars(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        data: ClassInstanceRef<Array<JavaChar>>,
        offset: i32,
        length: i32,
        position: i32,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::insert({this:?}, {data:?}, {offset}, {length}, {position})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "insert", "([CIII)V", (data, offset, length, position))
            .await
    }

    async fn delete(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, offset: i32, length: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::delete({this:?}, {offset}, {length})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "delete", "(II)V", (offset, length)).await
    }

    async fn get_max_size(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextBox::getMaxSize({this:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "getMaxSize", "()I", ()).await
    }

    async fn set_max_size(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, max_size: i32) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextBox::setMaxSize({this:?}, {max_size})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "setMaxSize", "(I)I", (max_size,)).await
    }

    async fn size(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextBox::size({this:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "size", "()I", ()).await
    }

    async fn get_caret_position(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextBox::getCaretPosition({this:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "getCaretPosition", "()I", ()).await
    }

    async fn get_constraints(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.TextBox::getConstraints({this:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "getConstraints", "()I", ()).await
    }

    async fn set_constraints(jvm: &Jvm, _context: &mut WieJvmContext, this: ClassInstanceRef<Self>, constraints: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::setConstraints({this:?}, {constraints})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "setConstraints", "(I)V", (constraints,)).await
    }

    async fn set_initial_input_mode(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        mode: ClassInstanceRef<String>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.TextBox::setInitialInputMode({this:?}, {mode:?})");

        let content = Self::content(jvm, &this).await?;
        jvm.invoke_virtual(&content, FIELD, "setInitialInputMode", "(Ljava/lang/String;)V", (mode,))
            .await
    }
}

#[cfg(test)]
mod test {
    use alloc::boxed::Box;

    use jvm::{ClassInstanceRef, runtime::JavaLangString};
    use rustjava_runtime::classes::java::lang::String;
    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::{classes::javax::microedition::lcdui::TextBox, get_protos};

    #[test]
    fn text_box_edits_its_text_within_the_maximum_size() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            let title = ClassInstanceRef::<String>::from(JavaLangString::from_rust_string(&jvm, "Name").await?);
            let text = ClassInstanceRef::<String>::from(JavaLangString::from_rust_string(&jvm, "AAA").await?);
            let text_box: ClassInstanceRef<TextBox> = jvm
                .new_class(
                    "javax/microedition/lcdui/TextBox",
                    "(Ljava/lang/String;Ljava/lang/String;II)V",
                    (title, text, 6, 0),
                )
                .await?
                .into();
            let class = "javax/microedition/lcdui/TextBox";

            let inserted = ClassInstanceRef::<String>::from(JavaLangString::from_rust_string(&jvm, "BC").await?);
            let _: () = jvm
                .invoke_virtual(&text_box, class, "insert", "(Ljava/lang/String;I)V", (inserted, 1))
                .await?;
            let _: () = jvm.invoke_virtual(&text_box, class, "delete", "(II)V", (0, 1)).await?;
            let string: ClassInstanceRef<String> = jvm.invoke_virtual(&text_box, class, "getString", "()Ljava/lang/String;", ()).await?;
            assert_eq!(JavaLangString::to_rust_string(&jvm, &string).await?, "BCAA");
            let size: i32 = jvm.invoke_virtual(&text_box, class, "size", "()I", ()).await?;
            assert_eq!(size, 4);

            let too_long = ClassInstanceRef::<String>::from(JavaLangString::from_rust_string(&jvm, "1234567").await?);
            let result: jvm::Result<()> = jvm
                .invoke_virtual(&text_box, class, "setString", "(Ljava/lang/String;)V", (too_long,))
                .await;
            assert!(result.is_err(), "a text longer than the maximum size is refused");

            let max: i32 = jvm.invoke_virtual(&text_box, class, "setMaxSize", "(I)I", (2,)).await?;
            assert_eq!(max, 2);
            let string: ClassInstanceRef<String> = jvm.invoke_virtual(&text_box, class, "getString", "()Ljava/lang/String;", ()).await?;
            assert_eq!(
                JavaLangString::to_rust_string(&jvm, &string).await?,
                "BC",
                "a smaller maximum truncates the text"
            );

            Ok(())
        })
    }
}

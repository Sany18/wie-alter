use alloc::vec;

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::String;

use wie_jvm_support::{WieJavaClassProto, WieJvmContext};

use crate::classes::{
    javax::microedition::lcdui::{ChoiceGroup, Command, Font, Graphics, Image},
    net::wie::{KeyboardEventType, MIDPKeyCode},
};

const IMPLICIT: i32 = 3;

// class javax.microedition.lcdui.List
// Elements, selection, layout and painting are kept in a private ChoiceGroup; IMPLICIT lists use it as an
// indicator-less exclusive choice whose selection follows the highlight.
pub struct List;

#[allow(clippy::too_many_arguments)]
impl List {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "javax/microedition/lcdui/List",
            parent_class: Some("javax/microedition/lcdui/Screen"),
            interfaces: vec!["javax/microedition/lcdui/Choice"],
            methods: vec![
                JavaMethodProto::new("<clinit>", "()V", Self::cl_init, MethodAccessFlags::STATIC),
                JavaMethodProto::new("<init>", "(Ljava/lang/String;I)V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "<init>",
                    "(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V",
                    Self::init_with_elements,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("size", "()I", Self::size, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getString", "(I)Ljava/lang/String;", Self::get_string, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getImage",
                    "(I)Ljavax/microedition/lcdui/Image;",
                    Self::get_image,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "append",
                    "(Ljava/lang/String;Ljavax/microedition/lcdui/Image;)I",
                    Self::append,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new(
                    "insert",
                    "(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V",
                    Self::insert,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("delete", "(I)V", Self::delete, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("deleteAll", "()V", Self::delete_all, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "set",
                    "(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V",
                    Self::set,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("isSelected", "(I)Z", Self::is_selected, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getSelectedIndex", "()I", Self::get_selected_index, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getSelectedFlags", "([Z)I", Self::get_selected_flags, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setSelectedIndex", "(IZ)V", Self::set_selected_index, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setSelectedFlags", "([Z)V", Self::set_selected_flags, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("setFitPolicy", "(I)V", Self::set_fit_policy, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getFitPolicy", "()I", Self::get_fit_policy, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setFont",
                    "(ILjavax/microedition/lcdui/Font;)V",
                    Self::set_font,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("getFont", "(I)Ljavax/microedition/lcdui/Font;", Self::get_font, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "setSelectCommand",
                    "(Ljavax/microedition/lcdui/Command;)V",
                    Self::set_select_command,
                    MethodAccessFlags::PUBLIC,
                ),
                JavaMethodProto::new("handleKeyEvent", "(II)V", Self::handle_key_event, MethodAccessFlags::empty()),
                JavaMethodProto::new(
                    "handlePaintEvent",
                    "(Ljavax/microedition/lcdui/Graphics;)V",
                    Self::handle_paint_event,
                    MethodAccessFlags::empty(),
                ),
            ],
            fields: vec![
                JavaFieldProto::new(
                    "SELECT_COMMAND",
                    "Ljavax/microedition/lcdui/Command;",
                    FieldAccessFlags::PUBLIC | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
                ),
                JavaFieldProto::new("listType", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("choice", "Ljavax/microedition/lcdui/ChoiceGroup;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("selectCommand", "Ljavax/microedition/lcdui/Command;", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("scrollY", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn cl_init(jvm: &Jvm, _: &mut WieJvmContext) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::<clinit>()");

        let label = JavaLangString::from_rust_string(jvm, "").await?;
        let command = jvm
            .new_class("javax/microedition/lcdui/Command", "(Ljava/lang/String;II)V", (label, 1, 0))
            .await?;
        jvm.put_static_field(
            "javax/microedition/lcdui/List",
            "SELECT_COMMAND",
            "Ljavax/microedition/lcdui/Command;",
            command,
        )
        .await
    }

    async fn init(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, title: ClassInstanceRef<String>, list_type: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::<init>({this:?}, {title:?}, {list_type})");

        let strings = jvm.instantiate_array("Ljava/lang/String;", 0).await?;
        jvm.invoke_special(
            &this,
            "javax/microedition/lcdui/List",
            "<init>",
            "(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V",
            (title, list_type, strings, None),
        )
        .await
    }

    async fn init_with_elements(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        title: ClassInstanceRef<String>,
        list_type: i32,
        strings: ClassInstanceRef<Array<String>>,
        images: ClassInstanceRef<Array<Image>>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::<init>({this:?}, {title:?}, {list_type}, {strings:?}, {images:?})");

        if !(1..=IMPLICIT).contains(&list_type) {
            return Err(jvm.exception("java/lang/IllegalArgumentException", "invalid list type").await);
        }

        let _: () = jvm.invoke_special(&this, "javax/microedition/lcdui/Screen", "<init>", "()V", ()).await?;

        // ChoiceGroup has no IMPLICIT type; build it as EXCLUSIVE, then mark it implicit.
        let choice_type = if list_type == IMPLICIT { 1 } else { list_type };
        let mut choice: ClassInstanceRef<ChoiceGroup> = jvm
            .new_class(
                "javax/microedition/lcdui/ChoiceGroup",
                "(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V",
                (None, choice_type, strings, images),
            )
            .await?
            .into();
        if list_type == IMPLICIT {
            jvm.put_field(&mut choice, "javax/microedition/lcdui/ChoiceGroup", "choiceType", "I", IMPLICIT)
                .await?;
        }

        let select_command: ClassInstanceRef<Command> = if list_type == IMPLICIT {
            jvm.get_static_field("javax/microedition/lcdui/List", "SELECT_COMMAND", "Ljavax/microedition/lcdui/Command;")
                .await?
        } else {
            None.into()
        };

        jvm.put_field(&mut this, "javax/microedition/lcdui/List", "listType", "I", list_type)
            .await?;
        jvm.put_field(
            &mut this,
            "javax/microedition/lcdui/List",
            "choice",
            "Ljavax/microedition/lcdui/ChoiceGroup;",
            choice,
        )
        .await?;
        jvm.put_field(
            &mut this,
            "javax/microedition/lcdui/List",
            "selectCommand",
            "Ljavax/microedition/lcdui/Command;",
            select_command,
        )
        .await?;
        jvm.put_field(&mut this, "javax/microedition/lcdui/List", "scrollY", "I", 0).await?;

        jvm.invoke_virtual(
            &this,
            "javax/microedition/lcdui/Displayable",
            "setTitle",
            "(Ljava/lang/String;)V",
            (title,),
        )
        .await
    }

    async fn choice(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<ClassInstanceRef<ChoiceGroup>> {
        jvm.get_field(this, "javax/microedition/lcdui/List", "choice", "Ljavax/microedition/lcdui/ChoiceGroup;")
            .await
    }

    async fn repaint(jvm: &Jvm, this: &ClassInstanceRef<Self>) -> JvmResult<()> {
        jvm.invoke_virtual(this, "javax/microedition/lcdui/Displayable", "requestRepaint", "()V", ())
            .await
    }

    async fn size(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.List::size({this:?})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "size", "()I", ())
            .await
    }

    async fn get_string(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<ClassInstanceRef<String>> {
        tracing::debug!("javax.microedition.lcdui.List::getString({this:?}, {index})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(
            &choice,
            "javax/microedition/lcdui/ChoiceGroup",
            "getString",
            "(I)Ljava/lang/String;",
            (index,),
        )
        .await
    }

    async fn get_image(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<ClassInstanceRef<Image>> {
        tracing::debug!("javax.microedition.lcdui.List::getImage({this:?}, {index})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(
            &choice,
            "javax/microedition/lcdui/ChoiceGroup",
            "getImage",
            "(I)Ljavax/microedition/lcdui/Image;",
            (index,),
        )
        .await
    }

    async fn append(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        text: ClassInstanceRef<String>,
        image: ClassInstanceRef<Image>,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.List::append({this:?}, {text:?}, {image:?})");

        let choice = Self::choice(jvm, &this).await?;
        let index: i32 = jvm
            .invoke_virtual(
                &choice,
                "javax/microedition/lcdui/ChoiceGroup",
                "append",
                "(Ljava/lang/String;Ljavax/microedition/lcdui/Image;)I",
                (text, image),
            )
            .await?;
        Self::repaint(jvm, &this).await?;
        Ok(index)
    }

    async fn insert(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        index: i32,
        text: ClassInstanceRef<String>,
        image: ClassInstanceRef<Image>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::insert({this:?}, {index}, {text:?}, {image:?})");

        let choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(
                &choice,
                "javax/microedition/lcdui/ChoiceGroup",
                "insert",
                "(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V",
                (index, text, image),
            )
            .await?;
        Self::repaint(jvm, &this).await
    }

    async fn delete(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::delete({this:?}, {index})");

        let choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "delete", "(I)V", (index,))
            .await?;
        Self::repaint(jvm, &this).await
    }

    async fn delete_all(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::deleteAll({this:?})");

        let choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "deleteAll", "()V", ())
            .await?;
        Self::repaint(jvm, &this).await
    }

    async fn set(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        index: i32,
        text: ClassInstanceRef<String>,
        image: ClassInstanceRef<Image>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::set({this:?}, {index}, {text:?}, {image:?})");

        let choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(
                &choice,
                "javax/microedition/lcdui/ChoiceGroup",
                "set",
                "(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V",
                (index, text, image),
            )
            .await?;
        Self::repaint(jvm, &this).await
    }

    async fn is_selected(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<bool> {
        tracing::debug!("javax.microedition.lcdui.List::isSelected({this:?}, {index})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "isSelected", "(I)Z", (index,))
            .await
    }

    async fn get_selected_index(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.List::getSelectedIndex({this:?})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "getSelectedIndex", "()I", ())
            .await
    }

    async fn get_selected_flags(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        flags: ClassInstanceRef<Array<bool>>,
    ) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.List::getSelectedFlags({this:?}, {flags:?})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "getSelectedFlags", "([Z)I", (flags,))
            .await
    }

    async fn set_selected_index(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32, selected: bool) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::setSelectedIndex({this:?}, {index}, {selected})");

        let mut choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(
                &choice,
                "javax/microedition/lcdui/ChoiceGroup",
                "setSelectedIndex",
                "(IZ)V",
                (index, selected),
            )
            .await?;
        let list_type: i32 = jvm.get_field(&this, "javax/microedition/lcdui/List", "listType", "I").await?;
        if list_type == IMPLICIT && selected {
            jvm.put_field(&mut choice, "javax/microedition/lcdui/ChoiceGroup", "highlightedIndex", "I", index)
                .await?;
        }
        Self::repaint(jvm, &this).await
    }

    async fn set_selected_flags(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        flags: ClassInstanceRef<Array<bool>>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::setSelectedFlags({this:?}, {flags:?})");

        let choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "setSelectedFlags", "([Z)V", (flags,))
            .await?;
        Self::repaint(jvm, &this).await
    }

    async fn set_fit_policy(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, fit_policy: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::setFitPolicy({this:?}, {fit_policy})");

        let choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "setFitPolicy", "(I)V", (fit_policy,))
            .await?;
        Self::repaint(jvm, &this).await
    }

    async fn get_fit_policy(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>) -> JvmResult<i32> {
        tracing::debug!("javax.microedition.lcdui.List::getFitPolicy({this:?})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(&choice, "javax/microedition/lcdui/ChoiceGroup", "getFitPolicy", "()I", ())
            .await
    }

    async fn set_font(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32, font: ClassInstanceRef<Font>) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::setFont({this:?}, {index}, {font:?})");

        let choice = Self::choice(jvm, &this).await?;
        let _: () = jvm
            .invoke_virtual(
                &choice,
                "javax/microedition/lcdui/ChoiceGroup",
                "setFont",
                "(ILjavax/microedition/lcdui/Font;)V",
                (index, font),
            )
            .await?;
        Self::repaint(jvm, &this).await
    }

    async fn get_font(jvm: &Jvm, _: &mut WieJvmContext, this: ClassInstanceRef<Self>, index: i32) -> JvmResult<ClassInstanceRef<Font>> {
        tracing::debug!("javax.microedition.lcdui.List::getFont({this:?}, {index})");

        let choice = Self::choice(jvm, &this).await?;
        jvm.invoke_virtual(
            &choice,
            "javax/microedition/lcdui/ChoiceGroup",
            "getFont",
            "(I)Ljavax/microedition/lcdui/Font;",
            (index,),
        )
        .await
    }

    async fn set_select_command(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        mut this: ClassInstanceRef<Self>,
        command: ClassInstanceRef<Command>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::setSelectCommand({this:?}, {command:?})");

        let list_type: i32 = jvm.get_field(&this, "javax/microedition/lcdui/List", "listType", "I").await?;
        if list_type != IMPLICIT {
            return Ok(());
        }
        jvm.put_field(
            &mut this,
            "javax/microedition/lcdui/List",
            "selectCommand",
            "Ljavax/microedition/lcdui/Command;",
            command,
        )
        .await
    }

    async fn handle_key_event(jvm: &Jvm, _: &mut WieJvmContext, mut this: ClassInstanceRef<Self>, event_type: i32, code: i32) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::handleKeyEvent({this:?}, {event_type}, {code})");

        let pressed = event_type == KeyboardEventType::KeyPressed as i32;
        let repeated = event_type == KeyboardEventType::KeyRepeated as i32;
        if !(pressed || repeated) {
            return Ok(());
        }

        let choice = Self::choice(jvm, &this).await?;
        let list_type: i32 = jvm.get_field(&this, "javax/microedition/lcdui/List", "listType", "I").await?;
        let _: i32 = jvm
            .invoke_virtual(&choice, "javax/microedition/lcdui/Item", "handleItemKey", "(I)I", (code,))
            .await?;

        if list_type == IMPLICIT {
            let highlighted: i32 = jvm
                .get_field(&choice, "javax/microedition/lcdui/ChoiceGroup", "highlightedIndex", "I")
                .await?;
            if highlighted >= 0 {
                let _: () = jvm
                    .invoke_virtual(
                        &choice,
                        "javax/microedition/lcdui/ChoiceGroup",
                        "setSelectedIndex",
                        "(IZ)V",
                        (highlighted, true),
                    )
                    .await?;
            }
            if pressed && code == MIDPKeyCode::FIRE as i32 {
                let command: ClassInstanceRef<Command> = jvm
                    .get_field(
                        &this,
                        "javax/microedition/lcdui/List",
                        "selectCommand",
                        "Ljavax/microedition/lcdui/Command;",
                    )
                    .await?;
                if !command.is_null() {
                    let _: bool = jvm
                        .invoke_virtual(
                            &this,
                            "javax/microedition/lcdui/Displayable",
                            "dispatchCommand",
                            "(Ljavax/microedition/lcdui/Command;)Z",
                            (command,),
                        )
                        .await?;
                }
            }
        }

        // Keep the highlighted row inside the viewport.
        let width: i32 = jvm
            .invoke_virtual(&this, "javax/microedition/lcdui/Displayable", "getWidth", "()I", ())
            .await?;
        let height: i32 = jvm
            .invoke_virtual(&this, "javax/microedition/lcdui/Displayable", "getHeight", "()I", ())
            .await?;
        let bounds: ClassInstanceRef<Array<i32>> = jvm
            .invoke_virtual(&choice, "javax/microedition/lcdui/Item", "getFocusContentBounds", "(I)[I", (width,))
            .await?;
        if !bounds.is_null() {
            let bounds: vec::Vec<i32> = jvm.load_array(&bounds, 0, 2).await?;
            let (top, bottom) = (bounds[0], bounds[1]);
            let mut scroll: i32 = jvm.get_field(&this, "javax/microedition/lcdui/List", "scrollY", "I").await?;
            if top < scroll {
                scroll = top;
            } else if bottom > scroll + height {
                scroll = bottom - height;
            }
            jvm.put_field(&mut this, "javax/microedition/lcdui/List", "scrollY", "I", scroll.max(0))
                .await?;
        }

        Self::repaint(jvm, &this).await
    }

    async fn handle_paint_event(
        jvm: &Jvm,
        _: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        graphics: ClassInstanceRef<Graphics>,
    ) -> JvmResult<()> {
        tracing::debug!("javax.microedition.lcdui.List::handlePaintEvent({this:?}, {graphics:?})");

        let _: () = jvm
            .invoke_special(
                &this,
                "javax/microedition/lcdui/Screen",
                "handlePaintEvent",
                "(Ljavax/microedition/lcdui/Graphics;)V",
                (graphics.clone(),),
            )
            .await?;

        let choice = Self::choice(jvm, &this).await?;
        let width: i32 = jvm
            .invoke_virtual(&this, "javax/microedition/lcdui/Displayable", "getWidth", "()I", ())
            .await?;
        let content_height: i32 = jvm
            .invoke_virtual(&choice, "javax/microedition/lcdui/Item", "preferredContentHeight", "(I)I", (width,))
            .await?;
        let scroll: i32 = jvm.get_field(&this, "javax/microedition/lcdui/List", "scrollY", "I").await?;
        jvm.invoke_virtual(
            &choice,
            "javax/microedition/lcdui/Item",
            "paintContent",
            "(Ljavax/microedition/lcdui/Graphics;IIIIZ)V",
            (graphics, 0, -scroll, width, content_height, true),
        )
        .await
    }
}

#[cfg(test)]
mod test {
    use alloc::{boxed::Box, vec::Vec};

    use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult, runtime::JavaLangString};
    use rustjava_runtime::classes::java::lang::String;

    use test_utils::run_jvm_test;
    use wie_util::Result;

    use crate::{
        classes::{
            javax::microedition::lcdui::{Graphics, Image, List},
            net::wie::{KeyboardEventType, MIDPKeyCode},
        },
        get_protos,
    };

    async fn implicit_list(jvm: &Jvm, texts: &[&str]) -> JvmResult<ClassInstanceRef<List>> {
        let mut array: ClassInstanceRef<Array<ClassInstanceRef<String>>> = jvm.instantiate_array("Ljava/lang/String;", texts.len()).await?.into();
        let mut strings = Vec::with_capacity(texts.len());
        for text in texts {
            strings.push(ClassInstanceRef::<String>::from(JavaLangString::from_rust_string(jvm, text).await?));
        }
        jvm.store_array(&mut array, 0, strings).await?;

        Ok(jvm
            .new_class(
                "javax/microedition/lcdui/List",
                "(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V",
                (None, 3, array, None),
            )
            .await?
            .into())
    }

    async fn paint(jvm: &Jvm, list: &ClassInstanceRef<List>, width: i32, height: i32) -> JvmResult<ClassInstanceRef<Image>> {
        let image: ClassInstanceRef<Image> = jvm
            .invoke_static(
                "javax/microedition/lcdui/Image",
                "createImage",
                "(II)Ljavax/microedition/lcdui/Image;",
                (width, height),
            )
            .await?;
        let graphics: ClassInstanceRef<Graphics> = jvm
            .invoke_virtual(
                &image,
                "javax/microedition/lcdui/Image",
                "getGraphics",
                "()Ljavax/microedition/lcdui/Graphics;",
                (),
            )
            .await?;
        let _: () = jvm
            .invoke_virtual(
                list,
                "javax/microedition/lcdui/Displayable",
                "handlePaintEvent",
                "(Ljavax/microedition/lcdui/Graphics;)V",
                (graphics,),
            )
            .await?;
        Ok(image)
    }

    async fn count_color(jvm: &Jvm, image: &ClassInstanceRef<Image>, width: i32, height: i32, color: (u8, u8, u8)) -> JvmResult<usize> {
        let pixels = Image::image(jvm, image).await?;
        let mut count = 0;
        for y in 0..height {
            for x in 0..width {
                let pixel = pixels.get_pixel(x, y);
                if (pixel.r, pixel.g, pixel.b) == color {
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    #[test]
    fn implicit_list_paints_rows_and_selection_follows_keys() -> Result<()> {
        run_jvm_test(Box::new([get_protos().into()]), |jvm| async move {
            let list = implicit_list(&jvm, &["first", "second"]).await?;
            assert!(jvm.is_instance(&**list, "javax/microedition/lcdui/Choice"));
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(&list, "javax/microedition/lcdui/List", "getSelectedIndex", "()I", ())
                    .await?,
                0
            );

            let image = paint(&jvm, &list, 120, 80).await?;
            assert!(count_color(&jvm, &image, 120, 80, (0x17, 0x21, 0x2b)).await? > 0, "row text is painted");
            assert!(
                count_color(&jvm, &image, 120, 80, (0xdc, 0xec, 0xf7)).await? > 0,
                "highlighted row is painted"
            );

            let _: () = jvm
                .invoke_virtual(
                    &list,
                    "javax/microedition/lcdui/Displayable",
                    "handleKeyEvent",
                    "(II)V",
                    (KeyboardEventType::KeyPressed as i32, MIDPKeyCode::DOWN as i32),
                )
                .await?;
            assert_eq!(
                jvm.invoke_virtual::<_, i32>(&list, "javax/microedition/lcdui/List", "getSelectedIndex", "()I", ())
                    .await?,
                1
            );
            Ok(())
        })
    }
}

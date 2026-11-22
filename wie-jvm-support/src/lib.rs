#![no_std]
extern crate alloc;

mod context;
mod jvm_implementation;
pub mod native;
mod runtime;

use alloc::{boxed::Box, format, string::ToString};

use jvm::{JavaError, JavaValue, Jvm, runtime::JavaLangString};
use rustjava_runtime::Runtime;

use wie_backend::System;
use wie_util::{Result, WieError};

pub use context::{WieJavaClassProto, WieJvmContext};
pub use jvm_implementation::{JvmImplementation, RustJavaJvmImplementation};
use runtime::JvmRuntime;

pub static WIE_RUSTJAR: &str = "wie.rustjar";

pub struct JvmSupport;

impl JvmSupport {
    pub async fn new_jvm<T>(
        system: &System,
        jar_name: Option<&str>,
        protos: Box<[Box<[WieJavaClassProto]>]>,
        properties: &[(&str, &str)],
        implementation: T,
    ) -> Result<Jvm>
    where
        T: JvmImplementation + Sync + Send + 'static,
    {
        let external_roots = implementation.external_roots();
        let runtime = JvmRuntime::new(system.clone(), implementation, protos);

        let class_path = if let Some(x) = jar_name {
            if cfg!(windows) {
                format!("{WIE_RUSTJAR};{x}")
            } else {
                format!("{WIE_RUSTJAR}:{x}")
            }
        } else {
            WIE_RUSTJAR.to_string()
        };

        let properties = [
            ("file.encoding", "EUC-KR"),
            ("java.class.path", &class_path),
            //("rustjava.disable_explicit_gc", "true"),
        ]
        .iter()
        .chain(properties.iter())
        .copied()
        .collect();
        let jvm = Jvm::new(
            rustjava_runtime::get_bootstrap_class_loader(Box::new(runtime.clone())),
            move || runtime.current_task_id(),
            properties,
            external_roots,
        )
        .await
        .map_err(|x| WieError::FatalError(format!("Failed to create JVM: {x}")))?;

        Ok(jvm)
    }

    pub async fn to_wie_err(jvm: &Jvm, err: JavaError) -> WieError {
        match err {
            JavaError::JavaException(x) => {
                let _frame = jvm.push_native_frame(&[JavaValue::Object(Some(x.clone()))]);
                let string_writer = jvm.new_class("java/io/StringWriter", "()V", ()).await.unwrap();
                let print_writer = jvm
                    .new_class("java/io/PrintWriter", "(Ljava/io/Writer;)V", (string_writer.clone(),))
                    .await
                    .unwrap();

                let _: () = jvm
                    .invoke_virtual(&x, "java/lang/Throwable", "printStackTrace", "(Ljava/io/PrintWriter;)V", (print_writer,))
                    .await
                    .unwrap();

                let trace = jvm
                    .invoke_virtual(&string_writer, "java/io/StringWriter", "toString", "()Ljava/lang/String;", [])
                    .await
                    .unwrap();

                WieError::FatalError(format!("\n{}", JavaLangString::to_rust_string(jvm, &trace).await.unwrap()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec::Vec};

    use jvm::{Array, ClassInstanceRef, runtime::JavaLangString};
    use test_utils::run_jvm_test;

    #[test]
    fn iso_8859_1_readers_and_writers() -> wie_util::Result<()> {
        run_jvm_test(Box::new([]), |jvm| async move {
            let text = "Año ¡Olé! ©";
            let latin1: Vec<i8> = text.chars().map(|c| c as u32 as u8 as i8).collect();

            let output = jvm.new_class("java/io/ByteArrayOutputStream", "()V", ()).await?;
            let encoding = JavaLangString::from_rust_string(&jvm, "ISO-8859-1").await?;
            let writer = jvm
                .new_class(
                    "java/io/OutputStreamWriter",
                    "(Ljava/io/OutputStream;Ljava/lang/String;)V",
                    (output.clone(), encoding.clone()),
                )
                .await?;
            let value = JavaLangString::from_rust_string(&jvm, text).await?;
            let _: () = jvm
                .invoke_virtual(&writer, "java/io/Writer", "write", "(Ljava/lang/String;)V", (value,))
                .await?;
            let _: () = jvm.invoke_virtual(&writer, "java/io/Writer", "flush", "()V", ()).await?;
            let bytes: ClassInstanceRef<Array<i8>> = jvm
                .invoke_virtual(&output, "java/io/ByteArrayOutputStream", "toByteArray", "()[B", ())
                .await?;
            let length = jvm.array_length(&bytes).await?;
            assert_eq!(jvm.load_array::<i8>(&bytes, 0, length).await?, latin1);

            let input = jvm.new_class("java/io/ByteArrayInputStream", "([B)V", (bytes,)).await?;
            let reader = jvm
                .new_class(
                    "java/io/InputStreamReader",
                    "(Ljava/io/InputStream;Ljava/lang/String;)V",
                    (input, encoding),
                )
                .await?;
            let mut read = Vec::new();
            loop {
                let char: i32 = jvm.invoke_virtual(&reader, "java/io/Reader", "read", "()I", ()).await?;
                if char == -1 {
                    break;
                }
                read.push(char::from_u32(char as u32).unwrap());
            }
            assert_eq!(read.into_iter().collect::<alloc::string::String>(), text);

            Ok(())
        })
    }

    #[test]
    fn string_value_of_object() -> wie_util::Result<()> {
        run_jvm_test(Box::new([]), |jvm| async move {
            let descriptor = "(Ljava/lang/Object;)Ljava/lang/String;";
            let object = jvm.new_class("java/lang/Integer", "(I)V", (42,)).await?;
            let value = jvm.invoke_static("java/lang/String", "valueOf", descriptor, (object,)).await?;
            assert_eq!(JavaLangString::to_rust_string(&jvm, &value).await?, "42");

            let null = ClassInstanceRef::<Array<i8>>::new(None);
            let value = jvm.invoke_static("java/lang/String", "valueOf", descriptor, (null,)).await?;
            assert_eq!(JavaLangString::to_rust_string(&jvm, &value).await?, "null");

            Ok(())
        })
    }
}

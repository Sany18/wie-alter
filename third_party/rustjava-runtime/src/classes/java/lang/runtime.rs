use alloc::{format, vec};

use jvm::{ClassInstanceRef, Jvm, Result};
use jvm_class_proto::JavaMethodProto;
use jvm_types::{ClassAccessFlags, MethodAccessFlags};

use crate::{RuntimeClassProto, RuntimeContext};

/// The heap a game sees. The host heap grows as needed, so the whole of a typical phone heap is reported free; games
/// use these numbers to pick how much to cache.
const HEAP_SIZE: i64 = 0x100000;

// class java.lang.Runtime
pub struct Runtime;

impl Runtime {
    pub fn as_proto() -> RuntimeClassProto {
        RuntimeClassProto {
            name: "java/lang/Runtime",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PRIVATE),
                JavaMethodProto::new("getRuntime", "()Ljava/lang/Runtime;", Self::get_runtime, MethodAccessFlags::STATIC),
                JavaMethodProto::new(
                    "totalMemory",
                    "()J",
                    Self::total_memory,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::NATIVE,
                ),
                JavaMethodProto::new(
                    "freeMemory",
                    "()J",
                    Self::free_memory,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::NATIVE,
                ),
                JavaMethodProto::new("gc", "()V", Self::gc, MethodAccessFlags::PUBLIC | MethodAccessFlags::NATIVE),
                JavaMethodProto::new("exit", "(I)V", Self::exit, MethodAccessFlags::PUBLIC),
            ],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL,
        }
    }

    async fn init(jvm: &Jvm, _: &mut RuntimeContext, this: ClassInstanceRef<Runtime>) -> Result<()> {
        tracing::debug!("java.lang.Runtime::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        Ok(())
    }

    async fn get_runtime(jvm: &Jvm, _: &mut RuntimeContext) -> Result<ClassInstanceRef<Self>> {
        tracing::debug!("java.lang.Runtime::getRuntime");

        let instance = jvm.new_class("java/lang/Runtime", "()V", []).await?;

        Ok(instance.into())
    }

    async fn total_memory(_: &Jvm, _: &mut RuntimeContext, this: ClassInstanceRef<Runtime>) -> Result<i64> {
        tracing::debug!("java.lang.Runtime::totalMemory({this:?})");

        Ok(HEAP_SIZE)
    }

    async fn free_memory(_: &Jvm, _: &mut RuntimeContext, this: ClassInstanceRef<Runtime>) -> Result<i64> {
        tracing::debug!("java.lang.Runtime::freeMemory({this:?})");

        Ok(HEAP_SIZE)
    }

    async fn gc(jvm: &Jvm, _: &mut RuntimeContext, this: ClassInstanceRef<Runtime>) -> Result<()> {
        tracing::debug!("java.lang.Runtime::gc({this:?})");

        if let Err(error) = jvm.collect_garbage() {
            return Err(jvm
                .exception("java/lang/InternalError", &format!("Garbage collection failed: {error:#}"))
                .await);
        }

        Ok(())
    }

    async fn exit(_: &Jvm, context: &mut RuntimeContext, _: ClassInstanceRef<Self>, status: i32) -> Result<()> {
        tracing::debug!("java.lang.Runtime::exit({status})");

        context.exit(status);
        Ok(())
    }
}

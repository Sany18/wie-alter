#![no_std]
extern crate alloc;

pub mod classes;

use wie_jvm_support::WieJavaClassProto;

pub fn get_protos() -> [WieJavaClassProto; 6] {
    [
        classes::com::nokia::mid::sound::Sound::as_proto(),
        classes::com::nokia::mid::ui::DeviceControl::as_proto(),
        classes::com::nokia::mid::ui::DirectGraphics::as_proto(),
        classes::com::nokia::mid::ui::DirectGraphicsImpl::as_proto(),
        classes::com::nokia::mid::ui::DirectUtils::as_proto(),
        classes::com::nokia::mid::ui::FullCanvas::as_proto(),
    ]
}

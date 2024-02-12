#![cfg(target_vendor = "apple")]
#![feature(type_alias_impl_trait)]

use objc2::__macro_helpers::MsgSend;
use objc2::Message;
use objc2::rc::Id;

mod buffer;
mod draw_quad;
mod error;
mod filter_chain;
mod filter_pass;
mod graphics_pipeline;
mod luts;
mod options;
mod samplers;
mod texture;

/// Need this because a bunch of non-mutable MTL structs aren't cloneable
pub(crate) unsafe fn clone_unsafe<T: Message>(id: &Id<T>) -> Id<T> {
    // SAFETY:
    // - The object is known to not be mutable due to the `IsIdCloneable`
    //   bound. Additionally, since the object is already an `Id`, types
    //   like `NSObject` and `NSString` that have a mutable subclass is
    //   also allowed (since even if the object is originally an
    //   `Id<NSMutableString>`, by converting it into `Id<NSObject>` or
    //   `Id<NSString>` that fact is wholly forgotten, and the object
    //   cannot ever be mutated again).
    // - The pointer is valid.
    let obj = unsafe { Id::retain(id.into_raw_receiver() as *mut T) };
    // SAFETY: `objc_retain` always returns the same object pointer, and
    // the pointer is guaranteed non-null.
    unsafe { obj.unwrap_unchecked() }
}
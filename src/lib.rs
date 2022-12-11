#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![cfg_attr(
    doc,
    warn(
        rustdoc::bare_urls,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_codeblock_attributes,
        rustdoc::invalid_rust_codeblocks,
        rustdoc::missing_crate_level_docs,
    )
)]
#![cfg_attr(nightly, feature(doc_cfg))]
#![cfg_attr(nightly, doc(cfg_hide(doc)))]

pub mod check;
pub mod com;
mod errors;
pub use errors::*;

/// Invokes a Win32 function with the provided argument and checks the return
/// value for success, or creates a crate error with context.
///
/// The supported values for check are:
/// - nonzero_isize
/// - nonzero_u16
/// - last_err
/// - hwnd
/// - bool
/// - res
///
/// ### Usage
///
/// ```
/// use ::call32::call;
/// use ::windows::Win32::System::LibraryLoader::GetModuleHandleA;
///
/// let _module = call!(res; GetModuleHandleA(None)).unwrap();
/// ```
#[macro_export]
macro_rules! call {
    ($check:expr ; $fn:ident ( $( $param:expr),* ) ) => {
        ::paste::paste! {
            $crate::check:: [< check_ $check >] (
                || unsafe { [<$fn>]( $( $param, )* ) } ,
                ::std::stringify!([<$fn>])
            )
        }
    }
}

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

pub mod com;
mod errors;
pub mod mappings;
pub use errors::*;

/// Invokes a Win32 function with the provided argument and maps the return
/// value into a Result with additional context.
///
/// The supported values for mapping are:
/// - map_nonzero_isize
/// - map_nonzero_u16
/// - map_last_err
/// - map_hwnd
/// - map_bool
/// - map_result
///
/// ### Usage
///
/// ```
/// use ::call32::call;
/// use ::windows::Win32::System::LibraryLoader::GetModuleHandleA;
///
/// let _module = call!(map_result; GetModuleHandleA(None)).unwrap();
/// ```
#[macro_export]
macro_rules! call {
    ($mapping:expr ; $fn:ident ( $( $param:expr),* ) ) => {
        ::paste::paste! {
            $crate::mappings:: [< $mapping >] (
                || unsafe { [<$fn>]( $( $param, )* ) } ,
                ::std::stringify!([<$fn>])
            )
        }
    }
}

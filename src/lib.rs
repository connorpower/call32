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

/// Macro to simplify calling a Win32 functions. Map the return value into a
/// `Result` with additional context in the case of an error.
///
/// # Syntax
///
/// ```text
/// call!( MAPPING; API_FUNCTION_CALL(ARGS...))
/// ```
///
/// Where `MAPPING` is one of the supported mappings below and
/// `API_FUNCTION_CALL` is a literal Win32 API function call complete with
/// arguments, if any (e.g. `GetModuleHandleA()`).
///
/// # Mappings
///
/// The supported values for mapping are:
///
/// * [`map_nonzero_u8`][]
/// * [`map_nonzero_u16`][]
/// * [`map_nonzero_u32`][]
/// * [`map_nonzero_u64`][]
/// * [`map_nonzero_usize`][]
/// * [`map_nonzero_i8`][]
/// * [`map_nonzero_i16`][]
/// * [`map_nonzero_i32`][]
/// * [`map_nonzero_i64`][]
/// * [`map_nonzero_isize`][]
/// * [`map_last_err`][]
/// * [`map_ptr`][]
/// * [`map_bool`][]
/// * [`map_result`][]
///
/// # Usage
///
/// ```
/// use ::call32::call;
/// use ::windows::Win32::System::LibraryLoader::GetModuleHandleA;
///
/// let _module = call!(map_result; GetModuleHandleA(None)).unwrap();
/// ```
///
/// [`map_nonzero_u8`]:  crate::mappings::map_nonzero_u8
/// [`map_nonzero_u16`]: crate::mappings::map_nonzero_u16
/// [`map_nonzero_u32`]: crate::mappings::map_nonzero_u32
/// [`map_nonzero_u64`]: crate::mappings::map_nonzero_u64
/// [`map_nonzero_usize`]: crate::mappings::map_nonzero_usize
/// [`map_nonzero_i8`]:  crate::mappings::map_nonzero_i8
/// [`map_nonzero_i16`]: crate::mappings::map_nonzero_i16
/// [`map_nonzero_i32`]: crate::mappings::map_nonzero_i32
/// [`map_nonzero_i64`]: crate::mappings::map_nonzero_i64
/// [`map_nonzero_isize`]: crate::mappings::map_nonzero_isize
/// [`map_last_err`]: crate::mappings::map_last_err
/// [`map_ptr`]: crate::mappings::map_ptr
/// [`map_bool`]: crate::mappings::map_bool
/// [`map_result`]: crate::mappings::map_result
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

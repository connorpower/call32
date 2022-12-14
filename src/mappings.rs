//! Mapping functions convert Win32 API return types into `Result`s with
//! additional context in the case of an error.

use crate::{Error, Result};
use ::std::num::{
    NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU16, NonZeroU32,
    NonZeroU64, NonZeroU8, NonZeroUsize,
};
use ::windows::{
    core::{Error as Win32Error, Result as Win32Result, PCSTR, PCWSTR, PSTR, PWSTR},
    Win32::Foundation::{GetLastError, SetLastError, BOOL, HWND, WIN32_ERROR},
};

macro_rules! impl_nonzero {
    ($num:ty => $nonzero:ty) => {
        ::paste::paste! {
            #[doc = "Calls Win32 API which defines success by a non-zero"      ]
            #[doc = "[`" $num "`] return type."                                ]
            #[doc = ""                                                         ]
            #[doc = "Returns a guaranteed [`" $nonzero "`][] integer or"       ]
            #[doc = "otherwise maps the result of `F` to a crate error"        ]
            #[doc = "complete with system error message context. This function"]
            #[doc = "be used with [`call!`][] by specifying the appropriate"   ]
            #[doc = "mapping name, e.g.: `call!(map_nonzero_" $num "; ...)`"   ]
            #[doc = ""                                                         ]
            #[doc = "# Parameters"                                             ]
            #[doc = ""                                                         ]
            #[doc = " - `function`: A closure to run which returns a"          ]
            #[doc = "   [`" $num "`] result. This is typically a Win32 API"    ]
            #[doc = "   function within an unsafe block."                      ]
            #[doc = " - `source_hint`: A debugging hint to help identify the"  ]
            #[doc = "   source of an error should one occur. This is typically"]
            #[doc = "   just the Win32 function name as a string. The macro"   ]
            #[doc = "   [`call!`] automatically extracts the function name"    ]
            #[doc = "   from the macro arguments to use as the value for this" ]
            #[doc = "   source hint."                                          ]
            #[doc = ""                                                         ]
            #[doc = "# Usage"                                                  ]
            #[doc = ""                                                         ]
            #[doc = "```rust"                                                  ]
            #[doc = "use ::call32::{call, mappings::map_nonzero_" $num "};"     ]
            #[doc = "# unsafe fn Win32APICall() -> " $num " {"                 ]
            #[doc = "#     1 as " $num ""                                      ]
            #[doc = "# }"                                                      ]
            #[doc = ""                                                         ]
            #[doc = "// Use as a standalone function:"                         ]
            #[doc = "let result = map_nonzero_" $num "(|| unsafe {"            ]
            #[doc = "    Win32APICall()"                                       ]
            #[doc = "}, \"Win32APICall\");"                                    ]
            #[doc = ""                                                         ]
            #[doc = "assert!(result.is_ok());"                                 ]
            #[doc = ""                                                         ]
            #[doc = ""                                                         ]
            #[doc = "// Or, more commonly, use with the `call!` macro:"        ]
            #[doc = "let result = call!(map_nonzero_" $num "; Win32APICall());"]
            #[doc = ""                                                         ]
            #[doc = "assert!(result.is_ok());"                                 ]
            #[doc = "```"                                                      ]
            #[doc = ""                                                         ]
            #[doc = "[`" $num "`]: " $num ""                                   ]
            #[doc = "[`" $nonzero "`]: std::num::" $nonzero ""                 ]
            #[doc = "[`call!`]: crate::call"                                   ]
            pub fn [<map_nonzero_ $num>]<F>(function: F, source_hint: &'static str) -> Result<$nonzero>
            where
                F: FnOnce() -> $num,
            {
                <$nonzero>::new(function()).ok_or_else(||
                    Error::Unexpected {
                        source_hint,
                        underlying_error: Win32Error::from_win32(),
                    }
                )
            }
        }
    };
}

impl_nonzero!(u8 => NonZeroU8);
impl_nonzero!(u16 => NonZeroU16);
impl_nonzero!(u32 => NonZeroU32);
impl_nonzero!(u64 => NonZeroU64);
impl_nonzero!(usize => NonZeroUsize);
impl_nonzero!(i8 => NonZeroI8);
impl_nonzero!(i16 => NonZeroI16);
impl_nonzero!(i32 => NonZeroI32);
impl_nonzero!(i64 => NonZeroI64);
impl_nonzero!(isize => NonZeroIsize);

/// Calls Win32 API which defines success by directly setting the thread's
/// last-error value.
///
/// The return type of the provided function is mapped to a `Result`,
/// which will be `Ok` if the thread's last-error code was still `0` after the
/// function call, and `Err` if a non-zero error code was found. The last-error
/// code is usually retrieved via [`GetLastError`]. As is best practice for
/// these types of function, any current error flag is cleared by calling
/// [`SetLastError`] with `0` immediately before calling the function.
///
/// This mapping can be used with the [`call!`][] macro by specifying the
/// appropriate mapping name, e.g.: `call!(map_last_err; ...)`.
///
/// # Parameters
///
/// - `function`: A closure to run. This is typically an Win32 API function
///   within an unsafe block.
/// - `source_hint`: A debugging hint to help identify the source of an error
///   should one occur. This is typically just the Win32 function name as a
///   string. The macro [`call!`] automatically extracts the function name from
///   the macro arguments to use as the value for this source hint.
///
/// # Usage
///
/// ```rust
/// use ::call32::{call, mappings::map_last_err};
/// # unsafe fn Win32APICall() -> isize { 0 }
///
/// // Use as a standalone function:
/// let result = map_last_err(|| unsafe {
///     Win32APICall()
/// }, "Win32APICall");
///
/// assert!(result.is_ok());
///
///
/// // Or, more commonly, use with the `call!` macro:
/// let result = call!(map_last_err; Win32APICall());
///
/// assert!(result.is_ok());
/// ```
///
/// [`call!`]: crate::call
/// [`Result`]: crate::Result
/// [`GetLastError`]: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
/// [`SetLastError`]: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-setlasterror
pub fn map_last_err<F, R>(function: F, source_hint: &'static str) -> Result<R>
where
    F: FnOnce() -> R,
{
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let result = function();

    unsafe { GetLastError() }
        .ok()
        .map(|_| result)
        .map_err(|underlying_error| Error::Unexpected {
            source_hint,
            underlying_error,
        })
}

/// Calls Win32 API which defines success by returning a
/// [`::windows::Win32::Foundation::BOOL`] value.
///
/// The bool result is mapped into a `Result<(), Error>` for ergonomic error
/// handling.  If an error is detected, additional context will be automatically
/// retrieved from the system by calling [`GetLastError`] and associating the
/// context with the returned `Err`.
///
/// This mapping can be used with the [`call!`][] macro by specifying the
/// appropriate mapping name, e.g.: `call!(map_bool; ...)`.
///
/// # Parameters
///
/// - `function`: A closure to run. This is typically a Win32 API function
///   within an unsafe block.
/// - `source_hint`: A debugging hint to help identify the source of an error
///   should one occur. This is typically just the Win32 function name as a
///   string. The macro [`call!`] automatically extracts the function name from
///   the macro arguments to use as the value for this source hint.
///
/// # Usage
///
/// ```rust
/// # use ::windows::Win32::Foundation::BOOL;
/// use ::call32::{call, mappings::map_bool};
/// # unsafe fn Win32APICall() -> BOOL { BOOL(1) }
///
/// // Use as a standalone function:
/// let result = map_bool(|| unsafe {
///     Win32APICall()
/// }, "Win32APICall");
///
/// assert!(result.is_ok());
///
///
/// // Or, more commonly, use with the `call!` macro:
/// let result = call!(map_bool; Win32APICall());
///
/// assert!(result.is_ok());
/// ```
///
/// [`::windows::Win32::Foundation::BOOL`]: windows::Win32::Foundation::BOOL
/// [`call!`]: crate::call
/// [`GetLastError`]: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
pub fn map_bool<F>(function: F, source_hint: &'static str) -> Result<()>
where
    F: FnOnce() -> BOOL,
{
    function()
        .ok()
        .map_err(|underlying_error| Error::Unexpected {
            source_hint,
            underlying_error,
        })
}

/// Calls Win32 API which returns a [`::windows::core::Result<T>`] type.
///
/// If an error is detected, additional context will be automatically retrieved
/// from the system by calling [`GetLastError`] and associating the context with
/// the returned `Err`.
///
/// This mapping can be used with the [`call!`][] macro by specifying the
/// appropriate mapping name, e.g.: `call!(map_result; ...)`.
///
/// # Parameters
///
/// - `function`: A closure to run. This is typically a Win32 API function
///   within an unsafe block.
/// - `source_hint`: A debugging hint to help identify the source of an error
///   should one occur. This is typically just the Win32 function name as a
///   string. The macro [`call!`] automatically extracts the function name from
///   the macro arguments to use as the value for this source hint.
///
/// # Usage
///
/// ```rust
/// # use ::windows::core::Result;
/// use ::call32::{call, mappings::map_result};
/// # unsafe fn Win32APICall() -> Result<()> { Ok(()) }
///
/// // Use as a standalone function:
/// let result = map_result(|| unsafe {
///     Win32APICall()
/// }, "Win32APICall");
///
/// assert!(result.is_ok());
///
///
/// // Or, more commonly, use with the `call!` macro:
/// let result = call!(map_result; Win32APICall());
///
/// assert!(result.is_ok());
/// ```
///
/// [`::windows::core::Result<T>`]: windows::core::Result
/// [`call!`]: crate::call
/// [`GetLastError`]: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
pub fn map_result<F, R>(function: F, source_hint: &'static str) -> Result<R>
where
    F: FnOnce() -> Win32Result<R>,
{
    function().map_err(|underlying_error| Error::Unexpected {
        source_hint,
        underlying_error,
    })
}

/// Calls Win32 API which defines success with a non-zero [`Win32Ptr`] return
/// types.
///
/// The Win32 API has many types that are conceptually pointers, but the Rust
/// language projection in the [`::windows`][] crate unfortunately has no common
/// method across all Win32 pointers to determine if they're valid. To that end,
/// [`::call32`][] defines the [`Win32Ptr`][] trait and provides an
/// implementation for all the common types:
///
/// * [`HWND`][]: The window handle for a desktop window.
/// * [`PSTR`][]: A pointer to a null-terminated string of 8-bit Windows (ANSI)
///   characters.
/// * [`PWSTR`][]: A pointer to a null-terminated string of 16-bit Unicode
///   characters.
/// * [`PCSTR`][]: A pointer to a constant null-terminated string of 8-bit
///   Windows (ANSI) characters.
/// * [`PCWSTR`][]: A pointer to a constant null-terminated string of 16-bit
///   Unicode characters.
///
/// If an error is detected, additional context will be automatically retrieved
/// from the system by calling [`GetLastError`] and associating the context with
/// the returned `Err`.
///
/// This mapping can be used with the [`call!`][] macro by specifying the
/// appropriate mapping name, e.g.: `call!(map_ptr; ...)`.
///
/// # Parameters
///
/// - `function`: A closure to run. This is typically a Win32 API function
///   within an unsafe block.
/// - `source_hint`: A debugging hint to help identify the source of an error
///   should one occur. This is typically just the Win32 function name as a
///   string. The macro [`call!`][] automatically extracts the function name
///   from the macro arguments to use as the value for this source hint.
///
/// # Usage
///
/// ```rust
/// # use ::windows::{w, core::PCWSTR};
/// use ::call32::{call, mappings::map_ptr};
/// # unsafe fn Win32APICall() -> PCWSTR { w!("Hello, Redmond.").into() }
///
/// // Use as a standalone function:
/// let result = map_ptr(|| unsafe {
///     Win32APICall()
/// }, "Win32APICall");
///
/// assert!(result.is_ok());
///
///
/// // Or, more commonly, use with the `call!` macro:
/// let result = call!(map_ptr; Win32APICall());
///
/// assert!(result.is_ok());
/// ```
///
/// [`Win32Ptr`]: Win32Ptr
/// [`::call32`]: crate
/// [`::windows`]: windows
/// [`call!`]: crate::call
/// [`GetLastError`]: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
/// [`HWND`]: windows::Win32::Foundation::HWND
/// [`PSTR`]: windows::core::PSTR
/// [`PWSTR`]: windows::core::PSTR
/// [`PCSTR`]: windows::core::PSTR
/// [`PCWSTR`]: windows::core::PSTR
pub fn map_ptr<F, R>(function: F, source_hint: &'static str) -> Result<R>
where
    F: FnOnce() -> R,
    R: Win32Ptr,
{
    let ptr = function();

    if ptr.is_null() {
        Err(Error::Unexpected {
            source_hint,
            underlying_error: Win32Error::from_win32(),
        })
    } else {
        Ok(ptr)
    }
}

/// A blanket trait implemented for Win32 pointer types to provide a blanket
/// [`is_null()`] method for checking pointer validity.
///
/// The Win32 API has many types that are conceptually pointers, but the Rust
/// language projection in the [`::windows`][] crate unfortunately has no common
/// method across all Win32 pointers to determine if they're valid. To that end,
/// [`::call32`][] defines the [`Win32Ptr`][] trait and provides an
/// implementation for all the common types:
///
/// * [`HWND`][]: The window handle for a desktop window.
/// * [`PSTR`][]: A pointer to a null-terminated string of 8-bit Windows (ANSI)
///   characters.
/// * [`PWSTR`][]: A pointer to a null-terminated string of 16-bit Unicode
///   characters.
/// * [`PCSTR`][]: A pointer to a constant null-terminated string of 8-bit
///   Windows (ANSI) characters.
/// * [`PCWSTR`][]: A pointer to a constant null-terminated string of 16-bit
///   Unicode characters.
///
/// [`::call32`]: crate
/// [`::windows`]: windows
/// [`HWND`]: windows::Win32::Foundation::HWND
/// [`PSTR`]: windows::core::PSTR
/// [`PWSTR`]: windows::core::PSTR
/// [`PCSTR`]: windows::core::PSTR
/// [`PCWSTR`]: windows::core::PSTR
/// [`is_null()`]: Win32Ptr::is_null
pub trait Win32Ptr {
    /// Predicate method which indicates whether the pointer should be
    /// considered a null pointer.
    fn is_null(&self) -> bool;
}

macro_rules! impl_win32_ptr {
    ($type:ty; use_inner_is_null) => {
        impl Win32Ptr for $type {
            fn is_null(&self) -> bool {
                self.0.is_null()
            }
        }
    };
    ($type:ty; use_int_val) => {
        impl Win32Ptr for $type {
            fn is_null(&self) -> bool {
                self.0 == 0 as _
            }
        }
    };
}

impl_win32_ptr!(HWND; use_int_val);
impl_win32_ptr!(PSTR; use_inner_is_null);
impl_win32_ptr!(PWSTR; use_inner_is_null);
impl_win32_ptr!(PCSTR; use_inner_is_null);
impl_win32_ptr!(PCWSTR; use_inner_is_null);

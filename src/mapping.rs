//! A collection of functions which invoke Win32 API and map their return
//! values to a `Result` type with additional context in the case of an
//! unsuccessful call.

use crate::{get_last_err, Error, Result};
use ::std::num::{
    NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU16, NonZeroU32,
    NonZeroU64, NonZeroU8, NonZeroUsize,
};
use ::windows::{
    core::{Result as Win32Result, PCSTR, PCWSTR, PSTR, PWSTR},
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
            #[doc = "mapping name, e.g.: `call!(nonzero_" $num "; ...)`"       ]
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
            #[doc = "use ::call32::{call, mapping::map_nonzero_" $num "};"     ]
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
            #[doc = "let result = call!(nonzero_" $num "; Win32APICall());"    ]
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
                <$nonzero>::new(function()).ok_or_else(|| get_last_err(source_hint))
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
/// The return type of the provided function is mapped to a crate [`Result`],
/// which will be `Ok` if the thread's last-error code was still `0` after the
/// function call, and `Err` if a non-zero error code was found. The last-error
/// code is usually retrieved via [`GetLastError`]. As is best practice for
/// these types of function, any current error flag is cleared by calling
/// [`SetLastError`] with `0` immediately before calling the function.
///
/// This mapping can be used with the [`call!`][] macro by specifying the
/// appropriate mapping name, e.g.: `call!(last_err; ...)`.
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
/// use ::call32::{call, mapping::map_last_err};
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
/// let result = call!(last_err; Win32APICall());
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
    let res = function();
    let last_err = unsafe { GetLastError() };

    if last_err.is_ok() {
        Ok(res)
    } else {
        Err(Error::Unexpected {
            function: source_hint,
            context: last_err.to_hresult().into(),
        })
    }
}

/// Calls Win32 API which defines success by returning a [`BOOL`] value from the
/// windows crate.
///
/// The bool result is mapped into a `Result<(), Error>` for ergonomic error
/// handling.  If an error is detected, additional context will be automatically
/// retrieved from the system by calling [`GetLastError`] and associating the
/// context with the returned `Err`.
///
/// This mapping can be used with the [`call!`][] macro by specifying the
/// appropriate mapping name, e.g.: `call!(bool; ...)`.
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
/// use ::call32::{call, mapping::map_bool};
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
/// let result = call!(bool; Win32APICall());
///
/// assert!(result.is_ok());
/// ```
///
/// [`BOOL`]: windows::Win32::Foundation::BOOL
/// [`call!`]: crate::call
/// [`GetLastError`]: https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
pub fn map_bool<F>(function: F, source_hint: &'static str) -> Result<()>
where
    F: FnOnce() -> BOOL,
{
    function().ok().map_err(|e| Error::Unexpected {
        function: source_hint,
        context: e,
    })
}

/// Invokes a Win32 API which defines success by Win32 results. Maps
/// the result of `F` to an error on failure.
///
/// Can be used with [`call!`](crate::call) by specifying `res` as the type of
/// mapping, e.g.: `call!(res; ...)`
/// TODO: Tidy
pub fn map_res<F, V>(f: F, f_name: &'static str) -> Result<V>
where
    F: FnOnce() -> Win32Result<V>,
{
    f().map_err(|context| Error::Unexpected {
        function: f_name,
        context,
    })
}

/// Invokes a Win32 API which defines success by non-zero pointers. Maps
/// the result of `F` to an error on failure.
///
/// Can be used with [`call!`](crate::call) by specifying `ptr` as the type of
/// mapping, e.g.: `call!(ptr; ...)`
/// TODO: Tidy
pub fn map_ptr<F, P>(f: F, f_name: &'static str) -> Result<P>
where
    F: FnOnce() -> P,
    P: Win32Pointer,
{
    let ptr = f();

    if ptr.is_null() {
        Err(get_last_err(f_name))
    } else {
        Ok(ptr)
    }
}

/// A common trait implemented for Win32 pointer types.
pub trait Win32Pointer {
    /// Predicate method which indicates whether the pointer should be
    /// considered a null pointer.
    fn is_null(&self) -> bool;
}

macro_rules! impl_win32_ptr {
    ($type:ty; wrapped_is_null) => {
        impl Win32Pointer for $type {
            fn is_null(&self) -> bool {
                self.0.is_null()
            }
        }
    };
    ($type:ty; int_val) => {
        impl Win32Pointer for $type {
            fn is_null(&self) -> bool {
                self.0 == 0 as _
            }
        }
    };
}

impl_win32_ptr!(HWND; int_val);
impl_win32_ptr!(PSTR; wrapped_is_null);
impl_win32_ptr!(PWSTR; wrapped_is_null);
impl_win32_ptr!(PCSTR; wrapped_is_null);
impl_win32_ptr!(PCWSTR; wrapped_is_null);

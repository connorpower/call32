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
            #[doc = " - `function`: an `FnOnce` closure to run which returns a"]
            #[doc = "   [`" $num "`] result. This is typically an unsafe Win32"]
            #[doc = "   API function within an unsafe block."                  ]
            #[doc = " - `source_hint`: a debugging hint to help identify the"  ]
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
            #[doc = "assert!(result.is_ok());"                                 ]
            #[doc = ""                                                         ]
            #[doc = "// Or, more commonly, use with the `call!` macro:"        ]
            #[doc = "let result = call!(nonzero_" $num "; Win32APICall());"   ]
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

/// Invokes a Win32 API which indicates failure by setting the last error code
/// and not by return type or output params. The last error is cleared
/// immediately before invoking the function.
///
/// Can be used with [`call!`](crate::call) by specifying `last_err` as the type
/// of mapping, e.g.: `call!(last_err; ...)`
pub fn map_last_err<F, R>(f: F, f_name: &'static str) -> Result<R>
where
    F: FnOnce() -> R,
{
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let res = f();
    let last_err = unsafe { GetLastError() };

    if last_err.is_ok() {
        Ok(res)
    } else {
        Err(Error::Unexpected {
            function: f_name,
            context: last_err.to_hresult().into(),
        })
    }
}

/// Invokes a Win32 API which defines success by bool return values. Maps the
/// result of `F` to an error on failure.
///
/// Can be used with [`call!`](crate::call) by specifying `bool` as the type of
/// mapping, e.g.: `call!(bool; ...)`
pub fn map_bool<F>(f: F, f_name: &'static str) -> Result<()>
where
    F: FnOnce() -> BOOL,
{
    f().ok().map_err(|_| get_last_err(f_name))
}

/// Invokes a Win32 API which defines success by Win32 results. Maps
/// the result of `F` to an error on failure.
///
/// Can be used with [`call!`](crate::call) by specifying `res` as the type of
/// mapping, e.g.: `call!(res; ...)`
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

//! Crate-specific error and result types, plus common conversions.

use ::windows::core::{Error as Win32Error, HRESULT};

/// Result type returned by functions that call into Win32 API.
pub(crate) type Result<T> = ::std::result::Result<T, Error>;

/// Error type for functions that call into Win32 API. The error attempts to
/// pro-actively capture as much context as possible (error codes, system error
/// message strings, etc).
#[derive(::thiserror::Error, Debug)]
pub enum Error {
    /// An unexpected error occurred and was not handled internally.
    #[error("{source_hint}: unexpected win32 error. Caused by: {underlying_error}")]
    Unexpected {
        /// A hint as to to the error source. Often just the function name, but
        /// it could be anything.  [`call!`](crate::call) will automatically
        /// populate this field with the function name it is calling.
        source_hint: &'static str,
        /// The underlying Win32 [`Error`] which provides access to the error
        /// code and message.
        ///
        /// [`Error`]: windows ::core::Win32Error;
        underlying_error: Win32Error,
    },
}

impl Error {
    /// Returns the underlying Win32 error code.
    pub fn code(&self) -> HRESULT {
        match self {
            Self::Unexpected {
                underlying_error, ..
            } => underlying_error.code(),
        }
    }
}

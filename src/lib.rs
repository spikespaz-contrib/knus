#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

mod containers;
mod convert;
mod convert_ast;
mod grammar;
mod wrappers;

pub mod ast;
pub mod decode;
pub mod errors;
pub mod span;
pub mod traits;

#[cfg(feature = "derive")]
pub use knus_derive::{Decode, DecodeScalar};

pub use errors::Error;
pub use traits::{Decode, DecodeChildren, DecodeScalar};
pub use wrappers::{parse, parse_ast, parse_with_context};

#![cfg_attr(target_arch = "wasm32", no_std)]
extern crate alloc;

#[cfg(feature = "wasmer_traits")]
mod wasmer_traits;

pub mod cgo;
mod dicts;
mod types;

pub use dicts::Dictionary;
pub use types::{BrotliStatus, DEFAULT_WINDOW_SIZE};

#[cfg(not(feature = "rust_brotli"))]
mod native;
#[cfg(feature = "rust_brotli")]
mod pure;

#[cfg(not(feature = "rust_brotli"))]
pub use native::*;

#[cfg(feature = "rust_brotli")]
pub use pure::*;

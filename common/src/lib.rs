//! Crate containing client-server interoperability structs and functions, as well as some utils
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs)]

#[macro_use]
extern crate constcat;

/// Base endpoint for API requests
pub const API_BASE: &str = "/api";

pub mod libraries;
pub mod media;

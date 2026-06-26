//! Cellar SQL support: parsing, formatting, and dialect awareness built on
//! `sqlparser-rs`.
//!
//! Today this crate provides named/positional parameter detection and
//! binding preparation; formatting and lint helpers will land here too.

pub mod params;

pub use params::{detect_parameters, order_values, prepare, ParamError, PreparedStatement};

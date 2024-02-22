//! # checkr
//!
//! The checkr crate is the core analysis code for the checkr project. It
//! contains code for parsing the Guarded Command Language variant, running
//! analysis on them, infrastructure for generating random programs and inputs,
//! and a way to communicate with external implementations of the same analysis.
//!
//! ## Structuring analysis in environments
//!
//! Each analysis must implement the [`Environment`] trait. This defines the
//! input and output format for each analysis or environment. The input and
//! output are required to implement the serde [`Serialize`](serde::Serialize)
//! and [`Deserialize`](serde::Deserialize) traits, such that they can
//! communicate with the external world.
//!
//! ## Interacting with external implementations
//!
//! The primary goal of this project is to aid implementers of the same analysis
//! with checking their work continuously during development. To do so the
//! [`Driver`] struct together with the aforementioned [`Environment`] trait
//! provides an interface to interact with external code-bases in a generic way.
//!
//! ## Generating sample programs
//!
//! The [`generation`] module defines a trait for generating structures given a
//! source of randomness. It also implements this trait for all of the GCL
//! constructs, which allows programs to be generated in a programmatic way.
//! Similarly, the inputs of [`Environment`] implementations must too implement
//! [`Generate`](generation::Generate).

use std::borrow::Cow;

pub use miette;

pub mod egg;
pub mod generation;
pub mod interpreter;
pub mod pv;
pub mod security;

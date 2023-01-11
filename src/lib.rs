#![feature(box_patterns, box_syntax)]

use rand::prelude::*;

use crate::ast::Commands;

pub mod analysis;
pub mod ast;
pub mod environment;
pub mod fmt;
pub mod generation;
pub mod interpreter;
pub mod parse;
pub mod pg;
pub mod security;

pub fn generate_program(fuel: Option<u32>, seed: Option<u64>) -> (Commands, SmallRng) {
    let seed = match seed {
        Some(seed) => seed,
        None => rand::random(),
    };
    let mut rng = SmallRng::seed_from_u64(seed);

    let fuel = fuel.unwrap_or(10);

    let mut cx = generation::Context::new(fuel, &mut rng);

    (Commands(cx.many(5, 10, &mut rng)), rng)
}

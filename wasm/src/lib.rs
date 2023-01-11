use std::collections::HashMap;

use itertools::Itertools;
use verification_lawyer::environment::{Application, SecurityAnalysis, StepWise};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn hello_wasm(name: &str) -> String {
    format!("Hello, {name} from Rust! :) 🦀")
}

#[wasm_bindgen]
pub fn generate_program() -> String {
    verification_lawyer::generate_program(None, None)
        .0
        .to_string()
}

#[wasm_bindgen]
pub struct WebApplication {
    app: Application,
}

#[wasm_bindgen]
impl WebApplication {
    pub fn new() -> Self {
        let mut app = Application::new();
        app.add_env(StepWise).add_env(SecurityAnalysis);
        WebApplication { app }
    }

    pub fn list_envs(&self) -> String {
        self.app.envs.iter().map(|e| e.name()).join(",")
    }
}

impl Default for WebApplication {
    fn default() -> Self {
        Self::new()
    }
}

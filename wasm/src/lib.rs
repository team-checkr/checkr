use itertools::Itertools;
use serde::{Deserialize, Serialize};
use verification_lawyer::environment::{Application, SecurityAnalysis, SignEnv, StepWise};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn init_hook() {
    console_error_panic_hook::set_once();
}

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
        Self::default()
    }

    pub fn list_envs(&self) -> String {
        self.app.envs.iter().map(|e| e.name()).join(",")
    }

    pub fn generate(&self) -> String {
        let (cmds, mut rng) = verification_lawyer::generate_program(None, None);

        let g = Generation {
            program: cmds.to_string(),
            envs: self
                .app
                .envs
                .iter()
                .map(|env| (env.name(), env.gen_input(&cmds, &mut rng)))
                .collect(),
        };

        serde_json::to_string(&g).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Generation {
    program: String,
    envs: Vec<(String, (serde_json::Value, serde_json::Value))>,
}

impl Default for WebApplication {
    fn default() -> Self {
        let mut app = Application::new();
        app.add_env(StepWise)
            .add_env(SecurityAnalysis)
            .add_env(SignEnv);
        WebApplication { app }
    }
}

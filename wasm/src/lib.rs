use itertools::Itertools;
use serde::{Deserialize, Serialize};
use verification_lawyer::{
    env::{
        graph::{GraphEnv, GraphEnvInput},
        pv::ProgramVerificationEnv,
        AnyEnvironment, Application, Environment, Sample, SecurityEnv, SignEnv, StepWiseEnv,
    },
    pg::{Determinism, ProgramGraph},
    GeneratedProgram,
};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn init_hook() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

#[wasm_bindgen]
pub fn generate_program() -> String {
    verification_lawyer::generate_program(None, None)
        .cmds
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
        let GeneratedProgram { cmds, mut rng, .. } =
            verification_lawyer::generate_program(None, None);

        let g = Generation {
            program: cmds.to_string(),
            dot: ProgramGraph::new(Determinism::NonDeterministic, &cmds).dot(),
            envs: self
                .app
                .envs
                .iter()
                .map(|env| {
                    let sample = env.gen_sample(&cmds, &mut rng);
                    Env {
                        name: env.name(),
                        sample,
                    }
                })
                .collect(),
        };

        serde_json::to_string(&g).unwrap()
    }

    pub fn generate_program(&self) -> String {
        verification_lawyer::generate_program(None, None)
            .cmds
            .to_string()
    }

    pub fn dot(&self, deterministic: bool, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        GraphEnv
            .run(
                &cmds,
                &GraphEnvInput {
                    determinism: if deterministic {
                        Determinism::Deterministic
                    } else {
                        Determinism::NonDeterministic
                    },
                },
            )
            .dot
    }
    pub fn security(&self, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        let mut rng = verification_lawyer::generate_program(None, None).rng;
        let sample = SecurityEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&sample).unwrap()
    }
    pub fn step_wise(&self, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        let mut rng = verification_lawyer::generate_program(None, None).rng;
        let sample = StepWiseEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&sample).unwrap()
    }
    pub fn sign(&self, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        let mut rng = verification_lawyer::generate_program(None, None).rng;
        let sample = SignEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&sample).unwrap()
    }
    pub fn pv(&self, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        let mut rng = verification_lawyer::generate_program(None, None).rng;
        let sample = ProgramVerificationEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&sample).unwrap()
    }
}

#[typeshare::typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Env {
    name: String,
    sample: Sample,
}
#[typeshare::typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Generation {
    program: String,
    dot: String,
    envs: Vec<Env>,
}

impl Default for WebApplication {
    fn default() -> Self {
        let mut app = Application::new();
        app.add_env(StepWiseEnv)
            .add_env(SecurityEnv)
            .add_env(SignEnv);
        WebApplication { app }
    }
}

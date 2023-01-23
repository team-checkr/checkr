use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smtlib::{SatResultWithModel, Sort};
use tracing::{info, warn};
use verification_lawyer::{
    env::{
        graph::{GraphEnv, GraphEnvInput},
        pv::ProgramVerificationEnv,
        AnyEnvironment, Application, Environment, SecurityEnv, SignEnv, StepWiseEnv,
    },
    pg::{Determinism, ProgramGraph},
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
        let (cmds, _, _, mut rng) = verification_lawyer::generate_program(None, None);

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
                        input_json: sample.0.to_string(),
                        input_markdown: sample.1,
                        output_markdown: sample.2,
                    }
                })
                .collect(),
        };

        serde_json::to_string(&g).unwrap()
    }

    pub fn generate_program(&self) -> String {
        let (cmds, _, _, _) = verification_lawyer::generate_program(None, None);
        cmds.to_string()
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
        let (_, _, _, mut rng) = verification_lawyer::generate_program(None, None);
        let sample = SecurityEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&[sample.0.to_string(), sample.1, sample.2]).unwrap()
    }
    pub fn step_wise(&self, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        let (_, _, _, mut rng) = verification_lawyer::generate_program(None, None);
        let sample = StepWiseEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&[sample.0.to_string(), sample.1, sample.2]).unwrap()
    }
    pub fn sign(&self, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        let (_, _, _, mut rng) = verification_lawyer::generate_program(None, None);
        let sample = SignEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&[sample.0.to_string(), sample.1, sample.2]).unwrap()
    }
    pub fn pv(&self, src: &str) -> String {
        let Ok(cmds) = verification_lawyer::parse::parse_commands(src) else {
            return "Parse error".to_string()
        };
        let (_, _, _, mut rng) = verification_lawyer::generate_program(None, None);
        let sample = ProgramVerificationEnv.gen_sample(&cmds, &mut rng);
        serde_json::to_string(&[sample.0.to_string(), sample.1, sample.2]).unwrap()
    }
}

#[typeshare::typeshare]
struct Sample {
    input_json: String,
    input_md: String,
    output_md: String,
}

#[typeshare::typeshare]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Env {
    name: String,
    input_json: String,
    input_markdown: String,
    output_markdown: String,
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

#[wasm_bindgen]
pub struct WasmZ3 {
    ctx: String,
}

#[wasm_bindgen]
impl WasmZ3 {
    pub async fn new() -> Self {
        Self {
            ctx: init_context().await.as_string().unwrap(),
        }
    }
    pub async fn run(self) {
        use smtlib::AsyncSolver;
        let mut s = AsyncSolver::new(self).await.unwrap();
        let x = smtlib::Int::from_name("x");
        s.assert(x._eq(12)).await.unwrap();
        match s.check_sat_with_model().await.unwrap() {
            SatResultWithModel::Sat(m) => info!("{m}"),
            _ => warn!("No model produced!"),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl smtlib::backend::AsyncBackend for WasmZ3 {
    async fn exec(
        &mut self,
        cmd: &smtlib_lowlevel::ast::Command,
    ) -> Result<String, smtlib_lowlevel::Error> {
        let result = run(&self.ctx, &cmd.to_string()).await;
        result.as_string().ok_or_else(|| {
            smtlib_lowlevel::Error::IO(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "function did not return a string",
            ))
        })
    }
}

#[wasm_bindgen(module = "/z3-wrapper.js")]
extern "C" {
    async fn init_context() -> JsValue;
    async fn run(ctx: &str, cmd: &str) -> JsValue;
}

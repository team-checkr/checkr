#![allow(non_snake_case)]

mod def;
mod io;

pub use io::{Annotation, Error, Hash, Input, Meta, Output};
use rand::SeedableRng;

pub trait EnvExt: Env {
    const ANALYSIS: Analysis;

    fn generalize_input(input: &Self::Input) -> Input;
    fn generalize_output(output: &Self::Output) -> Output;
}

define_shell!(
    ce_calculator::CalcEnv[Calculator, "Calculator"],
    ce_compiler::CompilerEnv[Compiler, "Compiler"],
    ce_interpreter::InterpreterEnv[Interpreter, "Interpreter"],
    ce_bigcl::BiGCLEnv[BiGCL, "BiGCL"],
    ce_riscv::RiscVEnv[RiscV, "RISC-V"],
    ce_minimizer::MinimizerEnv[Minimizer, "DFA Minimizer"],
    ce_parser::ParserEnv[Parser, "Parser"],
    ce_security::SecurityEnv[Security, "Security"],
    ce_sign::SignEnv[Sign, "Sign Analysis"],
);

impl Analysis {
    pub fn gen_input_seeded(self, seed: Option<u64>) -> Input {
        let mut rng = match seed {
            Some(seed) => rand::rngs::SmallRng::seed_from_u64(seed),
            None => rand::rngs::SmallRng::from_os_rng(),
        };
        self.gen_input(&mut rng)
    }
}

impl Input {
    #[tracing::instrument(skip_all, fields(analysis = self.analysis().to_string()))]
    pub fn validate_output(
        &self,
        output: &Output,
    ) -> Result<(ValidationResult, Annotation), EnvError> {
        assert_eq!(self.analysis(), output.analysis());

        static VALIDATION: once_cell::sync::Lazy<
            dashmap::DashMap<
                (crate::io::Hash, crate::io::Hash),
                Result<(ValidationResult, Annotation), EnvError>,
            >,
        > = once_cell::sync::Lazy::new(Default::default);

        let key = (self.hash(), output.hash());
        if let Some(result) = VALIDATION.get(&key) {
            return result.clone();
        }

        let result = self.validate_output_helper(output);

        VALIDATION.insert(key, result.clone());

        result
    }
}

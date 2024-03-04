use ce_core::{define_env, rand, Env, Generate, ValidationResult};
use gcl::{ast::Commands, interpreter::InterpreterMemory, stringify::Stringify};
use serde::{Deserialize, Serialize};

define_env!(ParserEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Parser")]
pub struct Input {
    commands: Stringify<Commands>,
}

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "Parser")]
pub struct Output {
    pretty: Stringify<Commands>,
}

impl Env for ParserEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        Ok(Output {
            pretty: Stringify::new(input.commands.try_parse().map_err(
                ce_core::EnvError::invalid_input_for_program("failed to parse commands"),
            )?),
        })
    }

    fn validate(input: &Self::Input, output: &Self::Output) -> ce_core::Result<ValidationResult> {
        let (o_cmds, t_cmds) = match (
            Self::run(input)?.pretty.try_parse(),
            output.pretty.try_parse(),
        ) {
            (Ok(ours), Ok(theirs)) => (ours, theirs),
            (Err(err), _) | (_, Err(err)) => {
                return Ok(ValidationResult::Mismatch {
                    reason: format!("failed to parse pretty output: {:?}", err),
                })
            }
        };

        if !check_programs_for_semantic_equivalence(&o_cmds, &t_cmds) {
            return Ok(ValidationResult::Mismatch {
                reason: concat!(
                    "the pretty printed program is not semantically equivalent ",
                    "to the original program"
                )
                .to_string(),
            });
        }

        Ok(ValidationResult::CorrectTerminated)
    }
}

impl Generate for Input {
    type Context = ();

    fn gen<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        Self {
            commands: Stringify::new(Commands::gen(&mut Default::default(), rng)),
        }
    }
}

fn check_programs_for_semantic_equivalence(p1: &Commands, p2: &Commands) -> bool {
    let pg1 = gcl::pg::ProgramGraph::new(gcl::pg::Determinism::Deterministic, p1);
    let pg2 = gcl::pg::ProgramGraph::new(gcl::pg::Determinism::Deterministic, p2);

    let n_samples = 10;
    let n_steps = 10;

    let mut rng = <rand::rngs::SmallRng as rand::SeedableRng>::seed_from_u64(0xCEC34);

    for _ in 0..n_samples {
        let assignment = generate_input_assignment(p1, &mut rng);

        let mut exe1 = gcl::interpreter::Execution::new(assignment.clone());
        let mut exe2 = gcl::interpreter::Execution::new(assignment.clone());

        for _ in 0..n_steps {
            match (exe1.nexts(&pg1).first(), exe2.nexts(&pg2).first()) {
                (Some(next1), Some(next2)) => {
                    exe1 = next1.clone();
                    exe2 = next2.clone();
                }
                (None, None) => break,
                // NOTE: one of the executions is stuck while the other is not
                _ => return false,
            }
        }

        if exe1.current_mem() != exe2.current_mem() {
            return false;
        }
    }

    true
}

fn generate_input_assignment(
    commands: &gcl::ast::Commands,
    mut rng: &mut impl rand::Rng,
) -> InterpreterMemory {
    let initial_memory = gcl::memory::Memory::from_targets_with(
        commands.fv(),
        &mut rng,
        |rng, _| rng.gen_range(-10..=10),
        |rng, _| {
            let len = rng.gen_range(5..=10);
            (0..len).map(|_| rng.gen_range(-10..=10)).collect()
        },
    );
    InterpreterMemory {
        variables: initial_memory.variables,
        arrays: initial_memory.arrays,
    }
}

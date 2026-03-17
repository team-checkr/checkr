use ce_bigcl::Binify;
use ce_core::{Env, Generate, ValidationResult, define_env, rand};
use gcl::{
    ast::{AExpr, AOp, Array, BExpr, Commands, RelOp, Target, Variable},
    pg::{Action, Edge, Node, ProgramGraph},
};
use indexmap::IndexMap;
use riscvy::{Instruction, Label, Reg, RiscVFile, RiscVVM, Word};
use serde::{Deserialize, Serialize};
use stdx::stringify::Stringify;

define_env!(RiscVEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "RiscV")]
pub struct Input {
    commands: Stringify<Commands>,
}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "RiscV")]
pub struct Output {
    assembly: String,
}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[tapi(path = "RiscV")]
pub struct Annotation {
    pub pc: usize,
    pub regs: IndexMap<String, i32>,
    /// Map from name of label to it's location in memory and the value at that location
    pub variables: IndexMap<String, (i32, i32)>,
    pub memory: IndexMap<i32, i32>,
}

impl Env for RiscVEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    type Annotation = Annotation;

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let cmd =
            input
                .commands
                .try_parse()
                .map_err(ce_core::EnvError::invalid_input_for_program(
                    "failed to parse commands",
                ))?;
        let file = compile(input, &cmd);

        Ok(Output {
            assembly: file.to_string(),
        })
    }

    fn validate(
        input: &Self::Input,
        output: &Self::Output,
    ) -> ce_core::Result<(ValidationResult, Annotation)> {
        let file = match riscvy::RiscVFile::parse(&output.assembly) {
            Ok(file) => file,
            Err(err) => {
                return Ok((
                    ValidationResult::Mismatch {
                        reason: format!("failed to parse assembly: {err:?}"),
                    },
                    Annotation::default(),
                ));
            }
        };

        let cmd =
            input
                .commands
                .try_parse()
                .map_err(ce_core::EnvError::invalid_input_for_program(
                    "failed to parse commands",
                ))?;

        let (mem, asm) = file.assemble();
        let mut their_vm = RiscVVM::new(&asm, mem);
        let their_result = their_vm.run(10000);

        let ref_file = compile(input, &cmd);
        let (mem, asm) = ref_file.assemble();
        let mut ref_vm = RiscVVM::new(&asm, mem);
        let ref_result = ref_vm.run(10000);

        let display_data = ref_vm.to_display();

        let ann = Annotation {
            pc: display_data.pc,
            memory: display_data
                .memory
                .into_iter()
                .map(|(l, w)| (l.0, w.0))
                .collect(),
            regs: display_data
                .regs
                .into_iter()
                .map(|(l, w)| (l, w.0))
                .collect(),
            variables: display_data
                .variables
                .into_iter()
                .map(|(l, (a, b))| (l, (a.0, b.0)))
                .collect(),
        };

        use riscvy::StepResult;

        match (their_result, ref_result) {
            (StepResult::Exit, StepResult::Exit) => {}
            (StepResult::Ok, StepResult::Ok) => {}
            (StepResult::Stuck, StepResult::Stuck) => {}
            (_, _) => {
                return Ok((
                    ValidationResult::Mismatch {
                        reason: format!(
                            "programs stopped at different times. got: {their_result}, expected: {ref_result}",
                        ),
                    },
                    ann,
                ));
            }
        }

        for v in cmd.fv() {
            let x = their_vm.mem().load_word(&Label(format!("v{v}")));
            let y = ref_vm.mem().load_word(&Label(format!("v{v}")));
            if x != y {
                return Ok((
                    ValidationResult::Mismatch {
                        reason: format!(
                            "variable '{v}' has different value at end. got: {y}, expected: {x}",
                        ),
                    },
                    ann,
                ));
            }
        }

        Ok((ValidationResult::Correct, ann))
    }
}

fn compile(input: &Input, cmd: &Commands) -> RiscVFile {
    let mut ctx = ce_bigcl::Ctx::new(cmd.fv().into_iter().map(|t| t.name().to_string()).collect());
    let cmd = cmd.binify(&mut ctx);
    let fv = cmd.fv();
    let pg = ProgramGraph::new(gcl::pg::Determinism::NonDeterministic, &cmd);

    let mut file = RiscVFile::default();
    for name in &fv {
        file.push_data(name.to_label(), Word(0));
    }

    for node in pg.nodes() {
        use Instruction::*;

        file.push_label(node.to_label());
        match pg.outgoing(*node) {
            [] => {
                file.push_inst(li(Reg::a7(), Word(10)));
                file.push_inst(ecall);
            }
            [Edge(_, Action::Skip, t)] => {
                file.push_inst(j(t.to_label()));
            }
            [Edge(_, Action::Assignment(x, e), t)] => {
                match x {
                    Target::Variable(v) => {
                        file.push_inst(la(Reg::t0(), v.to_label()));
                        file.push_aexp(Reg::t1(), e);
                        file.push_inst(Instruction::sw(Reg::t1(), Word(0), Reg::t0()));
                    }
                    Target::Array(_, _) => todo!(),
                }
                file.push_inst(Instruction::j(t.to_label()));
            }
            [
                Edge(_, Action::Condition(a), t),
                Edge(_, Action::Condition(b), f),
            ] => {
                // NOTE: we know that b is !a
                match a {
                    BExpr::Bool(true) => file.push_inst(j(t.to_label())),
                    BExpr::Bool(false) => file.push_inst(j(f.to_label())),
                    BExpr::Rel(l, op, r) => {
                        match op {
                            RelOp::Lt => {
                                file.push_aexp(Reg::t0(), l);
                                file.push_aexp(Reg::t1(), r);
                                file.push_inst(blt(Reg::t0(), Reg::t1(), t.to_label()));
                                file.push_inst(j(f.to_label()));
                            }
                            RelOp::Gt => {
                                file.push_aexp(Reg::t1(), l);
                                file.push_aexp(Reg::t0(), r);
                                file.push_inst(blt(Reg::t0(), Reg::t1(), t.to_label()));
                                file.push_inst(j(f.to_label()));
                            }
                            // l <= r == ¬(r < l)
                            RelOp::Le => {
                                file.push_aexp(Reg::t1(), l);
                                file.push_aexp(Reg::t0(), r);
                                file.push_inst(blt(Reg::t0(), Reg::t1(), f.to_label()));
                                file.push_inst(j(t.to_label()));
                            }
                            // l >= r == ¬(l < r)
                            RelOp::Ge => {
                                file.push_aexp(Reg::t0(), l);
                                file.push_aexp(Reg::t1(), r);
                                file.push_inst(blt(Reg::t0(), Reg::t1(), f.to_label()));
                                file.push_inst(j(t.to_label()));
                            }
                            RelOp::Eq => {
                                file.push_aexp(Reg::t0(), l);
                                file.push_aexp(Reg::t1(), r);
                                file.push_inst(beq(Reg::t0(), Reg::t1(), t.to_label()));
                                file.push_inst(j(f.to_label()));
                            }
                            RelOp::Ne => {
                                file.push_aexp(Reg::t0(), l);
                                file.push_aexp(Reg::t1(), r);
                                file.push_inst(bne(Reg::t0(), Reg::t1(), t.to_label()));
                                file.push_inst(j(f.to_label()));
                            }
                        }
                    }
                    BExpr::Logic(_, _, _) => unreachable!(),
                    BExpr::Not(_) => unreachable!("found edge with ¬. they were: {a} and {b}"),
                }
            }
            edges => todo!("\n\n{}\n\n{cmd}\n\n{edges:?}", input.commands),
        }
    }
    file
}

trait RiscVEncoding {
    fn push_aexp(&mut self, reg: Reg, a: &AExpr);
}

impl RiscVEncoding for RiscVFile {
    fn push_aexp(&mut self, reg: Reg, a: &AExpr) {
        use Instruction::*;
        match a {
            AExpr::Number(n) => {
                self.push_inst(li(reg, Word(*n)));
            }
            AExpr::Reference(Target::Array(_, _)) => todo!(),
            AExpr::Reference(Target::Variable(y)) => {
                self.push_inst(lw(reg, y.to_label()));
            }
            AExpr::Binary(l, op, r) => {
                self.push_aexp(Reg::t1(), l);
                self.push_aexp(Reg::t2(), r);
                match op {
                    AOp::Plus => self.push_inst(add(Reg::t1(), Reg::t1(), Reg::t2())),
                    AOp::Minus => self.push_inst(sub(Reg::t1(), Reg::t1(), Reg::t2())),
                    AOp::Times => self.push_inst(mul(Reg::t1(), Reg::t1(), Reg::t2())),
                    AOp::Divide => self.push_inst(div(Reg::t1(), Reg::t1(), Reg::t2())),
                    AOp::Pow => self.push_halt(),
                }
            }
            AExpr::Minus(x) => {
                self.push_aexp(reg.clone(), x);
                self.push_inst(neg(reg.clone(), reg));
            }
        }
    }
}

trait ToLabel {
    fn to_label(&self) -> Label;
}

impl ToLabel for Target {
    fn to_label(&self) -> Label {
        match self {
            Target::Array(a, _) => a.to_label(),
            Target::Variable(v) => v.to_label(),
        }
    }
}
impl ToLabel for Array {
    fn to_label(&self) -> Label {
        Label(format!("v{self}"))
    }
}
impl ToLabel for Variable {
    fn to_label(&self) -> Label {
        Label(format!("v{self}"))
    }
}
impl ToLabel for Node {
    fn to_label(&self) -> Label {
        Label(format!("{self:?}"))
    }
}
impl ToLabel for Label {
    fn to_label(&self) -> Label {
        self.clone()
    }
}

impl Generate for Input {
    type Context = ();

    fn gn<R: rand::Rng>(_cx: &mut Self::Context, rng: &mut R) -> Self {
        use ce_core::gn::GclGenContext;
        let mut cx = GclGenContext {
            fuel: 5,
            ..GclGenContext::default()
        };
        Self {
            commands: Stringify::new(Commands(cx.many(1, 4, rng))),
        }
    }
}

use ce_bigcl::Binify;
use ce_core::{Env, Generate, ValidationResult, define_env, rand};
use gcl::{
    ast::{AExpr, AOp, BExpr, Commands, Target, Variable,  Array, RelOp},
    pg::{Action, Edge, Node, ProgramGraph},
};
use itertools::Either;
use serde::{Deserialize, Serialize};
use stdx::stringify::Stringify;

define_env!(RiscVEnv);

#[derive(tapi::Tapi, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    commands: Stringify<Commands>,
}

#[derive(tapi::Tapi, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {
    assembly: String,
}

impl Env for RiscVEnv {
    type Input = Input;

    type Output = Output;

    type Meta = ();

    fn run(input: &Self::Input) -> ce_core::Result<Self::Output> {
        let cmd =
            input
                .commands
                .try_parse()
                .map_err(ce_core::EnvError::invalid_input_for_program(
                    "failed to parse commands",
                ))?;
        let mut ctx =
            ce_bigcl::Ctx::new(cmd.fv().into_iter().map(|t| t.name().to_string()).collect());
        let cmd = cmd.binify(&mut ctx);
        // TODO: x1-x31 are reserved for the RISC-V architecture, so we should not use them for variables. We should use x32 and above instead.
        let fv = cmd.fv();
        let pg = ProgramGraph::new(gcl::pg::Determinism::NonDeterministic, &cmd);

        let mut file = RiscVFile {
            data: fv.iter().map(|name| (name.to_label(), Word(0))).collect(),
            text: [].to_vec(),
        };

        for node in pg.nodes() {
            use Instruction::*;

            file.push_label(node.to_label());
            match pg.outgoing(*node) {
                [] => {
                    file.push_inst(li(Reg::a7(), Value(Word(10))));
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
                            file.push_inst(Instruction::sw(Reg::t1(), 0, Reg::t0()));
                        }
                        Target::Array(_, _) => todo!(),
                    }
                    file.push_inst(Instruction::j(t.to_label()));
                }
                [
                    Edge(_, Action::Condition(a), t),
                    Edge(_, Action::Condition(_b), f),
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
                        BExpr::Not(_) => unreachable!(),
                    }
                }
                edges => todo!("\n\n{}\n\n{cmd}\n\n{edges:?}", input.commands),
            }
        }

        Ok(Output {
            assembly: file.to_string(),
        })
    }

    fn validate(_input: &Self::Input, _output: &Self::Output) -> ce_core::Result<ValidationResult> {
        Ok(ValidationResult::Correct)
    }
}

impl RiscVFile {
    pub fn push_aexp(&mut self, reg: Reg, a: &AExpr) {
        use Instruction::*;
        match a {
            AExpr::Number(n) => {
                self.push_inst(li(reg, Value(Word(*n))));
            }
            AExpr::Reference(Target::Array(_, _)) => todo!(),
            AExpr::Reference(Target::Variable(y)) => {
                self.push_inst(lw(reg, y.to_label()));
            }
            AExpr::Binary(l, op, r) => {
                self.push_aexp(Reg::t1(), &l);
                self.push_aexp(Reg::t2(), &r);
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
        let mut cx = ce_core::gn::GclGenContext::default();
        cx.fuel = 5;
        Self {
            commands: Stringify::new(Commands(cx.many(1, 4, rng))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Word(i32);
#[derive(Debug, Clone)]
struct Label(String);
#[derive(Debug, Clone)]
struct Value(Word);
#[derive(Debug, Clone)]
struct Reg(String);

#[allow(unused)]
impl Reg {
    fn t0() -> Reg {
        Reg("t0".to_string())
    }
    fn t1() -> Reg {
        Reg("t1".to_string())
    }
    fn t2() -> Reg {
        Reg("t2".to_string())
    }
    fn a0() -> Reg {
        Reg("a0".to_string())
    }
    fn a1() -> Reg {
        Reg("a1".to_string())
    }
    fn a2() -> Reg {
        Reg("a2".to_string())
    }
    fn a3() -> Reg {
        Reg("a3".to_string())
    }
    fn a4() -> Reg {
        Reg("a4".to_string())
    }
    fn a5() -> Reg {
        Reg("a5".to_string())
    }
    fn a6() -> Reg {
        Reg("a6".to_string())
    }
    fn a7() -> Reg {
        Reg("a7".to_string())
    }
}

struct RiscVFile {
    data: Vec<(Label, Word)>,
    text: Vec<Either<Label, Instruction>>,
}

#[repr(u8)]
enum CallNumber {
    Exit = 10,
}

impl RiscVFile {
    fn push_label(&mut self, label: impl ToLabel) {
        self.text.push(Either::Left(label.to_label()));
    }
    fn push_inst(&mut self, inst: Instruction) {
        self.text.push(Either::Right(inst));
    }
    fn push_halt(&mut self) {
        self.push_inst(Instruction::li(
            Reg::a7(),
            Value(Word(CallNumber::Exit as _)),
        ));
        self.push_inst(Instruction::ecall);
    }
}

#[allow(non_camel_case_types)]
mod helpers {
    pub type rd = super::Reg;
    pub type rs = super::Reg;
    pub type rs1 = super::Reg;
    pub type rs2 = super::Reg;
    pub type val = super::Value;
    pub type label = super::Label;
}

use helpers::*;

#[derive(Debug, Clone)]
#[allow(non_camel_case_types, unused)]
enum Instruction {
    // Load and Store Instructions
    // These instructions load data from memory into a register, copy data between registers, or store data from a register into memory.
    /// `li rd, val`
    ///
    /// ## Load immediate
    /// Load into register rd the 32-bit value val. (Pseudo instruction)
    li(rd, val),

    /// `lw rd, label`
    ///
    /// ## Load word
    /// Load into register rd the word stored at memory address label. (Pseudo instruction)
    lw(rd, label),

    /// `la rd, label`
    ///
    /// ## Load absolute
    /// Load into register rd the memory address label. (Pseudo instruction)
    la(rd, label),

    /// `mv rd, rs`
    ///
    /// ## Move
    /// Move (i.e. copy) the content of register rs into register rd.
    mv(rd, rs),

    /// `sw rs2, offset(rs1)`
    ///
    /// ## Store word
    /// Store the 32-bit value contained in the register rs2 into memory. The
    /// destination memory address is computed adding the value offset to the
    /// content of register rs1.
    sw(rs2, u32, rs1),

    // Integer Arithmetic Instructions
    // These instructions operate on base integer registers.
    /// `add rd, rs1, rs2`
    ///
    /// ## Addition
    /// Add the contents of registers rs1 and rs2 and store the result in
    /// register rd.
    add(rd, rs1, rs2),

    /// `neg rd, rs2`
    ///
    /// ## Negation
    /// Negates the contents of register rs2 and store the result in register
    /// rd.
    neg(rd, rs2),

    /// `sub rd, rs1, rs2`
    ///
    /// ## Subtraction
    /// Subtract the contents of register rs2 from rs1 and store the result in
    /// register rd.
    sub(rd, rs1, rs2),

    /// `mul rd, rs1, rs2`
    ///
    /// ## Multiplication
    /// Multiply the contents of registers rs2 and rs1 and store the result in
    /// register rd.
    mul(rd, rs1, rs2),

    /// `div rd, rs1, rs2`
    ///
    /// ## Division
    /// Divide the content of register rs1 by rs2 and store the result in
    /// register rd.
    div(rd, rs1, rs2),

    // Control Transfer Instructions
    // These instructions perform jumps, with or without conditions.
    /// `j label`
    ///
    /// ## Jump
    /// Jump to memory address label and execute the code stored there. (Pseudo
    /// instruction)
    j(label),

    /// `beq rs1, rs2, label`
    ///
    /// ## Branch if equal
    /// Compare the contents of registers rs1 and rs2, and jump to label if they
    /// are equal.
    beq(rs1, rs2, label),

    /// `bne rs1, rs2, label`
    ///
    /// ## Branch if not equal
    /// Compare the contents of registers rs1 and rs2, and jump to label if they
    /// are not equal.
    bne(rs1, rs2, label),

    /// `blt rs1, rs2, label`
    ///
    /// ## Branch if less than
    /// Compare the contents of registers rs1 and rs2, and jump to label if the
    /// content of rs1 is smaller than the content of rs2.
    blt(rs1, rs2, label),

    // System Instructions
    // These instructions allow a RISC-V assembly program to interact with the surrounding operating system.
    /// `ebreak`
    ///
    /// ## Environment break
    /// Stop the execution. This instruction acts as a breakpoint, and is used
    /// e.g. to let debuggers take control of a running program.
    ebreak,

    /// `ecall`
    ///
    /// ## Environment call
    /// Perform a system call. This will become clearer in when we will discuss
    /// the RISC-V Assembly Program Structure and RARS — RISC-V Assembler and
    /// Runtime Simulator.
    ecall,
}

impl std::fmt::Display for RiscVFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, ".data")?;
        for (name, word) in &self.data {
            writeln!(f, "{name}:\t.word {word}")?;
        }

        writeln!(f, ".text")?;
        for item in &self.text {
            match item {
                Either::Left(label) => {
                    writeln!(f, "{label}:")?;
                }
                Either::Right(inst) => {
                    writeln!(f, "{inst}")?;
                }
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}
impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}
impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::li(r, v) => write!(f, "\tli {r}, {v}"),
            Instruction::lw(r, l) => write!(f, "\tlw {r}, {l}"),
            Instruction::la(r, l) => write!(f, "\tla {r}, {l}"),
            Instruction::mv(a, b) => write!(f, "\tmv {a}, {b}"),
            Instruction::sw(a, o, b) => write!(f, "\tsw {a}, {o}({b})"),
            Instruction::add(a, b, c) => write!(f, "\tadd {a}, {b}, {c}"),
            Instruction::neg(a, b) => write!(f, "\tneg {a}, {b}"),
            Instruction::sub(a, b, c) => write!(f, "\tsub {a}, {b}, {c}"),
            Instruction::mul(a, b, c) => write!(f, "\tmul {a}, {b}, {c}"),
            Instruction::div(a, b, c) => write!(f, "\tdiv {a}, {b}, {c}"),
            Instruction::j(l) => write!(f, "\tj {l}"),
            Instruction::beq(a, b, l) => write!(f, "\tbeq {a}, {b}, {l}"),
            Instruction::bne(a, b, l) => write!(f, "\tbne {a}, {b}, {l}"),
            Instruction::blt(a, b, l) => write!(f, "\tblt {a}, {b}, {l}"),
            Instruction::ebreak => write!(f, "\tebreak"),
            Instruction::ecall => write!(f, "\tecall"),
        }
    }
}

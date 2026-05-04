mod instr;
mod parse;
mod vm;

use indexmap::IndexMap;
use itertools::Either;
pub use parse::ParseError;

pub use crate::instr::Instruction;

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Word(pub i32);
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Label(pub String);
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Reg(String);

#[allow(unused)]
impl Reg {
    pub fn t0() -> Reg {
        Reg("t0".to_string())
    }
    pub fn t1() -> Reg {
        Reg("t1".to_string())
    }
    pub fn t2() -> Reg {
        Reg("t2".to_string())
    }
    pub fn a0() -> Reg {
        Reg("a0".to_string())
    }
    pub fn a1() -> Reg {
        Reg("a1".to_string())
    }
    pub fn a2() -> Reg {
        Reg("a2".to_string())
    }
    pub fn a3() -> Reg {
        Reg("a3".to_string())
    }
    pub fn a4() -> Reg {
        Reg("a4".to_string())
    }
    pub fn a5() -> Reg {
        Reg("a5".to_string())
    }
    pub fn a6() -> Reg {
        Reg("a6".to_string())
    }
    pub fn a7() -> Reg {
        Reg("a7".to_string())
    }
}

#[derive(Default)]
pub struct RiscVFile {
    data: Vec<(Label, Word)>,
    text: Vec<Either<Label, Instruction<Reg, Label, Label>>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProgramPoint(u32);

impl ProgramPoint {
    pub fn inc(self) -> ProgramPoint {
        ProgramPoint(self.0 + 1)
    }
}

pub struct RiscVAssembly {
    labels: IndexMap<Label, ProgramPoint>,
    insts: Vec<Instruction<Reg, Label, Label>>,
}

pub enum CallNumber {}
impl CallNumber {
    pub const EXIT: Word = Word(10);
}

impl RiscVFile {
    pub fn parse(src: &str) -> Result<RiscVFile, ParseError> {
        parse::parse_file(src)
    }
    pub fn push_data(&mut self, label: Label, word: Word) {
        self.data.push((label, word));
    }
    pub fn push_label(&mut self, label: Label) {
        self.text.push(Either::Left(label));
    }
    pub fn push_inst(&mut self, inst: Instruction<Reg, Label, Label>) {
        self.text.push(Either::Right(inst));
    }
    pub fn push_halt(&mut self) {
        self.push_inst(Instruction::li(Reg::a7(), CallNumber::EXIT));
        self.push_inst(Instruction::ecall);
    }

    pub fn run(&self, steps: usize) -> (StepResult, RiscVVMDisplay) {
        let (bin, init_mem) = vm::Binary::from_file(self);
        let mut vm = vm::VM::new(init_mem);
        let res = vm.run(&bin, steps);
        (res, vm.display(&bin))
    }
}

impl RiscVAssembly {
    pub fn inst(&self, pc: ProgramPoint) -> Option<&Instruction<Reg, Label, Label>> {
        self.insts.get(pc.0 as usize)
    }
    pub fn lookup_label(&self, l: &Label) -> Option<ProgramPoint> {
        self.labels.get(l).copied()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StepResult {
    Ok,
    Stuck,
    Exit,
}

impl std::fmt::Display for StepResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepResult::Exit => write!(f, "exit"),
            StepResult::Stuck => write!(f, "stuck"),
            StepResult::Ok => write!(f, "ok"),
        }
    }
}

pub struct RiscVVMDisplay {
    pub pc: u32,
    pub regs: IndexMap<String, Word>,
    /// Map from name of label to it's location in memory and the value at that
    /// location
    pub variables: IndexMap<String, (Word, Word)>,
    pub memory: Vec<Word>,
}

impl std::fmt::Display for RiscVVMDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //let mut s = format!("mem: {:?}, pc: {:?}\n", self.memory, self.pc.0);
        writeln!(f, "CONTROL\n=========")?;
        writeln!(f, "pc: {}", self.pc)?;
        writeln!(f, "\nREGISTERS\n=========")?;
        for (reg, w) in &self.regs {
            writeln!(f, "{reg}: {w}")?;
        }
        writeln!(f, "\nVARIABLES\n=========")?;
        for (label, (loc, w)) in &self.variables {
            writeln!(f, "{label}@{loc}: {w}")?;
        }
        writeln!(f, "\nMEMORY\n=========")?;
        for (loc, w) in self.memory.iter().enumerate() {
            writeln!(f, "{loc}: {w}")?;
        }
        Ok(())
    }
}

impl std::ops::Add for Word {
    type Output = Word;
    fn add(self, rhs: Self) -> Self::Output {
        Word(self.0.wrapping_add(rhs.0))
    }
}
impl std::ops::Sub for Word {
    type Output = Word;
    fn sub(self, rhs: Self) -> Self::Output {
        Word(self.0.wrapping_sub(rhs.0))
    }
}
impl std::ops::Mul for Word {
    type Output = Word;
    fn mul(self, rhs: Self) -> Self::Output {
        Word(self.0.wrapping_mul(rhs.0))
    }
}
impl std::ops::Div for Word {
    type Output = Word;
    fn div(self, rhs: Self) -> Self::Output {
        Word(self.0.wrapping_div(rhs.0))
    }
}
impl std::ops::Neg for Word {
    type Output = Word;
    fn neg(self) -> Self::Output {
        Word(self.0.wrapping_neg())
    }
}

impl std::fmt::Display for RiscVFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, ".data")?;
        for (name, word) in &self.data {
            writeln!(f, "{name}:\t\t.word {word}")?;
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
impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        Ok(())
    }
}
impl std::fmt::Display for Instruction<Reg, Label, Label> {
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

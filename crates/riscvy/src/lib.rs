mod parse;

use indexmap::IndexMap;
use itertools::Either;
pub use parse::ParseError;

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
    text: Vec<Either<Label, Instruction>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ProgramPoint(usize);

impl ProgramPoint {
    pub fn inc(self) -> ProgramPoint {
        ProgramPoint(self.0 + 1)
    }
}

pub struct RiscVAssembly {
    labels: IndexMap<Label, ProgramPoint>,
    insts: Vec<Instruction>,
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
    pub fn push_inst(&mut self, inst: Instruction) {
        self.text.push(Either::Right(inst));
    }
    pub fn push_halt(&mut self) {
        self.push_inst(Instruction::li(Reg::a7(), CallNumber::EXIT));
        self.push_inst(Instruction::ecall);
    }
    pub fn assemble(self) -> (RiscVMemory, RiscVAssembly) {
        let mut asm = RiscVAssembly {
            labels: Default::default(),
            insts: Vec::new(),
        };
        let mem = RiscVMemory {
            labels: self
                .data
                .iter()
                .enumerate()
                .map(|(index, (label, _init))| (label.clone(), Word(index as _)))
                .collect(),
            heap: self
                .data
                .into_iter()
                .enumerate()
                .map(|(index, (_label, init))| (Word(index as _), init))
                .collect(),
            regs: Default::default(),
        };
        for item in self.text {
            match item {
                Either::Left(l) => {
                    asm.labels.insert(l, ProgramPoint(asm.insts.len()));
                }
                Either::Right(inst) => {
                    asm.insts.push(inst);
                }
            }
        }
        (mem, asm)
    }
}

impl RiscVAssembly {
    pub fn inst(&self, pc: ProgramPoint) -> Option<&Instruction> {
        self.insts.get(pc.0)
    }
    pub fn lookup_label(&self, l: &Label) -> Option<ProgramPoint> {
        self.labels.get(l).copied()
    }
}

#[allow(non_camel_case_types)]
mod helpers {
    pub type rd = super::Reg;
    pub type rs = super::Reg;
    pub type rs1 = super::Reg;
    pub type rs2 = super::Reg;
    pub type val = super::Word;
    pub type label = super::Label;
}

#[derive(Debug)]
pub struct RiscVMemory {
    regs: IndexMap<Reg, Word>,     // contents of registers
    labels: IndexMap<Label, Word>, // where in the heap is label l stored
    heap: IndexMap<Word, Word>,    // heap position to value stored there
}

pub struct RiscVVM<'a> {
    asm: &'a RiscVAssembly,
    memory: RiscVMemory,
    pc: ProgramPoint,
}

#[derive(Debug)]
pub enum StepResult {
    Ok,
    Stuck,
    Exit,
}

impl<'a> RiscVVM<'a> {
    pub fn new(asm: &'a RiscVAssembly, memory: RiscVMemory) -> RiscVVM<'a> {
        RiscVVM {
            asm,
            memory,
            pc: Default::default(),
        }
    }

    pub fn display(&self) -> String {
        //let mut s = format!("mem: {:?}, pc: {:?}\n", self.memory, self.pc.0);
        let mut s = String::new();
        s.push_str("CONTROL\n=========\n");
        s.push_str(&format!("pc: {}\n", self.pc.0));
        s.push_str("\nREGISTERS\n=========\n");
        for reg in &self.memory.regs {
            s.push_str(&format!("{}: {}\n", reg.0, reg.1.0));
        }
        s.push_str("\nVARIABLES\n=========\n");
        for label in &self.memory.labels {
            s.push_str(&format!(
                "{}@{}: {}\n",
                label.0,
                label.1,
                self.memory.load_at(*label.1)
            ));
        }
        s.push_str("\nMEMORY\n=========\n");
        for heap in &self.memory.heap {
            s.push_str(&format!("{}: {}\n", heap.0.0, heap.1.0));
        }
        s
    }

    pub fn run(&mut self) -> StepResult {
        for _ in 0..10000 {
            match self.step() {
                StepResult::Exit => return StepResult::Exit,
                StepResult::Stuck => return StepResult::Stuck,
                StepResult::Ok => {}
            }
        }
        StepResult::Ok
    }

    fn step(&mut self) -> StepResult {
        if let Some(inst) = self.asm.inst(self.pc) {
            self.pc = self.pc.inc();

            match inst {
                Instruction::li(reg, word) => {
                    self.memory.set_reg(reg.clone(), *word);
                }
                Instruction::lw(reg, label) => {
                    self.memory
                        .set_reg(reg.clone(), self.memory.load_word(label));
                }
                Instruction::la(reg, label) => {
                    self.memory
                        .set_reg(reg.clone(), self.memory.load_address(label));
                }
                Instruction::mv(reg, reg1) => {
                    self.memory.set_reg(reg.clone(), self.memory.load_reg(reg1));
                }
                Instruction::sw(reg, o, reg1) => {
                    self.memory
                        .set_at(*o + self.memory.load_reg(reg1), self.memory.load_reg(reg));
                }
                Instruction::add(reg, reg1, reg2) => {
                    self.memory.set_reg(
                        reg.clone(),
                        self.memory.load_reg(reg1) + self.memory.load_reg(reg2),
                    );
                }
                Instruction::neg(reg, reg1) => {
                    self.memory
                        .set_reg(reg.clone(), -self.memory.load_reg(reg1));
                }
                Instruction::sub(reg, reg1, reg2) => {
                    self.memory.set_reg(
                        reg.clone(),
                        self.memory.load_reg(reg1) - self.memory.load_reg(reg2),
                    );
                }
                Instruction::mul(reg, reg1, reg2) => {
                    self.memory.set_reg(
                        reg.clone(),
                        self.memory.load_reg(reg1) * self.memory.load_reg(reg2),
                    );
                }
                Instruction::div(reg, reg1, reg2) => {
                    let r = self.memory.load_reg(reg2);
                    if r == Word(0) {
                        return StepResult::Stuck;
                    } else {
                        self.memory
                            .set_reg(reg.clone(), self.memory.load_reg(reg1) / r);
                    }
                }
                Instruction::j(label) => {
                    if let Some(pc) = self.asm.lookup_label(label) {
                        self.pc = pc
                    } else {
                        return StepResult::Stuck;
                    }
                }
                Instruction::beq(reg, reg1, label) => {
                    let Some(pc) = self.asm.lookup_label(label) else {
                        return StepResult::Stuck;
                    };

                    if self.memory.load_reg(reg) == self.memory.load_reg(reg1) {
                        self.pc = pc;
                    }
                }
                Instruction::bne(reg, reg1, label) => {
                    let Some(pc) = self.asm.lookup_label(label) else {
                        return StepResult::Stuck;
                    };

                    if self.memory.load_reg(reg) != self.memory.load_reg(reg1) {
                        self.pc = pc;
                    }
                }
                Instruction::blt(reg, reg1, label) => {
                    let Some(pc) = self.asm.lookup_label(label) else {
                        return StepResult::Stuck;
                    };

                    if self.memory.load_reg(reg) < self.memory.load_reg(reg1) {
                        self.pc = pc;
                    }
                }
                Instruction::ebreak => return StepResult::Stuck,
                Instruction::ecall => match self.memory.load_reg(&Reg::a7()) {
                    CallNumber::EXIT => {
                        return StepResult::Exit;
                    }
                    _ => return StepResult::Stuck,
                },
            }

            StepResult::Ok
        } else {
            StepResult::Stuck
        }
    }
}

impl RiscVMemory {
    fn set_reg(&mut self, reg: Reg, word: Word) {
        self.regs.insert(reg, word);
    }
    fn load_reg(&self, reg: &Reg) -> Word {
        self.regs.get(reg).copied().unwrap_or_default()
    }
    fn set_at(&mut self, address: Word, word: Word) {
        self.heap.insert(address, word);
    }
    fn load_at(&self, word: Word) -> Word {
        self.heap.get(&word).copied().unwrap_or_default()
    }
    fn set_word(&mut self, label: Label, word: Word) {
        self.set_at(self.load_address(&label), word);
    }
    fn load_word(&self, label: &Label) -> Word {
        self.load_at(self.load_address(label))
    }
    fn load_address(&self, label: &Label) -> Word {
        self.labels.get(label).copied().unwrap_or_default()
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

use helpers::*;

#[derive(Debug, Clone)]
#[allow(non_camel_case_types, unused)]
pub enum Instruction {
    // Load and Store Instructions
    // These instructions load data from memory into a register, copy data between registers, or
    // store data from a register into memory.
    /// `li rd, val`
    ///
    /// ## Load immediate
    /// Load into register rd the 32-bit word val. (Pseudo instruction)
    li(rd, val),

    /// `lw rd, label`
    ///
    /// ## Load word
    /// Load into register rd the word stored at memory address label. (Pseudo
    /// instruction)
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
    /// Store the 32-bit word contained in the register rs2 into memory. The
    /// destination memory address is computed adding the word offset to the
    /// content of register rs1.
    sw(rs2, Word, rs1),

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
    // These instructions allow a RISC-V assembly program to interact with the surrounding
    // operating system.
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

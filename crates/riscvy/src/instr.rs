use crate::Word;

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types, unused)]
pub enum Instruction<R, L, P> {
    // Load and Store Instructions
    // These instructions load data from memory into a register, copy data between registers, or
    // store data from a register into memory.
    /// `li rd, val`
    ///
    /// ## Load immediate
    /// Load into register rd the 32-bit word val. (Pseudo instruction)
    li(R, Word),

    /// `lw rd, label`
    ///
    /// ## Load word
    /// Load into register rd the word stored at memory address label. (Pseudo
    /// instruction)
    lw(R, L),

    /// `la rd, label`
    ///
    /// ## Load absolute
    /// Load into register rd the memory address label. (Pseudo instruction)
    la(R, L),

    /// `mv rd, rs`
    ///
    /// ## Move
    /// Move (i.e. copy) the content of register rs into register rd.
    mv(R, R),

    /// `sw rs2, offset(rs1)`
    ///
    /// ## Store word
    /// Store the 32-bit word contained in the register rs2 into memory. The
    /// destination memory address is computed adding the word offset to the
    /// content of register rs1.
    sw(R, Word, R),

    // Integer Arithmetic Instructions
    // These instructions operate on base integer registers.
    /// `add rd, R, rs2`
    ///
    /// ## Addition
    /// Add the contents of registers rs1 and rs2 and store the result in
    /// register rd.
    add(R, R, R),

    /// `neg rd, rs2`
    ///
    /// ## Negation
    /// Negates the contents of register rs2 and store the result in register
    /// rd.
    neg(R, R),

    /// `sub rd, R, rs2`
    ///
    /// ## Subtraction
    /// Subtract the contents of register rs2 from rs1 and store the result in
    /// register rd.
    sub(R, R, R),

    /// `mul rd, R, rs2`
    ///
    /// ## Multiplication
    /// Multiply the contents of registers rs2 and rs1 and store the result in
    /// register rd.
    mul(R, R, R),

    /// `div rd, R, rs2`
    ///
    /// ## Division
    /// Divide the content of register rs1 by rs2 and store the result in
    /// register rd.
    div(R, R, R),

    // Control Transfer Instructions
    // These instructions perform jumps, with or without conditions.
    /// `j label`
    ///
    /// ## Jump
    /// Jump to memory address label and execute the code stored there. (Pseudo
    /// instruction)
    j(P),

    /// `beq rs1, rs2, label`
    ///
    /// ## Branch if equal
    /// Compare the contents of registers rs1 and rs2, and jump to label if they
    /// are equal.
    beq(R, R, P),

    /// `bne rs1, rs2, label`
    ///
    /// ## Branch if not equal
    /// Compare the contents of registers rs1 and rs2, and jump to label if they
    /// are not equal.
    bne(R, R, P),

    /// `blt rs1, rs2, label`
    ///
    /// ## Branch if less than
    /// Compare the contents of registers rs1 and rs2, and jump to label if the
    /// content of rs1 is smaller than the content of rs2.
    blt(R, R, P),

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

impl<R, L, P> Instruction<R, L, P> {
    pub fn map<S, T, U>(
        self,
        mut f: impl FnMut(R) -> S,
        mut g: impl FnMut(L) -> T,
        mut h: impl FnMut(P) -> U,
    ) -> Instruction<S, T, U> {
        use Instruction::*;

        match self {
            li(r, v) => li(f(r), v),
            lw(r, l) => lw(f(r), g(l)),
            la(r, l) => la(f(r), g(l)),
            mv(a, b) => mv(f(a), f(b)),
            sw(a, o, b) => sw(f(a), o, f(b)),
            add(a, b, c) => add(f(a), f(b), f(c)),
            neg(a, b) => neg(f(a), f(b)),
            sub(a, b, c) => sub(f(a), f(b), f(c)),
            mul(a, b, c) => mul(f(a), f(b), f(c)),
            div(a, b, c) => div(f(a), f(b), f(c)),
            j(l) => j(h(l)),
            beq(a, b, l) => beq(f(a), f(b), h(l)),
            bne(a, b, l) => bne(f(a), f(b), h(l)),
            blt(a, b, l) => blt(f(a), f(b), h(l)),
            ebreak => ebreak,
            ecall => ecall,
        }
    }
}

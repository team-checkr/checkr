module RiscV

open Io.RiscV
open Compiler
open AST
open State

type Register = string
type Ident = string
type Label = string

type Operand = 
    | Number of int
    | Variable of Ident
    
type Expression =
    //TODO: Extend expression and implement relevant functions
    | Add of (Operand * Operand)
    | Sub of (Operand * Operand)
    | Mul of (Operand * Operand)
    | Div of (Operand * Operand)
    | Neg of Operand
    | Direct of Operand
    
type booleanOperator =
    | Eq of (Operand * Operand)
    | Neq of (Operand * Operand)
    | Lt of (Operand * Operand)
    | Gt of (Operand * Operand)
    | Leq of (Operand * Operand)
    | Geq of (Operand * Operand)
    | True
    | False

type Action =
    | Assignment of (Ident * Expression * Label)
    | Comparison of (booleanOperator * Label * Label) // comparison, label to jump to if true, label to jump to if false
    | Skip of Label // dont know what we do yet

type Program = list<Ident * Action>

/// Individual RiscV instruction
type RiscV =
    // Assignments
    | Li of (Register * int)
    | La of (Register * Ident)
    | Lw of (Register * Ident)
    | Sw of (Register * int * Register)
    | NegV of (Register * Register)

    // Control flow
    | J of Label
    | Beq of Register * Register * Label
    | Bne of Register * Register * Label
    | Blt of Register * Register * Label

    // Integer Arithmatics
    | AddV of Register * Register * Register
    | SubV of Register * Register * Register
    | MulV of Register * Register * Register
    | DivV of Register * Register * Register

    | Ecall

    override this.ToString() =
        match this with
        // Assignments
        | Li (rd, imm) ->
            sprintf "li %O, %d" rd imm

        | La (rd, label) ->
            sprintf "la %O, %O" rd label

        | Lw (rd, label) ->
            sprintf "lw %O, %O" rd label

        | Sw (rs, offset, baseReg) ->
            sprintf "sw %O, %d(%O)" rs offset baseReg

        | NegV (rd, rs) ->
            sprintf "neg %O, %O" rd rs

        // Control flow
        | J label ->
            sprintf "j %O" label

        | Beq (r1, r2, label) ->
            sprintf "beq %O, %O, %O" r1 r2 label

        | Bne (r1, r2, label) ->
            sprintf "bne %O, %O, %O" r1 r2 label

        | Blt (r1, r2, label) ->
            sprintf "blt %O, %O, %O" r1 r2 label

        // Arithmetic
        | AddV (rd, r1, r2) ->
            sprintf "add %O, %O, %O" rd r1 r2

        | SubV (rd, r1, r2) ->
            sprintf "sub %O, %O, %O" rd r1 r2

        | MulV (rd, r1, r2) ->
            sprintf "mul %O, %O, %O" rd r1 r2

        | DivV (rd, r1, r2) ->
            sprintf "div %O, %O, %O" rd r1 r2

        | Ecall ->
            "ecall"

/// Compiled Instructions for a given node
type CompiledNode = Label * list<RiscV>

let state = State.StatefulBuilder()
let GenerateRegister = State (fun s ->
    let reg = sprintf "t%d" s
    reg, s + 1
)

// Invariant: Given the code is transpiled first, for each node there is at most 2 outgoing edges.
// If there are 2 outgoing edges, we have a comparison action, and the outgoing edges are the true and false branches of the comparison.
// If 1 outgoing edge, then we have a assignment action, and the outgoing edge is the next node to execute after the assignment.

//let expressionHandler 
//let binaryExpression

// Split up to ensureVariable or ensure Boolean. (For conditional we should process one branch and jump to the other)


let rec stripParenBool (b: booleanExpr) : booleanExpr =
    match b with
    | AST.booleanExpr.ParenBool inner -> stripParenBool inner
    | AST.booleanExpr.Neg inner -> AST.booleanExpr.Neg (stripParenBool inner)
    | AST.booleanExpr.And (l, r) -> AST.booleanExpr.And (stripParenBool l, stripParenBool r)
    | AST.booleanExpr.Or (l, r) -> AST.booleanExpr.Or (stripParenBool l, stripParenBool r)
    | AST.booleanExpr.Scand (l, r) -> AST.booleanExpr.Scand (stripParenBool l, stripParenBool r)
    | AST.booleanExpr.Scor (l, r) -> AST.booleanExpr.Scor (stripParenBool l, stripParenBool r)
    | _ -> b

let enforceOperand (expr: expr) : Operand =
    match expr with
        |expr.Variable(ident) -> Variable ident
        |expr.Num(int) -> Number int
        | _ -> failwith "Expected operands of expressions  to be either variable or number"

let rec enforceExpr (expr: AST.expr) : Expression = 
    // The invariant ensures we can only encounter binary operators or Negation.
    match expr with
    | PlusExpr(left, right) -> Add(enforceOperand left, enforceOperand right)
    | MinusExpr(left, right) -> Sub(enforceOperand left, enforceOperand right)
    | TimesExpr(left, right) -> Mul(enforceOperand left, enforceOperand right)
    | DivExpr(left, right) -> Div(enforceOperand left, enforceOperand right)
    | UMinusExpr(expr) -> Neg(enforceOperand expr)
    | Num _ 
    | expr.Variable _ -> Direct (enforceOperand expr)
    |ParenExpr(inner) -> enforceExpr inner
    | _ -> failwith ("non implemented arithmetic expression" + (string expr))


let enforceBoolean (b : booleanExpr) : booleanOperator =
    // match with the expected boolean expressions and convert
    match b with
    | booleanExpr.Eq(left,right) -> Eq(enforceOperand left, enforceOperand right)
    | booleanExpr.Neq(left, right) -> Neq(enforceOperand left, enforceOperand right)
    | booleanExpr.Lt(left, right) -> Lt(enforceOperand left, enforceOperand right)
    | booleanExpr.Gt(left, right) -> Gt(enforceOperand left, enforceOperand right)
    | booleanExpr.Leq(left, right) -> Leq(enforceOperand left, enforceOperand right)
    | booleanExpr.Geq(left, right) -> Geq(enforceOperand left, enforceOperand right)
    | booleanExpr.True -> True
    | booleanExpr.False -> False
    | _ -> failwith "non implemented booolean expression"



let extractTruthBranch
    (l: booleanExpr)
    (r: booleanExpr)
    (labels: Label * Label)
    : booleanExpr * Label * Label =
    let lNorm = stripParenBool l
    let rNorm = stripParenBool r
    match (lNorm, rNorm) with
    | AST.booleanExpr.Neg x, y when x = y -> y, snd labels, fst labels
    | x, AST.booleanExpr.Neg y when x = y -> x, fst labels, snd labels
    | _ ->
        failwith ("Expected complementary guards a and !a: " + string l + " / " + string r)


let extractComparison (actions : (Compiler.Action * Compiler.Action)) (labels : (Label * Label)) : Action =
    match actions with
        |B (l: booleanExpr), B (r: booleanExpr) -> 
            let (truth_branch, truth_ident, false_ident) = extractTruthBranch l r labels
            let left = enforceBoolean truth_branch
            
            Comparison(left, truth_ident, false_ident)

        | _ -> failwith "Unreachable due to invariant: the 2 received actions are not of type B."

let extractAssignment (action : Compiler.Action) (next_node : Label) : Action =
    match action with
    | Compiler.VariableAssignment (ident, expr) -> Assignment(ident, enforceExpr expr, next_node)
    | Compiler.Skip -> Skip (next_node)
    | _ -> failwith "Unreachable due to invariant: the received action is not of type VariableAssignment."


let enforceProgram (program: Graph) : Program =
    let nodes =
        program
        |> List.map (fun (node, _, _) -> node)
        |> List.distinct

    let outgoing (node: Node) = Interpreter.outgoing node program

    nodes
    |> List.map (fun node ->
        let edges = outgoing node
        match edges with
        | (_, action, next_node) :: [] ->
            node, extractAssignment action next_node
        | (_, left_action, left_next_node) :: (_, right_action, right_next_node) :: [] ->
            node, extractComparison (left_action, right_action) (left_next_node, right_next_node)
        | [] -> failwith "what about the empty case?"
        | _ -> failwith "invariant ensures 2 outgoing edges at max"
    )



let operandHandler (op:Operand): State<int, RiscV * Ident> = 
    state{
        let! register = GenerateRegister
        match op with
        | Number(int) -> return Li(register, int), register 
        | Variable(ident) -> return Lw(register, "v"+ident), register
    }



let comparisonHandler (op: booleanOperator, q1: Label, q2: Label) : State<int, list<RiscV>> =
    state {
        match op with
        // TODO: False and true are missing
        | Eq (left, right) ->   
            let! (pLeft, t0) = operandHandler left;
            let! (pRight, t1) = operandHandler right;
            return pLeft :: pRight :: Beq(t0, t1, q1) :: J(q2) :: []
        | Neq (left, right) -> 
            let! (pLeft, t0) = operandHandler left;
            let! (pRight, t1) = operandHandler right;
            return pLeft :: pRight :: Bne(t0, t1, q1) :: J(q2) :: []
        | Lt (left, right) -> 
            let! (pLeft, t0) = operandHandler left;
            let! (pRight, t1) = operandHandler right;
            return pLeft :: pRight :: Blt(t0, t1, q1) :: J(q2) :: []
        | Gt (left, right) -> 
            let! (pLeft, t0) = operandHandler right;
            let! (pRight, t1) = operandHandler left;
            return pLeft :: pRight :: Blt(t0, t1, q1) :: J(q2) :: []
        | Leq (left, right) -> 
            let! (pLeft, t0) = operandHandler right;
            let! (pRight, t1) = operandHandler left;
            return pLeft :: pRight :: Blt(t0, t1, q2) :: J(q1) :: []
        | Geq (left, right) ->
            let! (pLeft, t0) = operandHandler left;
            let! (pRight, t1) = operandHandler right;
            return pLeft :: pRight :: Blt(t0, t1, q2) :: J(q1) :: []
        | True -> return J(q1)::[]
        | False -> return J(q2)::[]
    }

// if a < b LT
// lw t0 va
// lw t1 vb
// blt t0, t1, qx
// j qy

// if a > b GT
// lw t1 va
// lw t0 vb
// blt t0, t1, qx
// j qy

// if a >= b GEQ
// lw t0 va
// lw t1 vb
// blt t0, t1, qy
// j qx

// if a <= b LEQ
// lw t1 va
// lw t0 vb
// blt t0, t1, qy
// j qx

let handleExpression (expression: Expression) : State<int, list<RiscV> * Ident> =
    // For opreand load it using state, giving the identifier used and riscV instruction
    state {
        match expression with
            | Add(left, right) ->  
                let! (instruction1, ident1) = operandHandler left
                let! (instruction2, ident2) = operandHandler right
                return [instruction1;instruction2]@[AddV(ident1, ident1, ident2)], ident1 //TODO: fix registers here?
            | Sub(left, right) ->                 
                let! (instruction1, ident1) = operandHandler left
                let! (instruction2, ident2) = operandHandler right
                return [instruction1;instruction2]@[SubV(ident1, ident1, ident2)], ident1 //TODO: fix registers here?
            | Mul(left, right) ->
                let! (instruction1, ident1) = operandHandler left
                let! (instruction2, ident2) = operandHandler right
                return [instruction1;instruction2]@[MulV(ident1, ident1, ident2)], ident1 //TODO: fix registers here?
            | Div(left, right) ->
                let! (instruction1, ident1) = operandHandler left
                let! (instruction2, ident2) = operandHandler right
                return [instruction1;instruction2]@[DivV(ident1, ident1, ident2)], ident1 //TODO: fix registers here?
            | Neg(operand) ->
                let! (instruction, ident) = operandHandler operand
                return instruction::[NegV(ident, ident)], ident
            | Direct (operand) ->
                let! (instruction, ident) = operandHandler operand
                return [instruction], ident

            //TODO: possibly should be a '_ -> failwith "non implemented expression type can't be handled!!"' here but monadland doesn't let me do it :(
            // you just need to do a |_ -> state{failwith "non implemented expression type"}
            // ok smart guy why don't you do it then!!!
            // I don't want to
            // Precise lack of skill
            // nuh-uh, i just dont wanna :p
            }


let handleAction (action: Action) (node_name: string) : State<int, CompiledNode> =
    state {
        match action with
        | Assignment(ident, expr: Expression, label) -> 
            // get adress <- "la va" ||| La(, ident)
            let! address_register = GenerateRegister
            let loadInstruction = La(address_register, "v" + ident) //v because our variables are named differently(?)
            
            // handle expression ie generate and extract registers to perform binary operator
            // save it at adress for adress
            let! (arithmeticInstructions, expression_ident) = handleExpression expr
            let result = (loadInstruction :: arithmeticInstructions) @ [Sw(expression_ident, 0, address_register)] //TODO: fix registers here?

            return node_name, result @ [J label]
            
        | Comparison(operator, label1, label2) -> 
            let! c = comparisonHandler(operator, label1, label2)
            return node_name, c        
        | Skip(label) -> return node_name, [J (label)]
    }
    
// retreve address (dont change state)
// retrieve variable

let outer (program: Program) : list<CompiledNode> =
    let result = program |> List.map (
        fun (ident, action) ->
        let result = handleAction action ident |> State.run 0
        fst result

    )
    result @ [CompiledNode ("qFinal", Li("a7", 10) :: Ecall :: [])]

let printCompiledNode (node: CompiledNode) : string =
    let (nodeName, instructions) = node
    let printedInstructions =
        instructions
        |> List.map (fun ins -> "\t" + string ins)
        |> String.concat "\n"

    match printedInstructions with
    | "" -> sprintf "%s:" nodeName
    | _ -> sprintf "%s:\n%s" nodeName printedInstructions
    
let printProgram (program: list<CompiledNode>) : string =
    program
    |> List.map printCompiledNode
    |> String.concat "\n\n"

let buildVariablePreamble (variables: list<string>) : string =
    let declarations =
        variables
        |> List.map (fun v -> "v" + v)
        |> List.distinct
        |> List.map (fun name -> sprintf "%s:\t\t.word 0" name)
        |> String.concat "\n"

    if declarations = "" then
        ".data\n.text"
    else
        ".data\n" + declarations + "\n.text"
    


let analysis (input: Input) : Output =
    let ast = Parser.parse_string input.commands
    let bigcl = BiGCL.analysis {commands=input.commands}
    
    let pg: Graph = Compiler.graph_non_det (Parser.parse_string bigcl.binary)
    let p = enforceProgram pg
    let variables = BiGCL.add_variable_to_list (Parser.parse_string bigcl.binary) []

    let pre = buildVariablePreamble variables
    let result = outer p
    let body = pre + "\n\n" + (printProgram result)
    { assembly = body}

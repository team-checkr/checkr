module Interpreter
open Io.Interpreter
open Io.GCL
open AST
open Parser
open Compiler


let getVar (mem: InterpreterMemory) (x: string) : int =
    match mem.variables.TryFind x with
    | Some v -> v
    | None   -> 0

let getArr (mem: InterpreterMemory) (a: string) (i: int) : int =
    match mem.arrays.TryFind a with
    | Some arr -> arr.[i]
    | None     -> 0

let setVar (mem: InterpreterMemory) (x: string) (v: int) : InterpreterMemory =
    { mem with variables = mem.variables |> Map.add x v }

let setArr (mem: InterpreterMemory) (a: string) (i: int) (v: int) : InterpreterMemory =
    let arr =
        match mem.arrays.TryFind a with
        | Some a -> a
        | None   -> List.replicate (i + 1) 0
    let updated = arr |> List.mapi (fun j x -> if j = i then v else x)
    { mem with arrays = mem.arrays |> Map.add a updated }


let rec evalExpr (mem: InterpreterMemory) (e: expr) : int =
    match e with
    | Num n          -> int n
    | Var x          -> getVar mem x
    | Array(a, i)    -> getArr mem a (evalExpr mem i)
    | PlusExpr(l, r) -> evalExpr mem l + evalExpr mem r
    | MinusExpr(l,r) -> evalExpr mem l - evalExpr mem r
    | TimesExpr(l,r) -> evalExpr mem l * evalExpr mem r
    | DivExpr(l, r)  ->
        let divisor = evalExpr mem r
        if divisor = 0 then failwith "Division by zero"
        else evalExpr mem l / divisor
    | PowExpr(l, r)  -> int (float (evalExpr mem l) ** float (evalExpr mem r))
    | UMinusExpr e   -> -(evalExpr mem e)


and evalBexpr (mem: InterpreterMemory) (b: bexpr) : bool =
    match b with
    | True             -> true
    | False            -> false
    | AndExpr(l, r)    -> evalBexpr mem l && evalBexpr mem r
    | OrExpr(l, r)     -> evalBexpr mem l || evalBexpr mem r
    | AndAndExpr(l, r) -> evalBexpr mem l && evalBexpr mem r  // short-circuit
    | OrOrExpr(l, r)   -> evalBexpr mem l || evalBexpr mem r  // short-circuit
    | NotExpr b        -> not (evalBexpr mem b)
    | EqExpr(l, r)     -> evalExpr mem l = evalExpr mem r
    | NeqExpr(l, r)    -> evalExpr mem l <> evalExpr mem r
    | GtExpr(l, r)     -> evalExpr mem l > evalExpr mem r
    | GteExpr(l, r)    -> evalExpr mem l >= evalExpr mem r
    | LtExpr(l, r)     -> evalExpr mem l < evalExpr mem r
    | LteExpr(l, r)    -> evalExpr mem l <= evalExpr mem r


let executeAction (mem: InterpreterMemory) (action: string) : InterpreterMemory option =
    // Re-parse the action label back into an AST node
    // Actions are either assignments or boolean conditions
    let tryParseExprAction () =
        // Try parsing as a command (assignment)
        match Parser.parse Grammar.start_commands action with
        | Ok (Assign(x, e)) ->
            let v = evalExpr mem e
            Some (setVar mem x v)
        | Ok (ArrAssign(a, i, e)) ->
            let idx = evalExpr mem i
            let v   = evalExpr mem e
            Some (setArr mem a idx v)
        | Ok Skip ->
            Some mem
        | _ ->
            // Try parsing as a boolean expression (guard)
            match Parser.parse Grammar.start_expression action with
            | Ok bexprAst ->
                // start_expression returns expr, not bexpr
                // guards are printed as bexpr strings, re-parse differently
                None
            | _ -> None
    tryParseExprAction ()

// Execute a single edge: returns Some newMemory if enabled, None if not
let tryEdge (mem: InterpreterMemory) (action: string) : InterpreterMemory option =
    try
        // Parse action as command first
        match Parser.parse Grammar.start_commands action with
        | Ok cmd ->
            match cmd with
            | Assign(x, e)       -> Some (setVar mem x (evalExpr mem e))
            | ArrAssign(a, i, e) -> Some (setArr mem a (evalExpr mem i) (evalExpr mem e))
            | Skip               -> Some mem
            | _                  -> None
        | _ ->
            // Not a command — must be a boolean guard
            // Parse as bexpr by wrapping in a dummy if
            let wrapped = sprintf "if %s -> skip fi" action
            match Parser.parse Grammar.start_commands wrapped with
            | Ok (If(Guard(b, Skip))) ->
                if evalBexpr mem b then Some mem else None
            | _ -> None
    with _ -> None


let step (edgeList: Edge list) (det: Determinism) (node: Node) (mem: InterpreterMemory)
         : (Node * string * InterpreterMemory) list =
    let outgoing = edgeList |> List.filter (fun (q, _, _) -> q = node)
    match det with
    | NonDeterministic ->
        // Return all enabled transitions
        outgoing |> List.choose (fun (_, action, q') ->
            match tryEdge mem action with
            | Some mem' -> Some (q', action, mem')
            | None      -> None)
    | Deterministic ->
        // Return only the first enabled transition
        outgoing
        |> List.tryPick (fun (_, action, q') ->
            match tryEdge mem action with
            | Some mem' -> Some (q', action, mem')
            | None      -> None)
        |> Option.toList


let rec execute (edgeList: Edge list) (det: Determinism) (node: Node) (mem: InterpreterMemory)
                (remaining: int) (acc: Step list) : Step list * Node * TerminationState =
    if remaining = 0 then
        (List.rev acc, node, Running)
    else
        match step edgeList det node mem with
        | [] ->
            // No enabled transitions
            let termination = if node = "qf" then Terminated else Stuck
            (List.rev acc, node, termination)
        | (nextNode, action, nextMem) :: _ ->
            // Take the first available step (for deterministic, only one exists anyway)
            let stepRecord = { action = action; node = nextNode; memory = nextMem }
            execute edgeList det nextNode nextMem (remaining - 1) (stepRecord :: acc)


let analysis (input: Input) : Output =
    let qs = "q0"
    let qf = "qf"
    match Parser.parse Grammar.start_commands input.commands with
    | Error e ->
        { initial_node  = qs
          final_node    = qf
          dot           = sprintf "// Parse error: %A" e
          trace         = []
          termination   = Stuck }
    | Ok ast ->
        Compiler.nodeCount <- 0
        let edgeList = Compiler.edges input.determinism ast qs qf
        let dot      = Compiler.printDot qs qf edgeList
        let (trace, finalNode, termination) =
            execute edgeList input.determinism qs input.assignment input.trace_length []
        { initial_node = qs
          final_node   = qf
          dot          = dot
          trace        = trace
          termination  = termination }
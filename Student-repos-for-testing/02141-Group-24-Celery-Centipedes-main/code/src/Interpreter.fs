module Interpreter

open Io.Interpreter
open AST

let rng = System.Random()

let rec evalExpr (mem: InterpreterMemory) (e: expr) : int32 =
    match e with
    | Num n -> int32 n
    | Var x -> Map.find x mem.variables
    | ArrayAccess (name, idx) ->
        let i = evalExpr mem idx
        let arr = Map.find name mem.arrays
        arr.[int i]
    | PlusExpr (a, b) -> evalExpr mem a + evalExpr mem b
    | MinusExpr (a, b) -> evalExpr mem a - evalExpr mem b
    | TimesExpr (a, b) -> evalExpr mem a * evalExpr mem b
    | DivExpr (a, b) -> evalExpr mem a / evalExpr mem b
    | PowExpr (a, b) ->
        let base' = evalExpr mem a
        let exp = evalExpr mem b
        if exp < 0 then failwith "negative exponent"
        else pown base' (int exp)
    | UMinusExpr a -> -(evalExpr mem a)

let rec evalBool (mem: InterpreterMemory) (b: boolExpr) : bool =
    match b with
    | True -> true
    | False -> false
    | AndExpr (a, b) -> evalBool mem a && evalBool mem b
    | OrExpr (a, b) -> evalBool mem a || evalBool mem b
    | ShortAndExpr (a, b) -> evalBool mem a && evalBool mem b
    | ShortOrExpr (a, b) -> evalBool mem a || evalBool mem b
    | NotExpr a -> not (evalBool mem a)
    | EqExpr (a, b) -> evalExpr mem a = evalExpr mem b
    | NeqExpr (a, b) -> evalExpr mem a <> evalExpr mem b
    | GtExpr (a, b) -> evalExpr mem a > evalExpr mem b
    | GteExpr (a, b) -> evalExpr mem a >= evalExpr mem b
    | LtExpr (a, b) -> evalExpr mem a < evalExpr mem b
    | LteExpr (a, b) -> evalExpr mem a <= evalExpr mem b

let tryRunLabel (mem: InterpreterMemory) (label: Compiler.Label) : InterpreterMemory option =
    try
        match label with
        | Compiler.CommandLabel Skip -> Some mem

        | Compiler.CommandLabel (Assign (Var x, rhs)) ->
            let value = evalExpr mem rhs
            let updatedVars = Map.add x value mem.variables
            Some { mem with variables = updatedVars }

        | Compiler.CommandLabel (Assign (ArrayAccess (name, index), rhs)) ->
            let i = evalExpr mem index |> int
            let value = evalExpr mem rhs
            let arr = Map.find name mem.arrays
            let updatedArr = List.updateAt i value arr
            let updatedArrays = Map.add name updatedArr mem.arrays
            Some { mem with arrays = updatedArrays }

        | Compiler.BoolLabel b -> if evalBool mem b then Some mem else None

        | _ -> None
    with
    | :? System.OverflowException -> None
    | :? System.Collections.Generic.KeyNotFoundException -> None
    | :? System.ArgumentException -> None
    | :? System.DivideByZeroException -> None
    | Failure _ -> None

let chooseEdge
    (determinism: Io.GCL.Determinism)
    (edges: (Compiler.Edge * InterpreterMemory) list)
    : (Compiler.Edge * InterpreterMemory) option =
    match edges with
    | [] -> None
    | _ ->
        match determinism with
        | Io.GCL.Deterministic -> Some(List.head edges)
        | Io.GCL.NonDeterministic ->
            let idx = rng.Next(List.length edges)
            Some(List.item idx edges)

let analysis (input: Input) : Output =
    let deterministic = input.determinism = Io.GCL.Deterministic

    match Parser.parse Grammar.start_command input.commands with
    | Error _ ->
        { initial_node = "qS"
          final_node = "qF"
          dot = ""
          trace = []
          termination = Stuck }

    | Ok ast ->
        let startNode = "qS"
        let finalNode = "qF"
        let allEdges = Compiler.edges deterministic ast startNode finalNode
        let dot = Compiler.printDot allEdges
        let initialMemory = input.assignment
        let maxSteps = int input.trace_length

        let rec walk currentNode mem stepsRemaining trace =
            if currentNode = finalNode then
                List.rev trace, Terminated

            elif stepsRemaining <= 0 then
                List.rev trace, Running

            else
                let executableEdges =
                    allEdges
                    |> List.choose (fun edge ->
                        if edge.source = currentNode then
                            match tryRunLabel mem edge.label with
                            | Some mem' -> Some(edge, mem')
                            | None -> None
                        else
                            None)

                match chooseEdge input.determinism executableEdges with
                | None -> List.rev trace, Stuck

                | Some (edge, mem') ->
                    let step =
                        { action = Compiler.printL edge.label
                          node = edge.target
                          memory = mem' }

                    walk edge.target mem' (stepsRemaining - 1) (step :: trace)

        let trace, termination = walk startNode initialMemory maxSteps []

        { initial_node = startNode
          final_node = finalNode
          dot = dot
          trace = trace
          termination = termination }

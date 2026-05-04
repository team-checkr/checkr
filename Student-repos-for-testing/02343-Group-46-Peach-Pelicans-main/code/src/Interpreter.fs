module Interpreter
open Io.Interpreter


let rec evalExpr (mem: InterpreterMemory) (expr: AST.expr) : int =
    match expr with
    | AST.Num n -> n
    | AST.VarExpr (AST.Var v) -> mem.variables.[v]
    | AST.VarExpr (AST.List (v, idxExpr)) -> 
        let idx = evalExpr mem idxExpr
        mem.arrays.[v].[idx]
    | AST.PlusExpr (x, y) -> evalExpr mem x + evalExpr mem y
    | AST.MinusExpr (x, y) -> evalExpr mem x - evalExpr mem y
    | AST.TimesExpr (x, y) -> evalExpr mem x * evalExpr mem y
    | AST.DivExpr (x, y) -> evalExpr mem x / evalExpr mem y
    | AST.PowExpr (x, y) -> pown (evalExpr mem x) (evalExpr mem y)
    | AST.UMinusExpr x -> - evalExpr mem x


let rec evalBool (mem: InterpreterMemory) (expr: AST.boolexpr) : bool =
    match expr with
    | AST.Bool b -> b
    | AST.And (x, y) -> evalBool mem x && evalBool mem y
    | AST.Or (x, y) -> evalBool mem x || evalBool mem y
    | AST.BitAnd (x, y) -> evalBool mem x && evalBool mem y
    | AST.BitOr (x, y) -> evalBool mem x || evalBool mem y
    | AST.Equal (x, y) -> evalExpr mem x = evalExpr mem y
    | AST.NotEq (x, y) -> evalExpr mem x <> evalExpr mem y
    | AST.SmallerThan (x, y) -> evalExpr mem x < evalExpr mem y
    | AST.SmallerEq (x, y) -> evalExpr mem x <= evalExpr mem y
    | AST.GreaterThan (x, y) -> evalExpr mem x > evalExpr mem y
    | AST.GreaterEq (x, y) -> evalExpr mem x >= evalExpr mem y
    | AST.Not x -> not (evalBool mem x)


let rec exec_steps (edges: List<Compiler.Edge>) (mem: InterpreterMemory) (node: string) (trace_len: int64) : List<Step> * TerminationState = 
    if trace_len <= 0 then [], Running
    elif node = "qF" then [], Terminated 
    else 
        let validEdges = edges |> List.filter (fun e -> 
            e.source = node && 
            match e.label with
            | Compiler.BoolLabel guard -> 
                try evalBool mem guard with _ -> false
            | _ -> true
        )
        
        match validEdges with
        | [] -> [], Stuck // No valid edges
        | edge :: _ -> // pick first valid edge
            try
                // assignment or list assignment
                let nextMem = 
                    match edge.label with
                    | Compiler.CommandLabel (AST.Assign(AST.Var v, expr)) -> 
                        let value = evalExpr mem expr
                        { mem with variables = mem.variables |> Map.add v value }
                    | Compiler.CommandLabel (AST.Assign(AST.List(v, idxExpr), expr)) ->
                        let idx = evalExpr mem idxExpr
                        let value = evalExpr mem expr
                        let newArray = mem.arrays.[v] |> List.mapi (fun i x -> if i = idx then value else x)
                        { mem with arrays = mem.arrays |> Map.add v newArray }
                    | _ -> mem

                let step : Step = {
                    action = Compiler.printL edge.label
                    node = edge.target
                    memory = nextMem
                }
                let futureSteps, finalTerm = exec_steps edges nextMem edge.target (trace_len - 1L)
                step :: futureSteps, finalTerm
            with _ ->
                [], Stuck


let analysis (input: Input) : Output =

    let compiler_out = Compiler.analysis {
        commands = input.commands
        determinism = input.determinism
    }

    let ast = match Compiler.parse Grammar.start_command input.commands with
              | Ok ast -> ast
              | Error e -> failwith "Parse error"

    let edges = fst (Compiler.edges ast "qS" "qF" 0 input.determinism)

    let initial_memory: InterpreterMemory = input.assignment

    let trace, term = exec_steps edges initial_memory "qS" (int64 input.trace_length)

    { initial_node = "qS"
      final_node = "qF"
      dot = compiler_out.dot
      trace = trace
      termination = term }
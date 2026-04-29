module Compiler
open Io.Compiler
open Io.GCL
open FSharp.Text.Lexing
open System
open AST

exception ParseError of Position * string * Exception

let parse parser src =
    let lexbuf = LexBuffer<char>.FromString src

    let parser = parser Lexer.tokenize

    try
        Ok(parser lexbuf)
    with
    | e ->
        let pos = lexbuf.EndPos
        let line = pos.Line
        let column = pos.Column
        let message = e.Message
        let lastToken = new String(lexbuf.Lexeme)
        eprintf "Parse failed at line %d, column %d:\n" line column
        eprintf "Last token: %s" lastToken
        eprintf "\n"
        Error(ParseError(pos, lastToken, e))

type Label = 
    |CommandLabel of commands
    |ExpressionLabel of expr
    |BoolExprLabel of bExpr

type Edge = {
    source: string
    label: Label
    target: string
}

let rec printExpr e =
    match e with
    | Num n        -> string n
    | Var x        -> x
    | PlusExpr (e1, e2) -> "(" + printExpr e1 + " + " + printExpr e2 + ")"
    | MinusExpr (e1, e2) -> "(" + printExpr e1 + " - " + printExpr e2 + ")"
    | TimesExpr (e1, e2) -> "(" + printExpr e1 + " * " + printExpr e2 + ")"
    | DivExpr (e1, e2) -> "(" + printExpr e1 + " / " + printExpr e2 + ")"
    | PowExpr (e1, e2) -> "(" + printExpr e1 + " ^ " + printExpr e2 + ")"
    | UMinusExpr e -> "(-" + printExpr e + ")"
    | Array(a,i) -> a + "[" + printExpr i + "]"

let rec printBExpr b =
    match b with
    | True -> "true"
    | False -> "false"
    | AndExpr (b1, b2) -> "(" + printBExpr b1 + " & " + printBExpr b2 + ")"
    | OrExpr (b1, b2) -> "(" + printBExpr b1 + " | " + printBExpr b2 + ")"
    | AndAndExpr (b1, b2) -> "(" + printBExpr b1 + " && " + printBExpr b2 + ")"
    | OrOrExpr (b1, b2) -> "(" + printBExpr b1 + " || " + printBExpr b2 + ")"
    | NotExpr b -> "!" + printBExpr b
    | EqExpr (e1, e2) -> "(" + printExpr e1 + " = " + printExpr e2 + ")"
    | NeqExpr (e1, e2) -> "(" + printExpr e1 + " != " + printExpr e2 + ")"
    | GtExpr (e1, e2) -> "(" + printExpr e1 + " > " + printExpr e2 + ")"
    | GteExpr (e1, e2) -> "(" + printExpr e1 + " >= " + printExpr e2 + ")"
    | LtExpr (e1, e2) -> "(" + printExpr e1 + " < " + printExpr e2 + ")"
    | LteExpr (e1, e2) -> "(" + printExpr e1 + " <= " + printExpr e2 + ")"

let counter = ref 0

let nextNode () =
    counter := !counter + 1
    "q" + string !counter

let doneCondition (gcs: gCommand list) : bExpr =
    let guards = gcs |> List.map (fun (GCommand(g, _)) -> g)
    let disjunction = List.reduce (fun acc g -> OrExpr(acc, g)) guards
    NotExpr disjunction

let rec edgesGC gcs q1 q2 =
    gcs
    |> List.collect (fun gc ->
        match gc with
        | GCommand (g,c) ->
            let q = nextNode ()
            { source=q1; label=BoolExprLabel g; target=q }
            :: edges c q q2 )

and edges c q1 q2 =
    match c with
    | Skip ->
        [{ source=q1; label=CommandLabel Skip; target=q2 }]

    | Assign(s,e) ->
        [{ source=q1; label=CommandLabel(Assign(s,e)); target=q2 }]

    | ArrayAssign(s,e1,e2) ->
        [{ source=q1; label=CommandLabel(ArrayAssign(s,e1,e2)); target=q2 }]

    | Sequence(c1,c2) ->
        let q = nextNode ()
        edges c1 q1 q @ edges c2 q q2

    | If gcs ->
        edgesGC gcs q1 q2

    | Do gcs ->
        let loopEdges = edgesGC gcs q1 q1
        let exitEdge = [{ source=q1; label=BoolExprLabel(doneCondition gcs); target=q2 }]
        loopEdges @ exitEdge

let rec edgesGC_det gcs q1 q2 =
    let rec go gcs acc =
        match gcs with
        | [] -> ([],acc)
        | GCommand(g, c) :: rest ->
            let guardLabel = AndExpr(g, NotExpr acc)
            let q = nextNode()
            let branchEdges =
                { source=q1; label=BoolExprLabel guardLabel; target=q }
                :: edges_det c q q2
            let newAcc = OrExpr(g, acc)
            let (restEdges, finalAcc) = go rest newAcc
            (branchEdges @ restEdges, finalAcc)
    go gcs False
 
and edges_det c q1 q2 =
    match c with
    | Skip ->
        [{ source=q1; label=CommandLabel Skip; target=q2 }]
 
    | Assign(s,e) ->
        [{ source=q1; label=CommandLabel(Assign(s,e)); target=q2 }]
 
    | ArrayAssign(s,e1,e2) ->
        [{ source=q1; label=CommandLabel(ArrayAssign(s,e1,e2)); target=q2 }]
 
    | Sequence(c1,c2) ->
        let q = nextNode ()
        edges_det c1 q1 q @ edges_det c2 q q2
 
    | If gcs ->
        let (edgeList, _) = edgesGC_det gcs q1 q2
        edgeList
 
    | Do gcs ->
        let (loopEdges, finalAcc) = edgesGC_det gcs q1 q1
        let exitEdge = [{ source=q1; label=BoolExprLabel(NotExpr finalAcc); target =q2}]
        loopEdges @ exitEdge

let printL l =
    match l with
    | CommandLabel Skip -> "skip"
    | CommandLabel (Assign(s, e)) -> s + " := " + (printExpr e)
    | CommandLabel (ArrayAssign(s, e1, e2)) -> s + "[" + (printExpr e1) + "] := " + (printExpr e2)
    | CommandLabel (If _) -> "if"
    | CommandLabel (Do _) -> "do"
    | CommandLabel (Sequence _) -> ";"
    | ExpressionLabel e -> printExpr e
    | BoolExprLabel b -> printBExpr b

let printDotEdges edgeList =
    edgeList
    |> List.map (fun e -> e.source + " -> " + e.target + " [label = \"" + printL e.label + "\"];")
    |> String.concat "\n"

let printDOT edgeList =
    "digraph program_graph {rankdir=LR;\n" + (printDotEdges edgeList) + "\n}"

        
let analysis (input: Input) : Output =
    counter := 0  // reset counter for new analysis
    match parse Grammar.start_commands input.commands with
        | Ok ast ->
            let edgeList =
                match input.determinism with
                | NonDeterministic -> edges     ast "q0" "qF"
                | Deterministic    -> edges_det ast "q0" "qF"
            { dot = printDOT edgeList }
        | Error e -> { dot = "" }
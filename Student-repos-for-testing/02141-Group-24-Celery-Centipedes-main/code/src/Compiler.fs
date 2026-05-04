module Compiler
open Io.Compiler

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
    CommandLabel of command
    | BoolLabel of boolExpr


type Edge = {
    source : string
    label : Label
    target : string
}

let rec done' gc =
    match gc with
    | Arrow (b, _) -> b
    | Choice (gc1, gc2) -> OrExpr(done' gc1, done' gc2)

let rec prependGuard extra gc =
    match gc with
    | Arrow (b, c) -> Arrow (AndExpr(extra, b), c)
    | Choice (gc1, gc2) -> Choice (prependGuard extra gc1, prependGuard extra gc2)

let freshNode n =
    "q" + string n, n + 1

let commandEdge qS c qF =
    { source = qS
      label = CommandLabel(c)
      target = qF }

let boolEdge qS b qF =
    { source = qS
      label = BoolLabel(b)
      target = qF }

type private EdgeWork =
    | CommandWork of command * string * string
    | GuardedWork of guardedCommand * string * string

let edges deterministic c qS qF  =
    let rec buildEdges work n =
        match work with
        | CommandWork (command, source, target) ->
            match command with
            | Skip ->
                [commandEdge source Skip target], n
            | Assign (lhs, rhs) ->
                [commandEdge source (Assign(lhs, rhs)) target], n
            | Sequence (c1, c2) ->
                let qM, n = freshNode n
                let e1, n = buildEdges (CommandWork(c1, source, qM)) n
                let e2, n = buildEdges (CommandWork(c2, qM, target)) n
                e1 @ e2, n
            | If gc ->
                buildEdges (GuardedWork(gc, source, target)) n
            | Do gc ->
                let d = done' gc
                let gcEdges, n = buildEdges (GuardedWork(gc, source, source)) n
                gcEdges @ [boolEdge source (NotExpr d) target], n
        | GuardedWork (gc, source, target) ->
            match gc with
            | Arrow (b, c) ->
                let qM, n = freshNode n
                let cEdges, n = buildEdges (CommandWork(c, qM, target)) n
                boolEdge source b qM :: cEdges, n
            | Choice (gc1, gc2) ->
                let e1, n = buildEdges (GuardedWork(gc1, source, target)) n
                let gc2' = if deterministic then prependGuard (NotExpr(done' gc1)) gc2 else gc2
                let e2, n = buildEdges (GuardedWork(gc2', source, target)) n
                e1 @ e2, n

    let edgeList, _ = buildEdges (CommandWork(c, qS, qF)) 0
    edgeList

let rec printExprL e =
    match e with
    | Num n-> string n
    | Var x -> x
    | ArrayAccess (name, idx) ->
        name + "[" + printExprL idx + "]"
    | PlusExpr (e1: expr, e2) ->
        "(" + printExprL e1 + " + " + printExprL e2 + ")"
    | MinusExpr (e1, e2) ->
        "(" + printExprL e1 + " - " + printExprL e2 + ")"
    | TimesExpr (e1, e2) ->
        "(" + printExprL e1 + " * " + printExprL e2 + ")"
    | DivExpr (e1, e2) ->
        "(" + printExprL e1 + " / " + printExprL e2 + ")"
    | PowExpr (e1, e2) ->
        "(" + printExprL e1 + " ^ " + printExprL e2 + ")"
    | UMinusExpr e ->
        "(-" + printExprL e + ")"

let rec printBoolExprL b =
    match b with
    | True -> "true"
    | False -> "false"
    | AndExpr (b1, b2) ->
        "(" + printBoolExprL b1 + " & " + printBoolExprL b2 + ")"
    | OrExpr (b1, b2) ->
        "(" + printBoolExprL b1 + " | " + printBoolExprL b2 + ")"
    | ShortAndExpr (b1, b2) ->
        "(" + printBoolExprL b1 + " && " + printBoolExprL b2 + ")"
    | ShortOrExpr (b1, b2) ->
        "(" + printBoolExprL b1 + " || " + printBoolExprL b2 + ")"
    | NotExpr b ->
        "!" + printBoolExprL b
    | EqExpr (e1, e2) ->
        "(" + printExprL e1 + " = " + printExprL e2 + ")"
    | NeqExpr (e1, e2) ->
        "(" + printExprL e1 + " != " + printExprL e2 + ")"
    | GtExpr (e1, e2) ->
        "(" + printExprL e1 + " > " + printExprL e2 + ")"
    | GteExpr (e1, e2) ->
        "(" + printExprL e1 + " >= " + printExprL e2 + ")"
    | LtExpr (e1, e2) ->
        "(" + printExprL e1 + " < " + printExprL e2 + ")"
    | LteExpr (e1, e2) ->
        "(" + printExprL e1 + " <= " + printExprL e2 + ")"

let printL (label:Label) =
    match label with
    | CommandLabel Skip -> "skip"
    | CommandLabel (Assign(lhs, rhs)) ->
        printExprL lhs + " := " + printExprL rhs
    | BoolLabel b ->
        printBoolExprL b
    | _ -> ""



let rec printDotEdges e =
    match e with
    | [] -> ""
    | e1 :: rest ->
        e1.source+"->"+e1.target+" [label = \""+(printL e1.label)+"\"];"
        + printDotEdges rest


let printDot e =
    match e with
    | _ -> "digraph program_graph {rankdir=LR;"
            + printDotEdges e
            + "}"


        // "digraph program_graph {rankdir=LR;"
        // +"node [shape = circle]; q;"
        // +"node [shape = doublecircle]; q1;"
        // +"node [shape = circle]"
        // +"q0 -> q1 [label = "skip"];"
        // +"}"

let analysis (input: Input) : Output =
    let deterministic = input.determinism = Io.GCL.Deterministic
    match parse Grammar.start_command input.commands with
        | Ok ast ->
            { dot = printDot (edges deterministic ast "qS" "qF") }
        | Error e -> {dot = ""}

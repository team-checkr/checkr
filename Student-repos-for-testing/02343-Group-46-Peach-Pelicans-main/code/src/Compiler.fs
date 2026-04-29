module Compiler
open Io.Compiler

open FSharp.Text.Lexing
open System
open AST
open Parser
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
    | CommandLabel of command
    | BoolLabel of boolexpr

type Edge = {
    source : string 
    label : Label
    target : string
}

let mkEdge source label target =
    { source = source; label = label; target = target }

let conjunction exprs =
    match exprs with
    | [] -> Bool true
    | head :: tail -> List.fold (fun acc expr -> BitAnd(acc, expr)) head tail

let disjunction exprs =
    match exprs with
    | [] -> Bool false
    | head :: tail -> List.fold (fun acc expr -> BitOr(acc, expr)) head tail

let printL l =
    match l with
    | CommandLabel Skip -> "skip"
    | CommandLabel (Assign(var, exp)) -> prettyPrintVar var + ":=" + prettyPrintExp exp
    | BoolLabel l -> prettyPrintBool l
    | _ -> "TODO"

let rec edges c q1 q2 counter determinism : Edge list * int = 
    match c with
    | Skip -> 
        [mkEdge q1 (CommandLabel Skip) q2], counter
    | Assign(var, exp) -> 
        [mkEdge q1 (CommandLabel (Assign(var, exp))) q2], counter
    | Semi(c1,c2) -> 
        let q = "q" + string counter
        let e1, counter1 = edges c1 q1 q (counter + 1) determinism
        let e2, counter2 = edges c2 q q2 counter1 determinism
        e1 @ e2, counter2
    | If guards -> 
        let allEdges, finalCounter, _ =
            guards |> List.fold (fun (accEdges, c, accGuard) (guard, cmd) ->
                let node = "q" + string c
                let edgeGuard = 
                    if determinism = Io.GCL.Deterministic then BitAnd(guard, Not(accGuard)) else guard
                let condEdge = mkEdge q1 (BoolLabel edgeGuard) node
                let bodyEdges, c' = edges cmd node q2 (c + 1) determinism
                let nextAccGuard = 
                    if determinism = Io.GCL.Deterministic then BitOr(guard, accGuard) else accGuard
                accEdges @ (condEdge :: bodyEdges), c', nextAccGuard
            ) ([], counter + 1, Bool false)
        allEdges, finalCounter
    | Loop guards ->
        let loopEdges, finalCounter, allGuards, finalAccGuard =
            guards |> List.fold (fun (accEdges, c, allGuards, accGuard) (guard, cmd) ->
                let node = "q" + string c
                let edgeGuard = 
                    if determinism = Io.GCL.Deterministic then BitAnd(guard, Not(accGuard)) else guard
                let condEdge = mkEdge q1 (BoolLabel edgeGuard) node
                let bodyEdges, c' = edges cmd node q1 (c + 1) determinism
                let nextAccGuard = 
                    if determinism = Io.GCL.Deterministic then BitOr(guard, accGuard) else accGuard
                accEdges @ (condEdge :: bodyEdges), c', allGuards @ [guard], nextAccGuard
            ) ([], counter + 1, [], Bool false)
        let exitGuard = 
            if determinism = Io.GCL.Deterministic then Not(finalAccGuard) else conjunction (allGuards |> List.map Not)
        loopEdges @ [mkEdge q1 (BoolLabel exitGuard) q2], finalCounter

let rec printDotEdges e =
    match e with
    | [] -> ""
    | e1 :: rest -> e1.source + " -> " + e1.target + " [label=\"" + printL e1.label + "\"];\n" + printDotEdges rest

let printDot e =
    match e with
    | _ -> "digraph program_graph {rankdir=LR;"
            + printDotEdges e
            + "}"

let analysis (input: Input) : Output =
    match parse Grammar.start_command input.commands with
        | Ok ast ->
            { dot = printDot (fst (edges ast "qS" "qF" 0 input.determinism))}
        | Error e -> { dot = "ERROR" }

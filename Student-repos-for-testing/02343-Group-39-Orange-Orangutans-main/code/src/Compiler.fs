module Compiler
open Io.Compiler
open AST
open Parser

//  Types

type Node = string
type Edge = Node * string * Node

//  Label printers

let labelExpr = ppExpr
let labelBexpr = ppBexpr

//  Fresh node generator

let mutable nodeCount = 0
let freshNode () =
    nodeCount <- nodeCount + 1
    sprintf "q%d" nodeCount

//  edges function

let rec edges (det: Io.GCL.Determinism) (cmd: command) (qs: Node) (qe: Node) : Edge list =
    match cmd with
    | Assign(x, e) ->
        [ (qs, sprintf "%s := %s" x (labelExpr e), qe) ]

    | ArrAssign(a, i, e) ->
        [ (qs, sprintf "%s[%s] := %s" a (labelExpr i) (labelExpr e), qe) ]

    | Skip ->
        [ (qs, "skip", qe) ]

    | Seq(c1, c2) ->
        let qm = freshNode ()
        edges det c1 qs qm @ edges det c2 qm qe

    | If(gc) ->
        edgesGC det gc qs qe []

    | Do(gc) ->
        edgesGC det gc qs qs [] @
        doneEdges gc qs qe

and edgesGC (det: Io.GCL.Determinism) (gc: gc) (qs: Node) (qe: Node) (prev: bexpr list) : Edge list =
    match gc with
    | Guard(b, c) ->
        let qm = freshNode ()
        let guardLabel =
            match det with
            | Io.GCL.NonDeterministic -> labelBexpr b
            | Io.GCL.Deterministic ->
                match prev with
                | [] -> labelBexpr b
                | _  ->
                    let negations =
                        prev
                        |> List.map (fun g -> sprintf "(!%s)" (labelBexpr g))
                        |> String.concat " & "
                    sprintf "%s & %s" negations (labelBexpr b)
        (qs, guardLabel, qm) :: edges det c qm qe

    | GCChoice(gc1, gc2) ->
        let gc1Edges  = edgesGC det gc1 qs qe prev
        let gc1Guards = collectGuards gc1
        let gc2Edges  = edgesGC det gc2 qs qe (prev @ gc1Guards)
        gc1Edges @ gc2Edges

and collectGuards (gc: gc) : bexpr list =
    match gc with
    | Guard(b, _)      -> [ b ]
    | GCChoice(g1, g2) -> collectGuards g1 @ collectGuards g2

and doneEdges (gc: gc) (qs: Node) (qe: Node) : Edge list =
    [ (qs, doneCondition gc, qe) ]

and doneCondition (gc: gc) : string =
    match gc with
    | Guard(b, _)      -> sprintf "(!%s)" (labelBexpr b)
    | GCChoice(g1, g2) -> sprintf "%s & %s" (doneCondition g1) (doneCondition g2)

//  DOT printer

let printDot (qs: Node) (qe: Node) (edgeList: Edge list) : string =
    let sb = System.Text.StringBuilder()
    sb.AppendLine("digraph program_graph {") |> ignore
    sb.AppendLine("    rankdir=LR") |> ignore
    sb.AppendLine("    node [shape=circle]") |> ignore
    sb.AppendLine(sprintf "    %s [shape=doublecircle]" qe) |> ignore
    for (q, label, q') in edgeList do
        let safe = label.Replace("\"", "\\\"")
        sb.AppendLine(sprintf "    %s -> %s [label=\"%s\"]" q q' safe) |> ignore
    sb.AppendLine("}") |> ignore
    sb.ToString()

// Analysis starts here

let analysis (input: Input) : Output =
    nodeCount <- 0
    let qs = "q0"
    let qe = "qf"
    match Parser.parse Grammar.start_commands input.commands with
    | Ok ast ->
        let edgeList = edges input.determinism ast qs qe
        { dot = printDot qs qe edgeList }
    | Error e ->
        { dot = sprintf "// Parse error: %A" e }
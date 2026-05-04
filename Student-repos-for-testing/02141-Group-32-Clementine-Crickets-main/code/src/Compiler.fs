module Compiler

open Io.Compiler
open AST
open Parser
open State



type Node = string

type Action =
    | Skip
    | B of booleanExpr
    | ArrayAssignment of (Ident * expr * expr)
    | VariableAssignment of (Ident * expr)

type Edge = Node * Action * Node
type Graph = Edge list

// Lets look at implementing a State Monad instead?
// I THINK WE SUCCEDED?!
let state = State.StatefulBuilder()
let nodeState : State<int, Node> = State (fun i -> (("q" + string i), i+1))
let generateNode : State<int, Node> =
  state {
    return! nodeState
  }


let rec done_gc (gc: guard) : booleanExpr =
    match gc with
    | SequenceGuard (left, right) -> (And(done_gc left, done_gc right))
    | Conditional (b, _) -> Neg(b)


// Non-deterministic approach
let rec non_det_edges (initial: Node) (final: Node) (command: command) : State<int, Graph> =
  state {
    match command with
    | command.Skip -> return [ (initial, Skip, final) ]
    | SequenceCommand (leftCommand, rightCommand) ->
        let! q = generateNode
        let! E1 = non_det_edges initial q leftCommand
        let! E2 = non_det_edges q final rightCommand
        return E1 @ E2 

    | command.VariableAssignment (inner) -> return [ (initial, VariableAssignment(inner), final) ]
    | command.ArrayAssignment (inner) -> return [ (initial, ArrayAssignment(inner), final) ]

    | IfCommand (guard) -> return! non_det_edges_guard initial final guard
    | DoCommand (guard) ->
        let b = done_gc guard

        let! E = non_det_edges_guard initial initial guard
        return E @ [ (initial, B(b), final) ]
  }


and non_det_edges_guard (initial: Node) (final: Node) (guard: guard) : State<int, Graph> =
  state {
    match guard with
    | SequenceGuard (leftGuard, rightGuard) ->
        let! E1 = non_det_edges_guard initial final leftGuard
        let! E2 = non_det_edges_guard initial final rightGuard
        return E1 @ E2

    | Conditional (b, command) ->
        let! q = generateNode
        let! E = non_det_edges q final command
        return [ (initial, B(b), q) ] @ E  
  }


let rec det_edges (initial: Node) (final: Node) (command: command) : State<int, Graph> =
  state {
    match command with
    | command.Skip -> return [ (initial, Skip, final) ]
    | SequenceCommand (leftCommand, rightCommand) ->
        let! q = generateNode

        let! E1 = det_edges initial q leftCommand
        let! E2 = det_edges q final rightCommand
        return E1 @ E2

    | command.VariableAssignment (inner) -> return [ (initial, VariableAssignment(inner), final) ]
    | command.ArrayAssignment (inner) -> return [ (initial, ArrayAssignment(inner), final) ]

    | IfCommand (guard) -> 
                                let! (graph, _) = det_edges_guard initial final guard False
                                return graph
    | DoCommand (guard) ->
        let! (E, d) = det_edges_guard initial initial guard False
        return E @ [(initial, B(Neg(d)), final)]
  }


and det_edges_guard (initial: Node) (final: Node) (guard: guard) (d: booleanExpr) : State<int, (Graph * booleanExpr)> =
  state {
    match guard with
    | Conditional (b, command) ->
        let! q = generateNode
        let! E = det_edges q final command
        return ([(initial, B(And(b, Neg(d))), q)] @ E, Or(b, d))
    | SequenceGuard(left, right) ->
        let! (E1, d1) = det_edges_guard initial final left d
        let! (E2, d2) = det_edges_guard initial final right d1
        return (E1 @ E2, d2)
  }



let stringify_action (action: Action) =
  match action with
  |B(b) -> Parser.printBoolean b
  |Skip -> "skip"
  |VariableAssignment(a) -> 
    let command = (command.VariableAssignment a)
    printCommand command
  |ArrayAssignment(a) -> 
    let command = (command.ArrayAssignment a)
    printCommand command



let dot_edge (edge: Edge) : string =
    let (initial, action, final) = edge

    let label_initial =
        match initial with
        | "qStart" -> "q▷"
        | other -> other

    let label_final =
        match final with
        | "qFinal" -> "q◀"
        | other -> other


    // qStart[label="q▷"]; qStart -> q1[label="a := b"]; q1[label="q1"];  
    let action = stringify_action action
    let result = "  " + initial + "[label=\"" + label_initial + "\"]; " + initial + " -> " + final + "[label=\"" + action + "\"]; " + final + "[label=\"" + label_final + "\"];\n"
    result



let dot (graph: Graph) : string =
    let result =
        "digraph G { \n"
        + (List.fold (fun acc edge -> acc + dot_edge edge) "" graph)
        + "}"

    result



let graph_non_det ast : Graph =
  let s = non_det_edges "qStart" "qFinal" ast
  let (graph, _) = State.run 1 s
  graph

let graph_det ast : Graph =
  let s = det_edges "qStart" "qFinal" ast
  let (graph, _) = State.run 1 s
  graph



let analysis (input: Input) : Output =
    let ast = Parser.parse_string input.commands
    let graph = match input.determinism.IsDeterministic with
                      |true -> graph_det ast
                      |false -> graph_non_det ast
    { dot = dot graph }



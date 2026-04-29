module Interpreter
open Io.Interpreter
open AST
open Compiler

// --> Formal Methods, Chapter 1.2 and Formal Methods, Chapter 2.3
// Chapters 1.1 / 1.2 / 2.2 / 2.3
// Definitions 1.13 1.11 2.17

type Action =
  | ActSkip
  | ActAssign of string * expr
  | ActArrayAssign of string * expr * expr
  | ActGuard of bExpr

let labelToAction (label : Label) : Action option =
  try
    match label with
    | CommandLabel Skip -> Some ActSkip
    | CommandLabel (Assign (s,e)) -> Some (ActAssign(s, e))
    | CommandLabel (ArrayAssign(s, e1, e2)) -> Some (ActArrayAssign (s, e1, e2))
    | BoolExprLabel b -> Some (ActGuard b)
    | _ -> None
  with
  | _ -> None

let rec AExpr e (mem : InterpreterMemory) : int32 =
  match e with
  | Num n        -> int32 n
  | Var x     ->  
      match mem.variables.TryFind x with
            | Some n -> n
            | None -> failwith "Var not found"         
  | PlusExpr (e1, e2) -> (AExpr e1 mem + AExpr e2 mem)  
  | MinusExpr (e1, e2) -> (AExpr e1 mem - AExpr e2 mem)
  | TimesExpr (e1, e2) -> (AExpr e1 mem * AExpr e2 mem)
  | DivExpr (e1, e2) -> (AExpr e1 mem / AExpr e2 mem)
  | PowExpr (e1, e2) -> 
    let b = AExpr e2 mem
    if b < 0 then failwith "Negative power :("
    pown (AExpr e1 mem) (int b)
  | UMinusExpr e -> -(AExpr e mem)
  | Array(a,i) ->
      match mem.arrays.TryFind a with
        | Some array -> 
            let newi = int (AExpr i mem)
            if newi < 0 || newi >= array.Length then failwith "Array index out of bounds"  
            array.[newi]
        | None -> failwith "Array not found" 
     

let rec BExpr b (mem : InterpreterMemory) =
  match b with
  | True  -> true
  | False  -> false
  | AndExpr (b1, b2) -> (BExpr b1 mem) && (BExpr b2 mem)
  | OrExpr (b1, b2) -> (BExpr b1 mem) || (BExpr b2 mem)
  | EqExpr (e1, e2) -> (AExpr e1 mem) = (AExpr e2 mem)
  | NeqExpr (e1, e2) -> (AExpr e1 mem) <> (AExpr e2 mem)
  | LtExpr (e1, e2) -> (AExpr e1 mem) < (AExpr e2 mem)
  | GtExpr (e1, e2) -> (AExpr e1 mem) > (AExpr e2 mem)
  | LteExpr (e1, e2) -> (AExpr e1 mem) <= (AExpr e2 mem)
  | GteExpr (e1, e2) -> (AExpr e1 mem) >= (AExpr e2 mem)
  | NotExpr b -> not (BExpr b mem)
  | AndAndExpr (b1,b2) -> (BExpr b1 mem) && (BExpr b2 mem)
  | OrOrExpr (b1, b2) -> (BExpr b1 mem) || (BExpr b2 mem)

let execAction (action: Action) (mem: InterpreterMemory) :InterpreterMemory option =
  try
    match action with
    | ActSkip -> Some mem
    | ActAssign (x,e) ->
      let v = AExpr e mem
      Some { mem with variables = mem.variables |> Map.add x v}
    | ActArrayAssign (a,iExpr, vExpr) ->
      let i = int (AExpr iExpr mem)
      let v = AExpr vExpr mem
      match mem.arrays.TryFind a with
        | None -> failwith "Array not found"
        | Some array ->
          if (i < 0 || i >= array.Length) then failwith "Index out of bound"
          let newArray= array |> List.mapi (fun idx old -> if idx = i then v else old)
          Some { mem with arrays = mem.arrays |> Map.add a newArray}
    | ActGuard b -> 
        if BExpr b mem
        then Some mem
        else None    
  with
  | :? System.DivideByZeroException -> None
  | :? System.OverflowException -> None
  | _ -> None

let rec prettyAction (action: Action) : string =
   match action with
   | ActSkip -> "skip"
   | ActAssign(var, expr) -> var + " := " + (prettyExpr expr)
   | ActArrayAssign (a, i, e) -> a + "[" + (prettyExpr i) + "] := " + (prettyExpr e)
   | ActGuard b -> prettyBExpr b

and prettyExpr = function    
    | Num n -> string n
    | PlusExpr (e1, e2) -> "(" + prettyExpr e1 + " + " + prettyExpr e2 + ")"
    | MinusExpr (e1, e2) -> "(" + prettyExpr e1 + " - " + prettyExpr e2 + ")"
    | TimesExpr (e1, e2) -> "(" + prettyExpr e1 + " * " + prettyExpr e2 + ")"
    | DivExpr (e1, e2) -> "(" + prettyExpr e1 + " / " + prettyExpr e2 + ")"
    | PowExpr (e1, e2) -> "(" + prettyExpr e1 + " ^ " + prettyExpr e2 + ")"
    | UMinusExpr e -> "(-" + prettyExpr e + ")"
    | Var x -> x
    | Array(a,i) -> a + "[" + prettyExpr i + "]"
  
and prettyBExpr (b: bExpr) =
    match b with
    | True               -> "true"
    | False              -> "false"
    | AndExpr(b1, b2)    -> "(" + prettyBExpr b1 + " & "  + prettyBExpr b2 + ")"
    | OrExpr(b1, b2)     -> "(" + prettyBExpr b1 + " | "  + prettyBExpr b2 + ")"
    | AndAndExpr(b1, b2) -> "(" + prettyBExpr b1 + " && " + prettyBExpr b2 + ")"
    | OrOrExpr(b1, b2)   -> "(" + prettyBExpr b1 + " || " + prettyBExpr b2 + ")"
    | NotExpr b1         -> "(!" + prettyBExpr b1 + ")"
    | EqExpr(a1, a2)     -> "(" + prettyExpr a1 + " = "  + prettyExpr a2 + ")"
    | NeqExpr(a1, a2)    -> "(" + prettyExpr a1 + " != " + prettyExpr a2 + ")"
    | GtExpr(a1, a2)     -> "(" + prettyExpr a1 + " > "  + prettyExpr a2 + ")"
    | GteExpr(a1, a2)    -> "(" + prettyExpr a1 + " >= " + prettyExpr a2 + ")"
    | LtExpr(a1, a2)     -> "(" + prettyExpr a1 + " < "  + prettyExpr a2 + ")"
    | LteExpr(a1, a2)    -> "(" + prettyExpr a1 + " <= " + prettyExpr a2 + ")"


let rng = System.Random(42)  // Seeded for reproducible testing of non-determinism

let tryStep
      (edges: List<Edge>)
      (node: string)
      (mem: InterpreterMemory)
      (det: Io.GCL.Determinism): (string*string*InterpreterMemory) option =

      let outgoing = edges |> List.filter (fun e ->e.source = node)

      let enable =
        outgoing |> List.choose (fun e ->
          match labelToAction e.label with
          | Some action ->
              (match execAction action mem with
              | Some newMem -> Some (prettyAction action, e.target, newMem)
              | None -> None)
          | None -> None)

      match enable with
      | [] -> None
      | transitions -> 
          match det with
          | Io.GCL.Determinism.Deterministic -> Some (List.head transitions)
          | Io.GCL.Determinism.NonDeterministic -> 
              let idx = rng.Next(transitions.Length)
              Some transitions.[idx]


// Need to match to our code
let rec exec_steps 
        (edges: List<Edge>) 
        (finalNode: string)
        (node: string) 
        (mem: InterpreterMemory) 
        (det: Io.GCL.Determinism)
        (trace_length: int): List<Step> * TerminationState = 
  
  if trace_length <= 0 then 
    let termination = if node = finalNode then Terminated else Running
    ([], termination)
  else 
    match tryStep edges node mem det with
      | None ->
        let termination = if node = finalNode then Terminated else Stuck
        ([],termination)

      | Some (actionStr,nextNode, nextMem) ->
            let step = { action = actionStr; node = nextNode ; memory = nextMem }
            let (rest, termination) = 
                exec_steps edges finalNode nextNode nextMem det (trace_length - 1) 
            (step :: rest, termination)

  // match trace_length with
  //   | tl when tl <= 0 -> []
  //   | tl ->
  //     let next_memory={
  //       variables = memory.variables |> Map.add "x" 5
  //       arrays = memory.arrays
  //     }

  //     let step = {
  //       action = "x := 5" // commands?
  //       node = "qF"
  //       memory = next_memory
  //       //memory = input.assignment
  //     }
  //    step :: exec_steps edges "qF" next_memory  (tl-1L) 

let validateMemory (mem: InterpreterMemory) : bool =
  try
    // Validate that variables and arrays maps are properly initialized
    mem.variables |> Map.iter (fun _ v -> ignore v)
    mem.arrays |> Map.iter (fun _ arr -> ignore (arr.Length))
    true
  with
  | _ -> false

let analysis (input: Input) : Output =
  Compiler.counter := 0

  // Validate initial memory
  if not (validateMemory input.assignment) then
    failwith "Invalid initial memory state"

  let ast =
    match Compiler.parse Grammar.start_commands input.commands with
    | Ok a -> a
    | Error _ -> failwith "Parse error"

  // let compiler_output = Compiler.analysis {
  //   commands = input.commands
  //   determinism = input.determinism
  // }

  let edgeList =
    match input.determinism with
    | Io.GCL.Determinism.NonDeterministic -> Compiler.edges ast "q0" "qF"
    | Io.GCL.Determinism.Deterministic -> Compiler.edges_det ast "q0" "qF"

  // let ast = Parser.parse input.commands
  // let edges = Compiler.edges ast input.determinism

  let initialNode = "q0"
  let finalNode = "qF"

  let (trace, termination) =
      exec_steps
        edgeList
        finalNode
        initialNode
        input.assignment
        input.determinism
        input.trace_length

  { initial_node = initialNode
    final_node = finalNode
    dot = Compiler.printDOT edgeList
    trace = trace
    termination = termination }



    // let initial_memory = input.assignment

    // let final_memory={
    //   variables = initial_memory.variables |> Map.add "x" 5
    //   arrays = initial_memory.arrays
    // }

    // let step = {
    //   action = "x := 5" // commands?
    //   node = "qF"
    //   memory = final_memory
    //   //memory = input.assignment
    // }

    // { initial_node = "qS"
    //   final_node = "qF"
    //   dot = compiler_output.dot
    //   trace = exec_steps [] "qS" initial_memory input.trace_length
    //   termination = Terminated }




module Interpreter
open Io.Interpreter
open AST

let step (a: string, node: string, mem) = {
  action = a
  node = node
  memory = mem
}

let outgoing node edges =
    edges |> List.filter (fun (n,_,_) -> n = node)


// I cant beleive no overflowing_add / checked_add exists.. :(
let tryPow (a:int) (b:int) : int option =
    let result = pown (bigint a) b
    if result > bigint System.Int32.MaxValue || result < bigint System.Int32.MinValue then
        None
    else
        Some (int result)

let tryAdd (a: int) (b:int) : int option =
  let result = (bigint a) + (bigint b)
  if result > bigint System.Int32.MaxValue || result < bigint System.Int32.MinValue then
        None
    else
        Some (int result)

let trySub (a: int) (b:int) : int option =
  let result = (bigint a) - (bigint b)
  if result > bigint System.Int32.MaxValue || result < bigint System.Int32.MinValue then
        None
    else
        Some (int result)

let tryMul (a: int) (b:int) : int option =
  let result = (bigint a) * (bigint b)
  if result > bigint System.Int32.MaxValue || result < bigint System.Int32.MinValue then
        None
    else
        Some (int result)




// I got carried away with the refactor -- We love the higher order functions
// A monad is just a monoid in the category of endofunctors

let rec eval_expr (mem: InterpreterMemory) expr: option<int> = 
    match expr with
    | Num n -> Some n
    | Variable x -> Map.tryFind x mem.variables
    | Array (a, i) ->
      // match eval_expr mem i with
      //   | None -> None
      //   | Some index ->
      //       match Map.tryFind a mem.arrays with
      //       | None -> None
      //       | Some arr -> Some arr.[index]
      
      // Maybe this is too hard to read, lets keep old version outcommented for now
      (eval_expr mem i) |>
      Option.bind (fun index -> (Map.tryFind a mem.arrays) |> Option.bind (fun (arr: List<int32>) -> List.tryItem index arr)  ) 

    | UMinusExpr expr -> 
      eval_expr mem expr |>
      Option.bind (fun num -> if num = System.Int32.MinValue then None else Some -num) 

    | PlusExpr (l, r) -> 
      // since tryAdd and friends reuturn option type aswell, we can look at using computation expression if the below is too hard to read
      eval_expr mem l |>
      Option.bind (fun left_evaluated -> (eval_expr mem r) |> Option.bind (fun right_evaluated -> tryAdd left_evaluated right_evaluated)  )

    | MinusExpr (l, r) ->
      eval_expr mem l |>
      Option.bind (fun left_evaluated -> (eval_expr mem r) |> Option.bind (fun right_evaluated -> trySub left_evaluated right_evaluated)  )

    | TimesExpr (l, r) -> 
      eval_expr mem l |>
      Option.bind (fun left_evaluated -> (eval_expr mem r) |> Option.bind (fun right_evaluated -> tryMul left_evaluated right_evaluated)  )

    | DivExpr (l, r) -> 
      eval_expr mem l |>
      Option.bind (fun numerator -> (eval_expr mem r) |> Option.bind (fun denominator -> if denominator = 0 then None else Some (numerator / denominator)))

    | PowExpr (l,r) ->  
      eval_expr mem l |>
      Option.bind (fun z1 -> 
        eval_expr mem r |>
        Option.bind (fun z2 -> 
          if z2 < 0 then None else tryPow z1 z2
        )
      )
    | ParenExpr e -> eval_expr mem e

let rec eval_bool (mem: InterpreterMemory)  (bExpr: booleanExpr) : option<bool> =
  match bExpr with 
  | True -> Some true
  | False -> Some false
  | Neg b -> eval_bool mem b |> Option.map (fun b -> not b)

  | And (b1, b2) -> Option.map2 (&&) (eval_bool mem b1) (eval_bool mem b2) 
  | Or (b1, b2) -> Option.map2 (||) (eval_bool mem b1) (eval_bool mem b2) 

  | Scand (b1, b2) -> 
      let first_eval = eval_bool mem b1
      Option.bind (fun b -> if b then eval_bool mem b2 else Some false) first_eval

  | Scor (b1, b2) -> 
      let first_evaluation = eval_bool mem b1
      Option.bind (fun b -> if not b then eval_bool mem b2 else Some true) first_evaluation

  | Eq (b1, b2) -> Option.map2 (=) (eval_expr mem b1) (eval_expr mem b2)
  | Neq (b1, b2) -> Option.map2 (<>) (eval_expr mem b1) (eval_expr mem b2)

  | Lt (b1, b2) ->  Option.map2 (<) (eval_expr mem b1) (eval_expr mem b2)

  | Gt (b1, b2) ->  Option.map2 (>) (eval_expr mem b1) (eval_expr mem b2)

  | Leq (b1, b2) -> Option.map2 (<=) (eval_expr mem b1) (eval_expr mem b2)

  | Geq (b1, b2) -> Option.map2 (>=) (eval_expr mem b1) (eval_expr mem b2)

  | ParenBool (b) -> eval_bool mem b




let exec_action action memory =
    match action with
    | Compiler.Skip ->
        Some memory

    | Compiler.B b -> match eval_bool memory b with
                                      |Some true -> Some memory
                                      |_ -> None

    | Compiler.VariableAssignment (x,e) -> 
      eval_expr memory e |>
      Option.bind (fun value -> Some { memory with variables = (Map.add x value memory.variables)})
      // match (eval_expr memory e) with
      //                                 |None -> None
      //                                 |Some i -> 
      //                                   let map = Map.add x i memory.variables
      //                                   Some { memory with variables = map }



    | Compiler.ArrayAssignment (ident,i,e) ->
      eval_expr memory i |>
      Option.bind (fun index -> 
        eval_expr memory e |>
        Option.bind (fun value -> 
          Map.tryFind ident memory.arrays |>
          Option.bind (fun list -> 
            // we defininitely need computation expressions, i dont want to add another bind :(
            if index < 0 || index >= list.Length then None else
                let result = List.mapi (fun i elem -> if i = index then value else elem ) list
                Some { memory with arrays = Map.add ident result memory.arrays }
          )
        )
      )

      // match (eval_expr memory i, eval_expr memory e) with
      //   |(None, _) | (_, None) -> None
      //   |(Some index, Some e) ->
      //     match Map.tryFind a memory.arrays with
      //     |None -> None
      //     |Some(l) -> 
      //       let new_array = List.toArray l
      //       new_array.[index] <- e
      //       let arrays = Map.add a (Array.toList new_array) memory.arrays
      //       Some { memory with arrays = arrays }


// TODO: we can make it tail recursive
let rec exec_steps (edges: Compiler.Graph) (node: Compiler.Node) (memory: InterpreterMemory)
  (trace_length: int64): List<Step>  * TerminationState =
    match trace_length with
    | tl when tl <= 0 -> [], Running // Set terminated to Running
    | _ -> 
      let next_edges =
          outgoing node edges

      match next_edges with
        | [] -> [], Terminated //Set termination to Terminated
        | (_, action, next_node) :: _ ->

              // Find the first edge Chat kode
              let possible_edge =
                  next_edges
                  |> List.tryPick (fun (_, action, next_node) ->
                      match exec_action action memory with
                      | Some newMem -> Some (action, next_node, newMem)
                      | None -> None)

              match possible_edge with
                | None -> [], Stuck  // set termination to Stuck
                | Some (action, next_node, newMem) ->
                    // create step for trace
                    let step = step (Compiler.stringify_action action, next_node, newMem)
                    let rest_trace, term = exec_steps edges next_node newMem (trace_length - 1L) // ikke tail recursive im sorry
                    step :: rest_trace, term


let analysis (input: Input) : Output =
    // failwith "Interpreter not yet implemented" 


    let ast = Parser.parse_string input.commands

    
    let graph =
        match input.determinism with
        | Io.GCL.Deterministic ->
          Compiler.graph_det ast
        | Io.GCL.NonDeterministic ->
          Compiler.graph_non_det ast


    let compiler_output = Compiler.analysis {
      commands = input.commands
      determinism = input.determinism
    }

    let trace, termnation = exec_steps graph "qStart" input.assignment (input.trace_length)
  

    { initial_node = "qStart"
      final_node = "qFinal"
      dot = compiler_output.dot
      trace = trace
      termination = termnation }


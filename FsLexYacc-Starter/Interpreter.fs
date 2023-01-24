module Interpreter

open Types

(*
    This defines the input and output for the interpreter. Please do not
    change the definitions below as they are needed for the validation and
    evaluation tools!
*)
type InterpreterMemory =
    { variables: Map<string, int>
      arrays: Map<string, List<int>> }

type Input =
    { determinism: Determinism
      assignment: InterpreterMemory
      trace_size: int }

type ProgramState =
    | Running
    | Stuck
    | Terminated

type ProgramTrace =
    { node: string
      state: ProgramState
      memory: InterpreterMemory }

type Output = List<ProgramTrace>

// Start you implementation here
let analysis (src: string) (input: Input) : Output =
    failwith "Interpreter not yet implemented"

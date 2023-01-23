module Graph

open Types

(*
    This defines the input and output for graphs. Please do not
    change the definitions below as they are needed for the validation and
    evaluation tools!
*)

type Input = { determinism: Determinism }

type Output = { dot: string }

// Start you implementation here
let analysis (src: string) (input: Input) : Output =
    failwith "Graph analysis not yet implemented"

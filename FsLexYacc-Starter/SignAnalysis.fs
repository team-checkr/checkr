module SignAnalysis

open Types

(*
     This defines the input and output of the sign analysis. Please do not
    change the definitions below as they are needed for the validation and
    evaluation tools!
*)

type Sign =
    | Negative
    | Zero
    | Positive

type SignAssignment =
    { variables: Map<string, Sign>
      arrays: Map<string, Set<Sign>> }

type Input =
    { determinism: Determinism
      assignment: SignAssignment }

type Output =
    { initial_node: string
      final_node: string
      nodes: Map<string, Set<SignAssignment>> }


// Start you implementation here
let analysis (src: string) (input: Input) : Output =
    failwith "Sign analysis not yet implemented"

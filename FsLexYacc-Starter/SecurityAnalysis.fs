module Security

open Types

(*
    This defines the input and output for the security analysis. Please do not
    change the definitions below as they are needed for the validation and
    evaluation tools!
*)

type Flow = { from: string; into: string }
let flow a b : Flow = { from = a; into = b }

type Classification =
    { variables: Map<string, string>
      arrays: Map<string, string> }

type SecurityInput =
    { lattice: Flow list
      classification: Classification }

type SecurityOutput =
    { actual: Flow list
      allowed: Flow list
      violations: Flow list }


// Start you implementation here
let security_analysis (src: string) (input: SecurityInput) : SecurityOutput =
    failwith "Security analysis not yet implemented"

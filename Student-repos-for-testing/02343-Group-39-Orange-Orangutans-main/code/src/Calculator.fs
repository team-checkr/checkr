module Calculator
open Io.Calculator

open AST
open System

let rec evaluate (expr: expr) : Result<int, string> =
    // TODO: start here
    match expr with
    | Num n -> Ok (int n)
    | PlusExpr (l, r) ->
        match evaluate l, evaluate r with
        | Ok l, Ok r -> Ok (l + r)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | MinusExpr (l, r) ->
        match evaluate l, evaluate r with
        | Ok l, Ok r -> Ok (l - r)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | TimesExpr (l, r) ->
        match evaluate l, evaluate r with
        | Ok l, Ok r -> Ok (l * r)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | DivExpr (l, r) ->
        match evaluate l, evaluate r with
        | Ok l, Ok r -> if r = 0 then Error "Division by zero" else Ok (l / r)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | PowExpr (l, r) ->
        match evaluate l, evaluate r with
        | Ok l, Ok r -> Ok (int (float l ** float r))
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | UMinusExpr e ->
        match evaluate e with
        | Ok n   -> Ok (-n)
        | Error e -> Error e
    | Var x      -> Error (sprintf "Unbound variable: %s" x)
    | Array _    -> Error "Array indexing not supported in evaluate"

let analysis (input: Input) : Output =
    match Parser.parse Grammar.start_expression input.expression with
    | Ok ast ->
        Console.Error.WriteLine("> {0}", (ast :> obj))
        match evaluate ast with
        | Ok result -> { result = result.ToString(); error = "" }
        | Error e -> { result = ""; error = String.Format("Evaluation error: {0}", e) }
    | Error e -> { result = ""; error = String.Format("Parse error: {0}", e) }

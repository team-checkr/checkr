module Calculator
open Io.Calculator

open AST
open System

let rec evaluate (expr: expr) : Result<int, string> =
    match expr with
    | Num x -> Ok x
    | PlusExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (a + b)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | TimesExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (a * b)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | MinusExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (a - b)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | DivExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok 0 -> Error "Division by zero"
        | Ok a, Ok b -> Ok (a / b)
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | PowExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (int (float a ** float b))
        | Error e, _ -> Error e
        | _, Error e -> Error e
    | UMinusExpr(x) ->
        match evaluate x with
        | Ok a -> Ok(-a)
        | Error e -> Error e
    

     
let analysis (input: Input) : Output =
    match Parser.parse Grammar.start_expression input.expression with
    | Ok ast ->
        Console.Error.WriteLine("> {0}", ast)
        match evaluate ast with
        | Ok result -> { result = result.ToString(); error = "" }
        | Error e -> { result = ""; error = String.Format("Evaluation error: {0}", e) }
    | Error e -> { result = ""; error = String.Format("Parse error: {0}", e) }

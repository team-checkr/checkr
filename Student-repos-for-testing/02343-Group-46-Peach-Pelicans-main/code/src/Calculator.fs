module Calculator
open Io.Calculator

open AST
open System

let rec evaluate (expr: expr) : Result<int, string> =
    match expr with
    | Num n -> Ok n
    | VarExpr _ -> Error "Variables and arrays are not supported by the calculator"
    | PlusExpr(x, y ) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (a + b)
        | _ -> Error "Failed to evaluate PlusExpr"
    | MinusExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (a - b)
        | _ -> Error "Failed to evaluate MinusExpr"
    | TimesExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (a * b)
        | _ -> Error "Failed to evaluate TimesExpr"
    | DivExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b ->
            if b = 0 then Error "Division by zero"
            else Ok (a / b)
        | _ -> Error "Failed to evaluate DivExpr"
    | PowExpr(x, y) ->
        match evaluate x, evaluate y with
        | Ok a, Ok b -> Ok (int (Math.Pow(float a, float b)))
        | _ -> Error "Failed to evaluate PowExpr"
    | UMinusExpr x ->
        match evaluate x with
        | Ok a -> Ok (-a)
        | _ -> Error "Failed to evaluate UMinusExpr"
    | _ -> Error "ERROR"

let analysis (input: Input) : Output =
    match Parser.parse Grammar.start_expression input.expression with
    | Ok ast ->
        Console.Error.WriteLine("> {0}", ast)
        match evaluate ast with
        | Ok result -> { result = result.ToString(); error = "" }
        | Error e -> { result = ""; error = String.Format("Evaluation error: {0}", e) }
    | Error e -> { result = ""; error = String.Format("Parse error: {0}", e) }

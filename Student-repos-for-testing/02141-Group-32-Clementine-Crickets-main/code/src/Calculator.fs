module Calculator
open Io.Calculator

open AST
open System

let rec evaluate (expr: expr) : Result<int, string> =
    match expr with
      |Num(number) -> Ok(number)
      |UMinusExpr(expr) -> match evaluate expr with
                                |Ok(num) -> Ok(-num)
                                |Error(error) -> failwith error
      |DivExpr(numerator, denominator) -> match (evaluate numerator, evaluate denominator) with
                                                        |(Ok(_), Ok(0)) -> failwith "Division by 0!! NO NO NO"
                                                        |(Ok(num), Ok(den)) -> Ok(num / den)
      |TimesExpr(left, right) -> match (evaluate left, evaluate right) with
                                                |(Ok(left), Ok(right)) -> Ok(left * right)
      |PlusExpr(left, right) -> match (evaluate left, evaluate right) with
                                                |(Ok(left), Ok(right)) -> Ok(left + right)
      |MinusExpr(left, right) -> match (evaluate left, evaluate right) with
                                          |(Ok(left), Ok(right)) -> Ok(left - right)
      |PowExpr(left, right) -> match (evaluate left, evaluate right) with
                                          |(Ok(left), Ok(right)) -> Ok(pown left right)      
      | ParenExpr(e) -> match evaluate e with
                              |Ok(r) -> Ok(r)


let analysis (input: Input) : Output =
    match Parser.parse Grammar.start_expression input.expression with
    | Ok ast ->
        Console.Error.WriteLine("> {0}", ast)
        match evaluate ast with
        | Ok result -> { result = result.ToString(); error = "" }
        | Error e -> { result = ""; error = String.Format("Evaluation error: {0}", e) }
    | Error e -> { result = ""; error = String.Format("Parse error: {0}", e) }

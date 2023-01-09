open FSharp.Text.Lexing
open System

open CalculatorTypesAST

open Parser
open Lexer

exception ParseError of Position * string * Exception

let parse src =
    let lexbuf = LexBuffer<char>.FromString src

    let parser = Parser.start Lexer.tokenize

    try
        Ok(parser lexbuf)
    with
    | e ->
        let pos = lexbuf.EndPos
        let line = pos.Line
        let column = pos.Column
        let message = e.Message
        let lastToken = new System.String(lexbuf.Lexeme)
        printf "Parse failed at line %d, column %d:\n" line column
        printf "Last token: %s" lastToken
        printf "\n"
        Error(ParseError(pos, lastToken, e))

let unwrap =
    function
    | Ok r -> r
    | Error e -> raise e

// We define the evaluation function recursively, by induction on the structure
// of arithmetic expressions (AST of type expr)
let rec eval e =
    match e with
    | Num (x) -> x
    | TimesExpr (x, y) -> eval (x) * eval (y)
    | DivExpr (x, y) -> eval (x) / eval (y)
    | PlusExpr (x, y) -> eval (x) + eval (y)
    | MinusExpr (x, y) -> eval (x) - eval (y)
    | PowExpr (x, y) -> eval (x) ** eval (y)
    | UPlusExpr (x) -> eval (x)
    | UMinusExpr (x) -> - eval(x)

// We implement here the function that interacts with the user
let rec compute n =
    if n = 0 then
        printfn "Bye bye"
    else
        printf "Enter an arithmetic expression: "

        try
            // We parse the input string
            let e = parse (Console.ReadLine()) |> unwrap
            // and print the result of evaluating it
            printfn "Result: %f" (eval (e))
            compute n
        with
        | err -> compute (n - 1)

// Console.WriteLine("Hello JSON: {0}", Newtonsoft.Json.JsonConvert.SerializeObject exp)

// Start interacting with the user
compute 3

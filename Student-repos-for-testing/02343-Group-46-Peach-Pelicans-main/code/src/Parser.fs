module Parser
open Io.Parser

open FSharp.Text.Lexing
open System
open AST

exception ParseError of Position * string * Exception


// src "1 + 1" --> Ok (PlusExpr (Num 1, Num 1))
let parse parser src =
    let lexbuf = LexBuffer<char>.FromString src

    let parser = parser Lexer.tokenize

    try
        Ok(parser lexbuf)
    with
    | e ->
        let pos = lexbuf.EndPos
        let line = pos.Line
        let column = pos.Column
        let message = e.Message
        let lastToken = new String(lexbuf.Lexeme)
        eprintf "Parse failed at line %d, column %d:\n" line column
        eprintf "Last token: %s" lastToken
        eprintf "\n"
        Error(ParseError(pos, lastToken, e))

//t takes as input a string, which is intended to describe a GCL program but may contain errors, and build an abstract syntax tree (AST) for it.

let rec prettyPrintExp exp =
    match exp with
    | Num n -> string n
    | VarExpr v -> prettyPrintVar v
    | TimesExpr (e1, e2) -> "(" + prettyPrintExp e1 + " * " + prettyPrintExp e2 + ")"
    | DivExpr (e1, e2) -> "(" + prettyPrintExp e1 + " / " + prettyPrintExp e2 + ")"
    | PlusExpr (e1, e2) -> "(" + prettyPrintExp e1 + " + " + prettyPrintExp e2 + ")"
    | MinusExpr (e1, e2) -> "(" + prettyPrintExp e1 + " - " + prettyPrintExp e2 + ")"
    | PowExpr (e1, e2) -> "(" + prettyPrintExp e1 + " ^ " + prettyPrintExp e2 + ")"
    | UMinusExpr e1 -> "-" + prettyPrintExp e1

// Pretty print boolean expressions
and prettyPrintBool bexp =
    match bexp with
    | Bool b -> if b then "true" else "false"
    | Equal (e1, e2) -> prettyPrintExp e1 + " = " + prettyPrintExp e2
    | NotEq (e1, e2) -> prettyPrintExp e1 + " != " + prettyPrintExp e2
    | SmallerThan (e1, e2) -> prettyPrintExp e1 + " < " + prettyPrintExp e2
    | GreaterThan (e1, e2) -> prettyPrintExp e1 + " > " + prettyPrintExp e2
    | SmallerEq (e1, e2) -> prettyPrintExp e1 + " <= " + prettyPrintExp e2
    | GreaterEq (e1, e2) -> prettyPrintExp e1 + " >= " + prettyPrintExp e2
    | And (b1, b2) -> "(" + prettyPrintBool b1 + " && " + prettyPrintBool b2 + ")"
    | Or (b1, b2) -> "(" + prettyPrintBool b1 + " || " + prettyPrintBool b2 + ")"
    | BitAnd (b1, b2) -> "(" + prettyPrintBool b1 + " & " + prettyPrintBool b2 + ")"
    | BitOr (b1, b2) -> "(" + prettyPrintBool b1 + " | " + prettyPrintBool b2 + ")"
    | Not b1 -> "!(" + prettyPrintBool b1 + ")"


and prettyPrintVar v =  
    match v with
    | Var s -> s
    | List (s, ex) -> s + "[" + prettyPrintExp ex + "]"

let prettyPrintGuardBool bexp =
    match bexp with
    | Equal _ | NotEq _ | SmallerThan _ | GreaterThan _ | SmallerEq _ | GreaterEq _ ->
        "(" + prettyPrintBool bexp + ")"
    | _ -> prettyPrintBool bexp

let rec prettyPrint ast : string =
    let indent level = String.replicate (level * 3) " "

    let rec printCommand level cmd =
        match cmd with
        | Skip -> 
            indent level + "skip"
            
        | Assign (v, exp) ->
            indent level + prettyPrintVar v + " := " + prettyPrintExp exp
            
        | Semi (c1, c2) ->
            printCommand level c1 + " ;\n" + printCommand level c2
            
        | If guards ->
            let printGuard isFirst (bexp, cmd) =
                let prefix = if isFirst then "if " else indent level + "[] "
                indent level + prefix + prettyPrintGuardBool bexp + " ->\n" + printCommand (level + 1) cmd
            let guardLines = guards |> List.mapi (fun i g -> printGuard (i = 0) g)
            String.concat "\n" guardLines + "\n" + indent level + "fi"
            
        | Loop guards ->
            let printGuard isFirst (bexp, cmd) =
                let prefix = if isFirst then "do " else indent level + "[] "
                indent level + prefix + prettyPrintGuardBool bexp + " ->\n" + printCommand (level + 1) cmd
            let guardLines = guards |> List.mapi (fun i g -> printGuard (i = 0) g)
            String.concat "\n" guardLines + "\n" + indent level + "od"
            
            
    printCommand 0 ast


let analysis (input: Input) : Output =
    match parse Grammar.start_command input.commands with
        | Ok ast ->
            Console.Error.WriteLine("> {0}", ast)
            { pretty = prettyPrint ast }
        | Error e -> { pretty = String.Format("Parse error: {0}", e) }

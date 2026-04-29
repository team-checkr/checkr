module Parser
open Io.Parser

open FSharp.Text.Lexing
open System
open AST

exception ParseError of Position * string * Exception

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

let rec printExpr expr : string =
    match expr with
    | Num(number) -> string number
    | Variable(var) -> var
    | Array(ident, expr) -> ident + "[" + (printExpr expr) + "]"    
    | UMinusExpr(number) -> "-" + (printExpr number)
    | TimesExpr(left, right) -> printExpr left + " * " + printExpr right
    | DivExpr(left, right) -> printExpr left + " / " + printExpr right
    | PlusExpr(left, right) -> printExpr left + " + " + printExpr right
    | MinusExpr(left, right) -> printExpr left + " - " + printExpr right
    | PowExpr(left, right) -> printExpr left + " ^ " + printExpr right
    | ParenExpr(expr) -> "(" + printExpr expr + ")"
    
let boolPrec (boolean: booleanExpr ): int =
  match boolean with  
  | True | False -> 5
  | Neg(_) -> 4
  | And(_) | Scand(_) -> 3
  | Eq(_) | Neq(_) |Lt(_) | Gt(_) |Leq(_) | Geq(_) | Or(_) | Scor(_) -> 2  
  | ParenBool(_) -> failwith "Parenbool isParenbool is handled before"


let rec printBooleanPrec (boolean: booleanExpr) (parentPrec: int) : string =
  // first if we encounter ParenBool simply ad paranthesis and call this function with prec = 0
  match boolean with
        |ParenBool(b) -> "(" + printBooleanPrec b 0 + ")"
        |_ -> 
  let prec = boolPrec boolean
  let result = match boolean with  
                | True -> "true"
                | False -> "false"
                | Neg(b) -> "!" + printBooleanPrec b prec
                | And(l, r) -> printBooleanPrec l prec + " & " + printBooleanPrec r prec
                | Or(l, r) -> printBooleanPrec l prec + " | " + printBooleanPrec r prec
                | Scand(l, r) -> printBooleanPrec l prec + " && " + printBooleanPrec r prec
                | Scor(l, r) -> printBooleanPrec l prec + " || " + printBooleanPrec r prec
                | Eq(l, r) ->  printExpr l + " = " + printExpr r
                | Neq(l, r) -> printExpr l + " != " + printExpr r
                | Lt(l, r) -> printExpr l + " < " + printExpr r                
                | Gt(l, r) -> printExpr l + " > " + printExpr r
                | Leq(l, r) -> printExpr l + " <= " + printExpr r
                | Geq(l, r) -> printExpr l + " >= " + printExpr r
                | ParenBool(_) -> failwith "UNREACHABLE, if not parenbool is fucked"

  if prec < parentPrec then "(" + result + ")"
  else result


let printBoolean boolean =
  printBooleanPrec boolean 0
  

let printCommand command : string = 
  match command with
  |VariableAssignment(ident, expr) -> ident + " := " + (printExpr expr)
  |ArrayAssignment(ident, left, right) -> ident + "[" + (printExpr left) + "]" + " := " + (printExpr right)
  |Skip -> "skip"  
  |_ -> failwith "Unreachable"

let rec identCommand command prefix =
  // If GC FI -- send prefix to gc 
  // GC [] GC -- no extra prefix here 
  // b -> C -- send prefix to C
  match command with
                |IfCommand guard -> prefix + "if " + identGuard guard prefix + "\n" + prefix + "fi"
                |DoCommand guard -> prefix + "do " + identGuard guard prefix + "\n" + prefix + "od"
                |SequenceCommand(first, second) -> identCommand first prefix + " ;\n" + identCommand second prefix
                |_ -> prefix + printCommand command

and identGuard guard prefix =
  match guard with
              |Conditional(b, c) -> printBoolean b + " -> \n" + identCommand c (prefix + "\t")
              |SequenceGuard(first, second) -> identGuard first prefix + "\n" + prefix + "[] " + identGuard second prefix


let rec prettyPrint ast : string =
   // TODO: start here
   identCommand ast ""

let parse_string (input: string) : command =
  match parse Grammar.start_command input with
                      | Ok ast -> ast
                      | Error e ->  failwith (String.Format("Parse error: {0}", e))

let analysis (input: Input) : Output =
    // TODO: change start_expression to start_commands
    match parse Grammar.start_command input.commands with
        | Ok ast ->
            Console.Error.WriteLine("> {0}", ast)
            { pretty = prettyPrint ast }
        | Error e -> { pretty = String.Format("Parse error: {0}", e) }



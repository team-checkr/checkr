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

let rec prettyPrintExpr (expr: expr) =
    match expr with
    | Num n -> string n
    | PlusExpr (e1, e2) -> "(" + prettyPrintExpr e1 + " + " + prettyPrintExpr e2 + ")"
    | MinusExpr (e1, e2) -> "(" + prettyPrintExpr e1 + " - " + prettyPrintExpr e2 + ")"
    | TimesExpr (e1, e2) -> "(" + prettyPrintExpr e1 + " * " + prettyPrintExpr e2 + ")"
    | DivExpr (e1, e2) -> "(" + prettyPrintExpr e1 + " / " + prettyPrintExpr e2 + ")"
    | PowExpr (e1, e2) -> "(" + prettyPrintExpr e1 + " ^ " + prettyPrintExpr e2 + ")"
    | UMinusExpr e -> "-" + prettyPrintExpr e
    | Var x -> x
    | Array(a,i) -> a + "[" + prettyPrintExpr i + "]"

let rec prettyPrint ast : string =
   // TODO: start here

   match ast with
   | Skip -> "skip"
   | Sequence(c1, c2) -> (prettyPrint c1) + " ;\n" + (prettyPrint c2)
   | Assign(var, expr) -> var + " := " + (prettyPrintExpr expr)
   | ArrayAssign (a, i, e) -> a + "[" + (prettyPrintExpr i) + "] := " + (prettyPrintExpr e)
   | If(gcs) -> "if " + (prettyPrintGCs gcs) + "\nfi"
   | Do(gcs) -> "do " + (prettyPrintGCs gcs) + "\nod"

and prettyPrintGCs gcs =
    gcs 
    |> List.map (fun (GCommand(b, c)) -> 
        (prettyPrintBExpr b) + " ->\n   " + 
        (prettyPrint c |> fun s -> s.Replace("\n", "\n   ")))
    |> String.concat "\n[] "
    //|> fun s -> s + "\n" 
and prettyPrintBExpr (b: bExpr) =
    match b with
    | True               -> "true"
    | False              -> "false"
    | AndExpr(b1, b2)    -> "(" + prettyPrintBExpr b1 + " & "  + prettyPrintBExpr b2 + ")"
    | OrExpr(b1, b2)     -> "(" + prettyPrintBExpr b1 + " | "  + prettyPrintBExpr b2 + ")"
    | AndAndExpr(b1, b2) -> "(" + prettyPrintBExpr b1 + " && " + prettyPrintBExpr b2 + ")"
    | OrOrExpr(b1, b2)   -> "(" + prettyPrintBExpr b1 + " || " + prettyPrintBExpr b2 + ")"
    | NotExpr b1                ->  "!" + prettyPrintBExpr b1
    | EqExpr(a1, a2)     -> "(" + prettyPrintExpr a1 + " = "  + prettyPrintExpr a2 + ")"
    | NeqExpr(a1, a2)    -> "(" + prettyPrintExpr a1 + " != " + prettyPrintExpr a2 + ")"
    | GtExpr(a1, a2)     -> "(" + prettyPrintExpr a1 + " > "  + prettyPrintExpr a2 + ")"
    | GteExpr(a1, a2)    -> "(" + prettyPrintExpr a1 + " >= " + prettyPrintExpr a2 + ")"
    | LtExpr(a1, a2)     -> "(" + prettyPrintExpr a1 + " < "  + prettyPrintExpr a2 + ")"
    | LteExpr(a1, a2)    -> "(" + prettyPrintExpr a1 + " <= " + prettyPrintExpr a2 + ")"

let analysis (input: Input) : Output =
    // TODO: change start_expression to start_commands
    match parse Grammar.start_commands input.commands with
        | Ok ast ->
            Console.Error.WriteLine("> {0}", ast)
            { pretty = prettyPrint ast }
        | Error e -> { pretty = String.Format("Parse error: {0}", e) }

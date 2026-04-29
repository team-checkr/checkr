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

let rec prettyPrint ast : string =
   // TODO: start here
    match ast with
    | Assign(x, e)          -> sprintf "%s := %s" x (ppExpr e)
    | ArrAssign(a, i, e)    -> sprintf "%s[%s] := %s" a (ppExpr i) (ppExpr e)
    | Skip                  -> "skip"
    | Seq(c1, c2)           -> sprintf "%s ;\n%s" (prettyPrint c1) (prettyPrint c2)
    | If(gc)                -> sprintf "if %s\nfi" (ppGC gc)
    | Do(gc)                -> sprintf "do %s\nod" (ppGC gc)

and ppGC (gc: gc) : string =
    match gc with
    | Guard(b, c)      -> sprintf "%s -> %s" (ppBexpr b) (prettyPrint c)
    | GCChoice(g1, g2) -> sprintf "%s\n[] %s" (ppGC g1) (ppGC g2)

and ppBexpr (b: bexpr) : string =
    match b with
    | True               -> "true"
    | False              -> "false"
    | AndExpr(l, r)      -> sprintf "(%s & %s)"  (ppBexpr l) (ppBexpr r)
    | OrExpr(l, r)       -> sprintf "(%s | %s)"  (ppBexpr l) (ppBexpr r)
    | AndAndExpr(l, r)   -> sprintf "(%s && %s)" (ppBexpr l) (ppBexpr r)
    | OrOrExpr(l, r)     -> sprintf "(%s || %s)" (ppBexpr l) (ppBexpr r)
    | NotExpr b          -> sprintf "(!%s)"       (ppBexpr b)
    | EqExpr(l, r)       -> sprintf "(%s = %s)"  (ppExpr l) (ppExpr r)
    | NeqExpr(l, r)      -> sprintf "(%s != %s)" (ppExpr l) (ppExpr r)
    | GtExpr(l, r)       -> sprintf "(%s > %s)"  (ppExpr l) (ppExpr r)
    | GteExpr(l, r)      -> sprintf "(%s >= %s)" (ppExpr l) (ppExpr r)
    | LtExpr(l, r)       -> sprintf "(%s < %s)"  (ppExpr l) (ppExpr r)
    | LteExpr(l, r)      -> sprintf "(%s <= %s)" (ppExpr l) (ppExpr r)

and ppExpr (e: expr) : string =
    match e with
    | Num n              -> string n
    | Var x              -> x
    | Array(a, i)        -> sprintf "%s[%s]" a (ppExpr i)
    | PlusExpr(l, r)     -> sprintf "(%s + %s)"  (ppExpr l) (ppExpr r)
    | MinusExpr(l, r)    -> sprintf "(%s - %s)"  (ppExpr l) (ppExpr r)
    | TimesExpr(l, r)    -> sprintf "(%s * %s)"  (ppExpr l) (ppExpr r)
    | DivExpr(l, r)      -> sprintf "(%s / %s)"  (ppExpr l) (ppExpr r)
    | PowExpr(l, r)      -> sprintf "(%s ^ %s)"  (ppExpr l) (ppExpr r)
    | UMinusExpr e       -> sprintf "(-%s)"       (ppExpr e)

let analysis (input: Input) : Output =
    // TODO: change start_expression to start_commands
    match parse Grammar.start_commands input.commands with
        | Ok ast ->
            Console.Error.WriteLine("> {0}", (ast :> obj))
            { pretty = prettyPrint ast }
        | Error e -> { pretty = String.Format("Parse error: {0}", e) }

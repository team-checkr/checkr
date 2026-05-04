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

let prettyPrint (ast: ASTNode) : string =
    let rec pp (depth: int) (ast: ASTNode) : string =
        let tab = String.replicate depth "   "
        match ast with
        | C Skip                        -> tab + "skip"
        | C(Sequence(c1, c2))           -> pp depth (C c1) + " ;\n" + pp depth (C c2)
        | C(Assign(x, a))               -> tab + pp 0 (E x) + " := " + pp 0 (E a)
        | C(If gc)                      -> tab + "if " + pp depth (GC gc) + "\n" + tab + "fi"
        | C(Do gc)                      -> tab + "do " + pp depth (GC gc) + "\n" + tab + "od"
        
        | GC(Arrow(b, c))               -> pp 0 (B b) + " ->\n" + pp (depth + 1) (C c)
        | GC(Choice(gc1, gc2))          ->
            let rest = pp depth (GC gc2)
            let trimmed = if rest.StartsWith(tab) then rest.Substring(tab.Length) else rest
            pp depth (GC gc1) + "\n" + tab + "[] " + trimmed
        
        | B True                        -> "true"
        | B False                       -> "false"
        
        | B(AndExpr(b1, b2))            -> "(" + pp 0 (B b1) + " & " + pp 0 (B b2) + ")"
        | B(OrExpr(b1, b2))             -> "(" + pp 0 (B b1) + " | " + pp 0 (B b2) + ")"
        | B(ShortAndExpr(b1, b2))       -> "(" + pp 0 (B b1) + " && " + pp 0 (B b2) + ")"
        | B(ShortOrExpr(b1, b2))        -> "(" + pp 0 (B b1) + " || " + pp 0 (B b2) + ")"
        
        | B(NotExpr inner) -> "!" + pp 0 (B inner)
        
        | B(EqExpr(a1, a2))             -> "(" + pp 0 (E a1) + " = " + pp 0 (E a2) + ")"
        | B(NeqExpr(a1, a2))            -> "(" + pp 0 (E a1) + " != " + pp 0 (E a2) + ")"
        | B(GtExpr(a1, a2))             -> "(" + pp 0 (E a1) + " > " + pp 0 (E a2) + ")"
        | B(GteExpr(a1, a2))            -> "(" + pp 0 (E a1) + " >= " + pp 0 (E a2) + ")"
        | B(LtExpr(a1, a2))             -> "(" + pp 0 (E a1) + " < " + pp 0 (E a2) + ")"
        | B(LteExpr(a1, a2))            -> "(" + pp 0 (E a1) + " <= " + pp 0 (E a2) + ")"
        
        | E(Num n)                      -> string n
        | E(Var x)                      -> x
        | E(ArrayAccess(a, idx))        -> a + "[" + pp 0 (E idx) + "]"
        | E(TimesExpr(a1, a2))          -> "(" + pp 0 (E a1) + " * " + pp 0 (E a2) + ")"
        | E(DivExpr(a1, a2))            -> "(" + pp 0 (E a1) + " / " + pp 0 (E a2) + ")"
        | E(PlusExpr(a1, a2))           -> "(" + pp 0 (E a1) + " + " + pp 0 (E a2) + ")"
        | E(MinusExpr(a1, a2))          -> "(" + pp 0 (E a1) + " - " + pp 0 (E a2) + ")"
        | E(PowExpr(a1, a2))            -> "(" + pp 0 (E a1) + " ^ " + pp 0 (E a2) + ")"
        | E(UMinusExpr a)               -> "-" + pp 0 (E a)
    pp 0 ast



let analysis (input: Input) : Output =
    match parse Grammar.start_command input.commands with
        | Ok ast ->
            Console.Error.WriteLine("> {0}", ast)
            { pretty = prettyPrint (C ast) }
        | Error e -> { pretty = String.Format("Parse error: {0}", e) }

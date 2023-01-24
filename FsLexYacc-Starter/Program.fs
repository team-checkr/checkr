open FSharp.Text.Lexing
open System
open Newtonsoft.Json
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

let rec evaluate: expr -> float =
    function
    | Num x -> x
    | TimesExpr (a, b) -> evaluate a * evaluate b
    | DivExpr (a, b) -> evaluate a / evaluate b
    | PlusExpr (a, b) -> evaluate a + evaluate b
    | MinusExpr (a, b) -> evaluate a - evaluate b
    | PowExpr (a, b) -> evaluate a ** evaluate b
    | UPlusExpr a -> evaluate a
    | UMinusExpr a -> -evaluate a

// Please do not change the main function, with exception to the "calc" case.
// The other cases are needed for the validation and evaluation tools!

[<EntryPoint>]
let main (args) =
    match args |> List.ofArray with
    | "calc" :: input ->
        match parse Parser.start (String.concat " " input) with
        | Ok ast ->
            Console.Error.WriteLine("> {0}", ast)
            Console.WriteLine("{0}", evaluate ast)
        | Error e -> Console.Error.WriteLine("Parse error: {0}", e)

        0
    | [ "graph"; src; input ] ->
        let input = JsonConvert.DeserializeObject<Graph.Input> input
        let output: Graph.Output = Graph.analysis src input
        Console.WriteLine("{0}", JsonConvert.SerializeObject output)

        0
    | [ "interpreter"; src; input ] ->
        let input = JsonConvert.DeserializeObject<Interpreter.Input> input
        let output: Interpreter.Output = Interpreter.analysis src input
        Console.WriteLine("{0}", JsonConvert.SerializeObject output)

        0
    | [ "program-verification"; src; input ] ->
        let input = JsonConvert.DeserializeObject<ProgramVerification.Input> input
        let output: ProgramVerification.Output = ProgramVerification.analysis src input
        Console.WriteLine("{0}", JsonConvert.SerializeObject output)

        0
    | [ "sign"; src; input ] ->
        let input = JsonConvert.DeserializeObject<SignAnalysis.Input> input
        let output: SignAnalysis.Output = SignAnalysis.analysis src input
        Console.WriteLine("{0}", JsonConvert.SerializeObject output)

        0
    | [ "security"; src; input ] ->
        let input = JsonConvert.DeserializeObject<Security.Input> input
        let output: Security.Output = Security.analysis src input
        Console.WriteLine("{0}", JsonConvert.SerializeObject output)

        0
    | _ ->
        let commands =
            [ "calc <EXPRESSION...>"
              "graph <SRC> <INPUT>"
              "interpreter <SRC> <INPUT>"
              "pv <SRC> <INPUT>"
              "sign <SRC> <INPUT>"
              "security <SRC> <INPUT>" ]

        Console.Error.WriteLine(
            "\x1B[1;31merror:\x1B[0m unrecognized input: \x1B[0;33m'{0}'\x1B[0m\n\n{1}\n\nAvailable commands:\n{2}",
            String.concat " " args,
            "\x1B[1mUsage: dotnet run\x1B[0m <COMMAND>",
            (List.fold (fun acc cmd -> acc + sprintf " - \x1B[1m%s\x1B[0m\n" cmd) "" commands)
        )

        1

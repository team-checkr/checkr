open System
open System.Text.Json
#nowarn "3261"

// Please do not change the main function.
// The cases are needed for the validation and evaluation tools!

[<EntryPoint>]
let main (args) =
    match args |> List.ofArray with
    | ["Calculator"; input] ->
        let input = JsonSerializer.Deserialize<Io.Calculator.Input> input
        let output: Io.Calculator.Output = Calculator.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | ["Parser"; input ] ->
        let input = JsonSerializer.Deserialize<Io.Parser.Input> input
        let output: Io.Parser.Output = Parser.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | [ "Compiler"; input ] ->
        let input = JsonSerializer.Deserialize<Io.Compiler.Input> input
        let output: Io.Compiler.Output = Compiler.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | [ "Interpreter"; input ] ->
        let input = JsonSerializer.Deserialize<Io.Interpreter.Input> input
        let output: Io.Interpreter.Output = Interpreter.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | [ "BiGCL"; input ] ->
        let input = JsonSerializer.Deserialize<Io.BiGCL.Input> input
        let output: Io.BiGCL.Output = BiGCL.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | [ "RiscV"; input ] ->
        let input = JsonSerializer.Deserialize<Io.RiscV.Input> input
        let output: Io.RiscV.Output = RiscV.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | [ "Sign"; input ] ->
        let input = JsonSerializer.Deserialize<Io.SignAnalysis.Input> input
        let output: Io.SignAnalysis.Output = SignAnalysis.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | [ "Security"; input ] ->
        let input = JsonSerializer.Deserialize<Io.SecurityAnalysis.Input> input
        let output: Io.SecurityAnalysis.Output = SecurityAnalysis.analysis input
        Console.WriteLine("{0}", JsonSerializer.Serialize output)

        0
    | _ ->
        let commands =
            [ "Calculator <INPUT>"
              "Parser <INPUT>"
              "Compiler <INPUT>"
              "Interpreter <INPUT>"
              "BiGCL <INPUT>"
              "RiscV <INPUT>"
              "Sign <INPUT>"
              "Security <INPUT>" ]

        Console.Error.WriteLine(
            "\x1B[1;31merror:\x1B[0m unrecognized input: \x1B[0;33m'{0}'\x1B[0m\n\n{1}\n\nAvailable commands:\n{2}",
            String.concat " " args,
            "\x1B[1mUsage: dotnet run\x1B[0m <COMMAND>",
            (List.fold (fun acc cmd -> acc + sprintf " - \x1B[1m%s\x1B[0m\n" cmd) "" commands)
        )

        1

This folder contains examples of "calculator" programs in F# and Java tha read an arithmetic expression from the command line and print the result of evaluating such expression.

## Files

F#/FsLexYacc
* [Lexer.fsl](Lexer.fsl): The F# lexer for arithmetic expressions
* [Parser.fsp](Parser.fsp): The F# parser for arithmetic expressions
* [CalculatorTypesAST.fs](CalculatorTypesAST.fs): F# types for AST of arithmetic expressions
* [Program.fs](Program.fs): The F# script for the calculator

## Running on macOS M1

Building on macOS requires the `dotnet-sdk` package. This can be installed using [brew](https://brew.sh):

```bash
brew install dotnet-sdk
```

## Instructions for F# #/FSLexYacc

To run the program do:

```bash
dotnet run
```

You should be able to interact with the calculator program as follows:

```
Enter an arithmetic expression: 1
1.0
Enter an arithmetic expression: 1 + 2
3.0
Enter an arithmetic expression: 1 + 2 * 3
7.0
```

# FsLexYacc-Starter

This folder contains the skeleton of a parser along with the input and output types for each analysis given in the assignment. It also contain an example of a "calculator" program in F# that reads an arithmetic expression from the command line and print the result of evaluating such expression for initial testing.

## Files

F#/FsLexYacc
* [Lexer.fsl](Lexer.fsl): The lexer for arithmetic expressions
* [Parser.fsp](Parser.fsp): The parser for arithmetic expressions
* [Types.fs](Types.fs): Global types that are used in many analysis
* [AST.fs](AST.fs): Types for AST of arithmetic expressions
* [Program.fs](Program.fs): The entrypoint for the program
* [Security.fs](Security.fs): File for the security analysis
* [SignAnalysis.fs](SignAnalysis.fs): File for the sign analysis
* [ProgramVerification.fs](ProgramVerification.fs): File for program verification
* [Graph.fs](Graph.fs): File for graphs
* [Interpreter.fs](Interpreter.fs): File for the interpreter


## Running on macOS M1

Building on macOS requires the `dotnet-sdk` package. This can be installed using [brew](https://brew.sh):

```bash
brew install dotnet-sdk
```

## Instructions for F#/FSLexYacc

To run the program do:

```bash
dotnet run
```

### Calculator

To run the calculator do:

```bash
dotnet run calc "1 + 52 * 23"
```

## Interactive UI

The analysis can be explored in the interactive tool. Run the program in `dev/` folder matching you operating system.

```bash
# Windows
./dev/win.exe --open

# macOS
./dev/macos --open

# linux
./dev/linux --open
```

With the `--open` flag this should open the tool at `http://localhost:3000/` in your browser.

The tool knows how to compile your program by the instructions in `run.toml`.

## Evaluation

Every time you push to git, the program gets evaluated automatically.

The result can be seen at GitLab in the `result` branch.

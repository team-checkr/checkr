This folder contains examples of "calculator" programs in F# and Java tha read an arithmetic expression from the command line and print the result of evaluating such expression.

## Files

F#/FsLexYacc
* [Lexer.fsl](Lexer.fsl): The F# lexer for arithmetic expressions
* [Parser.fsp](Parser.fsp): The F# parser for arithmetic expressions
* [CalculatorTypesAST.fs](CalculatorTypesAST.fs): F# types for AST of arithmetic expressions
* [Program.fs](Program.fs): The F# script for the calculator

## Running in VSCode and dev continer

The repository contains a setup for a [dev container](https://code.visualstudio.com/docs/remote/create-dev-container).
For this [Docker](https://www.docker.com/), [VSCode](https://code.visualstudio.com/) and the
[Remote - Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
extension is required.

To install docker on macOS use:

```
$ brew install --cask docker
```

Now open the project in VSCode, run (`CMD+Shift+P`) the command:

```
Remote-Containers: Rebuild and Reopen in Container
```

After this, you VSCode window will have all of the required extensions and settings applied, and the compiler can be built using the internal terminal. To run:

```bash
$ dotnet run
```

## Instructions for F# #/FSLexYacc

To run the program do:

```
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

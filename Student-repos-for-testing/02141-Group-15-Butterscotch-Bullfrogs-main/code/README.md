# GCL – F# Starter

This folder contains the skeleton of a parser along with the input and output types for each analysis given in the assignment.

## Files

- [Types.fs](src/Types.fs): Global types that are used in many analyses.
- [AST.fs](src/AST.fs): Types for the Abstract Syntax Tree (AST) of arithmetic expressions.
- [Parser.fsy](src/Parser.fsy): The parser for arithmetic expressions.
- [Lexer.fsl](src/Lexer.fsl): The lexer for arithmetic expressions.
- [Calculator.fs](src/Calculator.fs): Contains the code for the basic calculator.
- [Compiler.fs](src/Compiler.fs): File for the compiler.
- [BiGCL.fs](src/BiGCL.fs): File for the BiGCL transpiler.
- [RiscV.fs](src/RiscV.fs): File for the GCL to RISC-V compiler.
- [SignAnalysis.fs](src/SignAnalysis.fs): File for the sign analysis.
- [Security.fs](src/Security.fs): File for the security analysis.
- [Program.fs](src/Program.fs): The entry point for the program.

## Installation

Building this project requires .NET 9.0. For installation, follow the description matching your platform:

- **Windows:** Installation instructions for this, can be found [here](https://dotnet.microsoft.com/en-us/download).
- **macOS:** Building on macOS requires the `dotnet-sdk` package. This can be installed using [Homebrew](https://brew.sh) and running `brew install dotnet-sdk`.
- **Linux:** There are many ways to install on Linux, but a good starting point might be [this](https://fsharp.org/use/linux/).

To check that you have an up-to-date version run `dotnet --version` to display the version number, which should be something starting with 9. If it does not, consider updating your installation, and if that doesn't work, try uninstalling your current version and installing from scratch.

The next step is getting the code, which is done by cloning this repository and using `cd` to change directory to the newly cloned folder. To do this, make sure that you have your SSH keys set up correctly (instructions for [GitLab](https://docs.gitlab.com/ee/user/ssh.html)).

## Running the code

The primary way to interact with your code is through Inspectify, to run this simply run:

```bash
# On Windows
inspectify.ps1 --open
# On macOS and Linux
./inspectify.sh --open
```

With the `--open` flag this should open the tool at `http://localhost:3000/` in your browser.

The tool knows how to compile your program by the instructions in `run.toml`.

When ever you make changes to your code, it should automatically be recompiled and any analysis will be rerun in Inspectify.

> **Note:** The first time you try to run `inspectify` might take a few minutes because the script will download various dependencies. Depending on your system, you might also have to restart your computer after successfully running the script for the first time.

For most tasks, there is a reference implementation that you can use (by checking the corresponding button) for comparison.

## First steps

Open the repository in your code editor, and navitage to [`Calculator.fs`](Calculator.fs). This file contains the starting point for implementing the simple arithmetic calculator.

The first place to look is at the `analysis` function:

```fs
let analysis (input: Input) : Output =
    ...
```

This function parses the expression given from the input, and attempts to `evaluate` the `ast` if parsing succeeded.

The next step, is to look at:

```fs
let rec evaluate (expr: expr) : Result<int, string> =
    ...
```

Initially this contains a `// TODO` comment and a `failwith`. This is where you should start implementing the calculator.

As you develop, you should save your results periodically, and go to Inspectify to see any compilation errors, as well as the results of running the calculator. For this ensure that Inspectify is running, as described in [running the code](#running-the-code).

## Evaluation

Every time you push your Git repository, your code is ready to be evaluated automatically by your teachers.

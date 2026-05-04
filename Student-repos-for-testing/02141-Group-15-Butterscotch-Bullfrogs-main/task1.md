# Task 1: A parser for GCL

> **Make sure to [update](./task0.md#update-first) your repository and inspectify** 

> **Deadline: March 12, 23:59**
>
> You must submit your solutions by pushing them to the git repository assigned to your group.
> The last push before the deadline will be considered as your submission. 

The detailed rules of the mandatory assignment are found [here](README.md).

NOTE: ensure that the main branch of your repository is updated with:

- MacOS, Linux: `./update.sh`
- Windows: `powershell.exe -ExecutionPolicy Bypass -File update.ps1`

## Goals

The goal of this task is to build a parser for [GCL](gcl.md) that accepts or rejects programs and builds ASTs for them, thus working like the syntax checker of [formalmethods.dk/fm4fun](http://www.formalmethods.dk/fm4fun/). 

## Detailed Description

> **Relevant files in your group's repository:** 
> 
> `Parser.fs; Lexer.fsl; Grammar.fsy; AST.fs`
> 
>   <details>
>  <summary>Java?</summary>
> 
> `
   Parser_AST_PM.java; Command.java; Expression.java; GuardedCommand.java; Bool.java; GCL.g4; ASTBuilder.java
   `
 </details>
You should implement a parser that takes as input a string, which is intended to describe a [GCL program](gcl.md) but may contain errors, and build an abstract syntax tree (AST) for it. 

In addition, the program must produce compilation results: it should return whether the input is a program accepted by the [GCL grammar specified in this repository](gcl.md).

Furthermore, you must implement a "Pretty Printer", i.e. a code generator, that prints the AST so you can easily check your solution.

Launch inspectify as usual:

```
# On Windows
powershell.exe -ExecutionPolicy Bypass -File inspectify.ps1 --open
# On macOS and Linux
./inspectify.sh --open
```

Once inspectify has opened in your browser, click on `Parse` and start working on your task


## Hints
- Get inspired by the calculator example of the code framework in your group's repository.
- Watch the starter video.
- Start with the [GCL grammar specified in this repository](gcl.md) and adapt it to your parser generator:
    - `AST.fs`: add one type for each non-terminal symbol of the grammar, define the constructors of the type.
    - `Grammar.fsy`: add new non-terminals with their productions, add token declarations, and add the code generation part (based on your new types). You may need to specify precedence and associativity of some operators in the parser generator language, or by applying some of the grammar transformations seen in class. 
    - `Lexer.fsl`: add rules to define the new tokens. 
    - `Parser.fs`: you need to implement function `prettyPrint`, which, given the AST of the parsed GCL program, generates the code as a string.
    <details><summary>
    Java?</summary>
    
    - `Command.java; Expression.java; GuardedCommand.java; Bool.java;`: These are the types for each non-terminal symbol of the grammar, define the constructors of the type You will have to create interfaces for the type(s) you don't have already. - `GCL.g4`: add new non-terminals with their productions, add token declarations and rules to define them, and add the code generation part (based on your new types). You don't need to define tokens for everything but it can help you and gives you a better understanding of lexing. You may need to specify precedence and associativity of some operators in the parser generator language, or by applying some of the grammar transformations seen in class. - `ASTBuilder.java`: Convert the CST you get from ANTLR using your grammar into an AST. 
    - `Parser_AST_PM.java`: you need to implement function `prettyPrint`, which, given the AST of the parsed GCL program, generates the code as a string.
 </details>
 
- Address the task with an incremental approach. Start with a simple version of GCL (e.g. with just `skip`) and get your parser to work. Incrementally add more features (assignments, sequences, if/do, etc.) to the language until you cover the entire language.

## Feedback & Evaluation

You will need the GCL parser developed in this task to complete the follow-up tasks.

We encourage you to proactively ask for feedback from the TAs and the teacher.

Please submit your task solution by simply pushing changes to the main branch of your repository. You can do this as many times as you want. We will have a look at the latest update (before the deadline).

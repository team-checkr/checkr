# Task 0: Building a calculator

## Update first!

> NOTE: ensure that the main branch of your repository is updated with:
> - MacOS, Linux: `./update.sh`
> - Windows: `powershell.exe -ExecutionPolicy Bypass -File update.ps1`
> - Windows: `powershell.exe -ExecutionPolicy Bypass -File update.ps1`

## Goals

The goal of this task is to complete the calculator.

## Detailed Description

> **Relevant files in your group's repository:** 
> 
> `Calculator.fs; Lexer.fsl; Grammar.fsy; AST.fs`
> 
>   <details>
>  <summary>Java?</summary>
> 
> `
   Calculator_AST_PM.java; Expression.java; GCL.g4; ASTBuilder.java
   `
 </details>


Launch inspectify:

```
# On Windows
powershell.exe -ExecutionPolicy Bypass -File inspectify.ps1 --open
powershell.exe -ExecutionPolicy Bypass -File inspectify.ps1 --open
# On macOS and Linux
./inspectify.sh --open
```

Once Inspectify has opened in your browser, click on `Calculator`. Inspectify will complain. Your goal is be able to write arithmetic expressions like the ones of GCL (without variables and arrays).

## Hints
- Open file `Calculator.fs` and complete function `evaluate`.  <details><summary>Java?</summary>
   The file is `Calculator_AST_PM.java`.
 </details>

- Use pattern matching to implement `evaluate` recursively.
- File `AST.fs` contains the definition of type `expr`, which you should follow to identify the cases neded in the pattern maching <details><summary>Java?</summary>
   The file is `Expression.java`. </details>
- File `Grammar.fsy` describes the grammar of arithmetic expression and how expressions are built as values of type `expr`. <details><summary>Java?</summary>
   The file is `GCL.g4`. </details>
- Sketch of a possible solution:

```fsharp
match expr with
    | Num(x) -> Ok x
    | PlusExpr(x,y) -> 
    ...
```

<details>
<summary>Java?</summary>
 
```java
 return switch (expression){
            case Expression.NumExpr numExpr -> {
                yield numExpr.value();
            }
            case Expression.AddExpr addExpr -> {
                yield evaluate(addExpr.left()) + evaluate(addExpr.right());
            }
}
```
 </details>


Done? Move to [task 1](task1.md).

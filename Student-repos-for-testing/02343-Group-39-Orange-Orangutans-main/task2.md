# Task 2: A Compiler for GCL

> **Make sure to [update](./task0.md#update-first) your repository and inspectify** 

> **Deadline: March 19, 23:59**
>
> You must submit your solutions by pushing them to the git repository assigned to your group.
> The last push before the deadline will be considered as your submission. 

The detailed rules of the mandatory assignment are found [here](README.md).

NOTE: ensure that the main branch of your repository is updated with:

- MacOS, Linux: `./update.sh`
- Windows: `powershell.exe -ExecutionPolicy Bypass -File update.ps1`

## Goals

The goal of this task is to implement a compiler that turns GCL programs into Program Graphs (PGs) similar to the results you obtain under 'Program Graph' in [formalmethods.dk/fm4fun](http://www.formalmethods.dk/fm4fun/).

## Detailed Description

> **Relevant files in your group's repository:**
>
> `Compiler.fs`
> 
>   <details>
>  <summary>Java?</summary>
> 
> `
>   Compiler_AST_PM.java
>   `
>  </details>
> 


Your task is to implement the function
```
let analysis (input: Input) : Output =
    failwith "Compiler not yet implemented" // TODO: start here
```

<details>
<summary>Java?</summary>


    public static Io.Compiler.Output analysis(Io.Compiler.Input input) {
        Command ast = Initial_AST_Generate.compiler_generate(input);
        return new Io.Compiler.Output("digraph program_graph {rankdir=LR;qS -> q1[label = \"c := ( - 30 )\"];q1 -> qF[label = \"a := ( ( - 14 ) * 2 )\"];}");    
    }

</details>

which takes a [GCL program](gcl.md) and produces a program graph in the [DOT language](https://graphviz.org/doc/info/lang.html) - a language for visualizing graphs.
That is, the compiler must produce a program graph in the textual graphviz format used by the export feature on [formalmethods.dk/fm4fun](http://www.formalmethods.dk/fm4fun/). The input also specifies whether you have to produce a deterministic or a non-deterministic program graph.

Launch inspectify as usual:

```
# On Windows
powershell.exe -ExecutionPolicy Bypass -File inspectify.ps1 --open
# On macOS and Linux
./inspectify.sh --open
```

Once inspectify has opened in your browser, click on `Compiler` and start working on your task.

## Hints

* **IMPORTANT**: Implement 2 functions: (1) A function `edges` that, given an `Input`, produces an internal representation of the program graph as a set of edges and (2) A function `printDot` that, given a set of edges, generates program graph in the text-based [dot format](https://graphviz.org/doc/info/lang.html).
* You may need the `edges` functions in some subsequent task.
* You have to consider both deterministic and non-deterministic semantics of GCL. You can use a flag to deal with the corresponding type indicated in argument `input : Input`.
* To implement the non-deterministic case of the function `edges` follow [Formal Methods, Chapter 2.2](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936), which explains how to construct a program graph for a GCL program. You can also review these lecture notes about [PGs for GCL](https://gitlab.gbar.dtu.dk/02141/02343/-/blob/main/L02-The_Guarded_Command_Language/L02-Program_Graphs.md?ref_type=heads#from-gcl-to-pgs). We recommend to implement the function `edges` from the book, which takes as input the AST of a GCL program and produces as output a program graph (represented as a set of edges).
* Similarly, for the deterministic case we recommend to follow the approach in [Formal Methods, Chapter 2.4](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936) or the program-to-program transformation approach descibed in these lecture notes about [non-determinism](https://gitlab.gbar.dtu.dk/02141/02343/-/blob/main/L03-PG_and_GLC_round_up/L03-L03-PG_and_GLC_round_up.md?ref_type=heads).
* The function `printDot` prints the graph in the dot format given its representation as a set of edges.
* The function `analysis` will invoke first the `edges` function and then the `printDot`function.

## Feedback & Evaluation

You can evaluate your solution by comparing the result to the ones provided by the [`fm4fun`](http://www.formalmethods.dk/fm4fun/) or `inspectify` tools.

We encourage you to proactively ask for feedback from the TAs and the teacher.

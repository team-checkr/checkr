# Task 3: An Interpreter for GCL

> **Make sure to [update](./task0.md#update-first) your repository and inspectify** 

> **Deadline: March 23, 23:59**
>
> You must submit your solutions by pushing them to the git repository assigned to your group.
> The last push before the deadline will be considered as your submission.

The detailed rules of the mandatory assignment are found [here](README.md).

NOTE: ensure that the master branch of your repository is updated with:

- MacOS, Linux: `./update.sh`
- Windows: `powershell.exe -ExecutionPolicy Bypass -File update.ps1`

## Goals

The goal of this task is to implement an interpreter
that executes a GCL program on a given memory step by step, similarly to the environment `Step-wise Execution` on [formalmethods.dk/fm4fun](http://www.formalmethods.dk/fm4fun/) and `Interpreter` on `Inspectify`.


## Detailed Description

> **Relevant files in your group's repository:**
>
> `Interpreter.fs`
> 
>   <details>
>  <summary>Java?</summary>
> 
> `
>   Interpreter_AST_PM.java
>   `
>  </details>

Your task is to implement the function
```
let analysis (src: string) (input: Input) : Output = // TODO
```

<details>
<summary>Java?</summary>


    public static Io.Interpreter.Output analysis(Io.Interpreter.Input input) {
        Command ast = Initial_AST_Generate.interpreter_generate(input); 
        String dotString = "digraph program_graph {rankdir=LR;qS -> q1[label = \"c := ( - 30 )\"];q1 -> qF[label = \"a := ( ( - 14 ) * 2 )\"];}";
        List<Io.Interpreter.Step> trace =  List.of(new Step("assign", "Node1", new InterpreterMemory(Map.of("x", 10, "y", 20), Map.of("arr", List.of(1, 2, 3, 4)))));
        Io.Interpreter.TerminationState terminationState = TerminationState.Terminated;
        return new Io.Interpreter.Output("qS", "qF", dotString, trace, terminationState);
    }

</details>

The above functon takes a string representation of a [GCL program](gcl.md) and a structure input that determines
- whether we consider a deterministic program graph or not,
- the initial memory, and
- a `trace_length`.
As an output, the function should produce an execution sequence of length `trace_length` starting in an initial configuration with the provided initial memory. If no execution sequence of that length exists, it should produce an execution sequence that is complete or gets stuck.
Moreover, the output should indicate whether the execution sequence is complete, stuck or still running, i.e. it can still be extended.
The types for producing such an output are provided in `Interpreter.fs`.

## Hints

Follow [Formal Methods, Chapter 1.2](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936) and [Formal Methods, Chapter 2.3](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936) to build an interpreter based on the semantics of GCL programs and their program graphs:
* You are writing an interpreter for program graphs [Formal Methods, Chapter 1.2](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936), but of course such graphs are generated from GCL programs and that is why you need [Formal Methods, Chapter 2.3](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936).
* These lectures note recalls the idea of [running PGs](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936) as sequences of execution steps.
* Definition 1.11 of the [book]((https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936) defines the concept of execution step more precisely. You will most likely to implement a function to perform an execution step.
* Definition 1.11 relies on the semantic function `S`, which specifies how the memory is affected by actions (labels in the program graph). You will need to have a look at Definition 2.17, which defines the semantic function for GCL actions. This will in turn require you to implement the functions to evaluate arithmetic and boolean expressions [Formal Methods, Chapter 2.3](https://findit.dtu.dk/verify?locale=en&return=%2Fen%2Fcatalog%2F5d4ab0c9d9001d1f5204d936).
* Definition 1.12 defines the concept of execution sequence. The intrepreter needs produce execution sequences of a maximum length.

## Feedback & Evaluation

We encourage you to proactively ask for feedback from the TAs and the teacher.

The evaluation will use a more powerful version of `inspectify` in addition to manual inspection.

# Task 4: RISC-V Compiler

> **Make sure to [update](./task0.md#update-first) your repository and inspectify** 

> **Deadline: April 9, 23:59**
>
> You must submit your solutions by pushing them to the git repository assigned to your group.
> The last push before the deadline will be considered as your submission.

The detailed rules of the mandatory assignment are found [here](README.md).

NOTE: ensure that the master branch of your repository is updated with:

- MacOS, Linux: `./update.sh`
- Windows: `powershell.exe -ExecutionPolicy Bypass -File update.ps1`

## Goals

The goal of this task is to implement a compiler that turns GCL programs into RISC-V code.

You should see two environments in Inspecfiy:
* `BiGCL`: This is an intermediate step, more precise a GCL to GCL transpiler.
    * Input: A GCL program `P`.
    * Output:  A GCL program `Q` that is equivalent to `P` but does not use more than 2 operands in every expression. Moreover, `Q` does not use any binary boolean operator.
* `RiscV`: This is the final GCL to RISC-V code compiler.
    * Input: A GCL program `P`.
    * Output: An equivalent RISC-V program `Q`.

You will code both the `BiGCL` transpiler and the `RiscV` compiler.

> GOOD NEWS: You can ignore arrays in this task :)

## Recommended schedule

You will have 3 TA-assisted sessions for this task. We reccommend the following schedule:
* Session 1:
    * Learn about RISC-V reading these [lecture notes](https://courses.compute.dtu.dk/02247/f26/risc-v.html). 
    * Play with `RISC-V` to see examples of RISC-V programs based on simple GCL programs. You can also try the examples on the [`RARS`](https://courses.compute.dtu.dk/02247/f26/risc-v.html#sec-rars) emulator.
* Session 2: Work on the `BiGCL` transpiler. 
* Session 3: Work on the `RISC-V` compiler.

## Detailed Description

> **Relevant files in your group's repository:**
>
> `BiGCL.fs`
> `RiscV.fs`

## BiGCL Transpiler

Your task is to implement the function
```
let analysis (src: string) (input: Input) : Output = // TODO
```

The input is a GCL program `P`.

The output is a GCL program `Q` that has the following properties:
* It is equivalent to `P`.
* Arithmetic expressions contain 1 or 2 operands but no more:
    * Expressions that are ok:
        * `1`
        * `x` 
        * `x + y`
    * Expressions like are not ok:
        * `1 + 2 + 3`
        * `x + y + z` 
* Boolean expressions contain 1 or 2 operands and do not contain binary logical operators:
    * Expressions that are ok
        *  `true`
        * `x = y`
        * `x <= 0`
    * Expressions that are not ok:
        * `x <= 0 & true`
        * `true | false` 
        * `x = y + z`

### Why?

We have two main challenges for translating GCL into RISC-V:
1. The languages use different syntax. For example, consider how computing the sum of two numbers is done in each language:
    * GCL:
        ```
        a := b + c
        ```
    * RISC-V:
        ```riscv
        add a, b, c
        ```
2. GCL has less restrictions when it comes to complexity of expressions or branching:
    * Arithmetic Expressions:
        * GCL: arithmetic expression can have any number of operands.
        * RISC-V: arithmetic operations are limited to sum/mult/... of 2 operands. 
    * Boolean expressions:
        * GCL: Boolean expressions can have any number of operands.
        * RISC-V: Boolean expressions are used in branching and are limited to comparing two registers.
    * Branching:
        * GCL: Branching factor is arbritrary, you can have a guarded command with any positive number of branches.
        * Branching is either 1 (continue or unconditional jump) or 2 (conditional jumps).

The transpiler `BiGCL` is an intermediate step towards generating RISC-V code. The focus in on addressing the point (2) above and postponing (1) for a latter step.

The goal is to transform the GCL program`P` into another GCL program that uses a reduced instruction set similar to that of RISC-V.


### How?

One problem you will have to solve is how to transform assignments that use 3 or more operands like in

```c
a := b + c + d
```

into an equivalent program that uses only 2 operands at most. For example:

```c
tmp := b + c ;
a := tmp + d ;
```

Here is a sketch of something you could start with:

```
               bin(x:=n) = x:=n                // if n is a number
               bin(x:=y) = x:=y                // if y is a variable
        bin(x:=a1 op a2) = x:=a1               // if a1 and a2 are numbers/variables
        bin(x:=a1 op a2) = bin(x:=a1) ; bin(x:=x op a2) // owise
```

There is something wrong though... you will have to discover what.

Other challenges:
* How would you tranform a program with 3 branches into an equivalent one that uses 2 branches at most? For example:

    ```
    if 
        x<0 -> y:=-1
    []
        x=0 -> y:=0
    []
        x>0 -> y:=1
    fi
    ```
* How would you deal with Boolean conditions that use complex arithmetics expressions. For example this:

    ```
    if x <= y + z -> x := 1
    []
    x > y + z -> x := -1
    fi
    ```
<details>
<summary>HINT</summary> Try the reference implementation of `BiGCL` to see how each single challenge is dealt with. 
</details>

## RISC-V

Your task is to implement the function
```
let analysis (src: string) (input: Input) : Output = // TODO
```

The input is a GCL program. The output should be an equivalent RISC-V program.

### How?

If you have completed the `BiGCL` enviroment, try this:

1. Take a program that has at least one `if` or `do` with 3 or more branches. For example, this:

    ```
    if 
        x<0 -> y:=-1
    []
        x=0 -> y:=0
    []
        x>0 -> y:=1
    fi
    ```

2. Copy the generated output GCL code. It will look like this:

    ```
    if (x < 0) ->
    y := -1
    [] !(x < 0) ->
    if (x = 0) ->
        y := 0
    [] !(x = 0) ->
        if (x > 0) ->
            y := 1
        [] !(x > 0) ->
            stuck_ := (1 / 0)
        fi
    fi
    fi
    ```

3. Paste it into the `Compiler` environment. It will generate a PG. 

4. Observe the obtained graph.
    * How many outgoing edges are there per node?
        <details>
        <summary>Answer</summary> At most 2!  Just like branching in RISC-V where you can "jump or continue".
        </details>
    * How complex are the edge lables that contain boolean predicates? 
        <details>
        <summary>Answer</summary> They just contain `true`, `false`, (in)equalities or negation .
        </details>
    * Would you be able to translate them into RISC-V code?
        <details>
        <summary>Answer</summary> Hmmm... there seem to be close to RISC-V jump operations `beq`, `bneq` and `blt`...
        </details>
    * How complex are the edge lables that contain assignments?
        <details>
        <summary>Answer</summary> The rhs only contains numbers, variables or the combination of one number/variable and another number/variable in a binary operation.
        </details>
    * Would you be able translate them into RISC-V code?
        <details>
        <summary>Answer</summary> single assignments resemble load/store RISC-V operations `li`, `lw`,...., and assignments with 2 operands resemble RISC-V operations `add`, `sub`,... 
        </details>

> Your generated RISC-V code is allowed to use the operations listed in the [lecture notes](https://courses.compute.dtu.dk/02247/f26/risc-v.html), except for the floating point operations. If you need more instruction... ask the teacher :)

## Requirements
* You are free register and memory as you please
* For each GCL variable `xxx` you must use the RISC-V label `vxxx` (i.e. just put a `v` in front of the GCL variable name).

## Hints

* Use the functions you have implemented for `BiGCL` to generate a simpler GCL program `Q`. 
* Transform `Q` into a PG using your `Compiler`. The PG will be representing the control flow of your program with just conditional jumps from one node (RISC-V PC label) to another node (RISC-V PC label).
* Go through all edges and translate their labels into the corresponding RISC-V code. Soem cases will be trivial, some may require more though.
* Beware of nodes that have exactly two outgoing edges. These cases represent `if-then-else` situations. 

## Feedback & Evaluation

We encourage you to proactively ask for feedback from the TAs and the teacher.

The evaluation will use a more powerful version of `inspectify` in addition to manual inspection.

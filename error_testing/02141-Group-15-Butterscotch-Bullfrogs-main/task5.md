# Task 5: Program Verification

> **Make sure to [update](./task0.md#update-first) your repository and inspectify** 

> **Deadline: April 20, 23:59**
>
> You must submit your solutions by pushing them to the git repository assigned to your group.
> The last push before the deadline will be considered as your submission. 

The detailed rules of the mandatory assignment are found [here](README.md).

NOTE: ensure that the master branch of your repository is updated with:

- MacOS, Linux: `./update.sh`
- Windows: `powershell.exe -ExecutionPolicy Bypass -File update.ps1`

## Goals

The goal of this task is to apply your knowledge about program verification to write formal correctness proofs for a given set of GCL programs.


## Detailed Description

> **Relevant files in your group's repository:**
>
> `task5/part01.gcl`
> `task5/part02.gcl`
> `task5/part03.gcl`
> `task5/part04.gcl`
> `task5/part05.gcl`
> `task5/part06.gcl`
> `task5/part07.gcl`
> `task5/part08.gcl`
> `task5/part09.gcl`
> `task5/part10.gcl`

Each file listed above contains a Floyd-Hoare triple `{ P } C { Q }` consisting of a precondition `P`, a GCL command `C` and a postcondition `Q`.


For each of those Floyd-Hoare triples `{ P } C { Q }`, your task is to show that the triple is valid, i.e., `|= { P } C { Q }`, by extending it to a *fully annotated command*.

You can use our verification playground [CHIP](https://team-checkr.github.io/chip) to develop and check your solutions.
CHIP should yield both "verified" *and* "The program is fully annotated".

Notice that CHIP does not report "verified" for some of the given example programs because you might have to add suitable invariants first.

## Rules

You can add invariants and as many annotations as you want. However, you are *not* allowed to change the given command `C`, 
the precondition `P`, or the postcondition `Q`. This also means that you are *not* allowed to add any annotations above the provided precondition or below the provided postcondition.

As usual, push your solution to your group's git repository before the deadline. Your solution to each part must be provided in the initially provided file in the directory `task5`. For example, the solution to part 1 must be contained in the file `task5/part01.gcl`.

## Example

Imagine that the content of imaginary file part0.gcl as *provided* is

```
{x>=y}
x:=x+1;
y:=y-1
{x>y}
```
Then, you will have to edit the file so it looks like this:

```
{x>=y}
x:=x+1;
{x>y-1} // new annotation added
y:=y-1
{x>y} 
```
As you can see a new annotation was added (as indicated by the comment) and all other lines were untouched. If you just copy-past the entire file content into Chip, it should say ``The program is fully annotated``

*IMPORTANT*: We will take your file and verify it with Chip. We will also check that you obey to the rules described above.

## Hints

* All material needed for this task is covered by the slides on program verification, which are available on DTU learn.
* CHIP is typically powerful enough to verify a (correct) proof outline as long as you provide a precondition, a postcondition, and suitable invariants for all loops. You can thus first play around to find invariants before attempting to write a fully annotated command.
* You might want to verify smaller parts of a given program with CHIP before considering the whole program.
* Finding loop invariants is difficult. It may take you some time to fully comprehend how each program works in detail.
* You can use your interpreter to test the given programs on different inputs in order to get some intuition about potential invariants.
* You can also try to write a proof outline and infer from that how an invariant needs to look like.


## Feedback & Evaluation

Besides using CHIP, we encourage you to proactively ask for feedback from the TAs and the teacher.

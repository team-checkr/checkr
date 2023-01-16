# Group 1
## Step-wise Execution
<details><summary>
<strong>Program 1</strong> – 
Correct
</summary>



```py
d := b ;
c := d ;
b := -1 ;
b := d ;
if (68 <= -2) ->
   c := c
fi ;
do (b = 8) ->
   do (!(-29 <= c) | true) ->
      do !(b <= a) ->
         do ((false || (((-59 >= b) && !!!true) & true)) || false) ->
            c := -92
         od
      od
   od
od ;
d := d ;
if !(false & !(((a > b) || (!false || true)) | true)) ->
   d := c
fi ;
do !((-6 > d) & ((c >= b) && true)) ->
   d := -99
od ;
do !(a <= b) ->
   if (false & ((c <= a) || (d != -58))) ->
      do !(c <= -62) ->
         if !(-66 = a) ->
            do true ->
               c := a
            od
         fi
      od
   fi
od
```


### Input

#### Determinism:

Deterministic

#### Memory:

`[a = 7, b = 4, c = 6, d = 8]`


### Output 

```
StepWiseOutput(
    [
        ProgramTrace {
            state: Running,
            node: "Node 0",
            memory: Memory {
                variables: {
                    a: 7,
                    b: 4,
                    c: 6,
                    d: 8,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 2",
            memory: Memory {
                variables: {
                    a: 7,
                    b: 4,
                    c: 6,
                    d: 4,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 3",
            memory: Memory {
                variables: {
                    a: 7,
                    b: 4,
                    c: 4,
                    d: 4,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 4",
            memory: Memory {
                variables: {
                    a: 7,
                    b: -1,
                    c: 4,
                    d: 4,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Stuck,
            node: "Node 5",
            memory: Memory {
                variables: {
                    a: 7,
                    b: 4,
                    c: 4,
                    d: 4,
                },
                arrays: {},
            },
        },
    ],
)
```


</details>
<details><summary>
<strong>Program 2</strong> – 
Correct
</summary>



```py
do (((b != -98) & (!(d = 45) | !!true)) & false) ->
   b := b
od ;
if (d >= c) ->
   c := c
fi ;
a := d ;
c := d ;
a := b ;
do false ->
   c := -49
od ;
a := c ;
c := 24 ;
a := a ;
a := 95
```


### Input

#### Determinism:

Deterministic

#### Memory:

`[a = 5, b = -4, c = 9, d = 9]`


### Output 

```
StepWiseOutput(
    [
        ProgramTrace {
            state: Running,
            node: "Node 0",
            memory: Memory {
                variables: {
                    a: 5,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 2",
            memory: Memory {
                variables: {
                    a: 5,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 5",
            memory: Memory {
                variables: {
                    a: 5,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 4",
            memory: Memory {
                variables: {
                    a: 5,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 6",
            memory: Memory {
                variables: {
                    a: 9,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 7",
            memory: Memory {
                variables: {
                    a: 9,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 8",
            memory: Memory {
                variables: {
                    a: -4,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 9",
            memory: Memory {
                variables: {
                    a: -4,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 11",
            memory: Memory {
                variables: {
                    a: 9,
                    b: -4,
                    c: 9,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 12",
            memory: Memory {
                variables: {
                    a: 9,
                    b: -4,
                    c: 24,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Running,
            node: "Node 13",
            memory: Memory {
                variables: {
                    a: 9,
                    b: -4,
                    c: 24,
                    d: 9,
                },
                arrays: {},
            },
        },
        ProgramTrace {
            state: Terminated,
            node: "Node 1",
            memory: Memory {
                variables: {
                    a: 95,
                    b: -4,
                    c: 24,
                    d: 9,
                },
                arrays: {},
            },
        },
    ],
)
```


</details>

| Program    | Result              | Time         |
|------------|---------------------|--------------|
| Program 1  | Correct             | 152.220542ms |
| Program 2  | Correct             | 158.307083ms |
| Program 3  | Correct             | 155.47375ms  |
| Program 4  | Correct             | 154.718042ms |
| Program 5  | Correct<sup>*</sup> | 153.6765ms   |
| Program 6  | Correct             | 162.177667ms |
| Program 7  | Correct             | 153.255541ms |
| Program 8  | Correct<sup>*</sup> | 166.271875ms |
| Program 9  | Correct             | 156.283708ms |
| Program 10 | Correct<sup>*</sup> | 156.059834ms |
## Detection of Signs Analysis
<details><summary>
<strong>Program 1</strong> – 
Correct
</summary>



```py
d := b ;
c := d ;
b := -1 ;
b := d ;
if (68 <= -2) ->
   c := c
fi ;
do (b = 8) ->
   do (!(-29 <= c) | true) ->
      do !(b <= a) ->
         do ((false || (((-59 >= b) && !!!true) & true)) || false) ->
            c := -92
         od
      od
   od
od ;
d := d ;
if !(false & !(((a > b) || (!false || true)) | true)) ->
   d := c
fi ;
do !((-6 > d) & ((c >= b) && true)) ->
   d := -99
od ;
do !(a <= b) ->
   if (false & ((c <= a) || (d != -58))) ->
      do !(c <= -62) ->
         if !(-66 = a) ->
            do true ->
               c := a
            od
         fi
      od
   fi
od
```


### Input

Determinism: NonDeterministic

Memory: [a = -, b = -, c = +, d = +]


### Output 

| Node    | a | b | c | d |
|---------|---|---|---|---|
| Node 0  | - | - | + | + |
| Node 1  |   |   |   |   |
| Node 10 |   |   |   |   |
| Node 11 |   |   |   |   |
| Node 12 |   |   |   |   |
| Node 13 |   |   |   |   |
| Node 14 |   |   |   |   |
| Node 15 |   |   |   |   |
| Node 16 |   |   |   |   |
| Node 17 |   |   |   |   |
| Node 18 |   |   |   |   |
| Node 19 |   |   |   |   |
| Node 2  | - | - | + | - |
| Node 20 |   |   |   |   |
| Node 21 |   |   |   |   |
| Node 22 |   |   |   |   |
| Node 3  | - | - | - | - |
| Node 4  | - | - | - | - |
| Node 5  | - | - | - | - |
| Node 6  |   |   |   |   |
| Node 7  |   |   |   |   |
| Node 8  |   |   |   |   |
| Node 9  |   |   |   |   |


</details>
<details><summary>
<strong>Program 2</strong> – 
Mismatch { reason: "Produced world which did not exist in reference" }
</summary>



```py
do (((b != -98) & (!(d = 45) | !!true)) & false) ->
   b := b
od ;
if (d >= c) ->
   c := c
fi ;
a := d ;
c := d ;
a := b ;
do false ->
   c := -49
od ;
a := c ;
c := 24 ;
a := a ;
a := 95
```


### Input

Determinism: NonDeterministic

Memory: [a = -, b = -, c = -, d = 0]


### Output 

| Node    | a | b | c | d |
|---------|---|---|---|---|
| Node 0  | - | - | - | 0 |
| Node 1  | + | - | + | 0 |
| Node 10 |   |   |   |   |
| Node 11 | 0 | - | 0 | 0 |
| Node 12 | 0 | - | + | 0 |
| Node 13 | 0 | - | + | 0 |
| Node 2  | - | - | - | 0 |
| Node 3  |   |   |   |   |
| Node 4  | - | - | - | 0 |
| Node 5  | - | - | - | 0 |
| Node 6  | 0 | - | - | 0 |
| Node 7  | 0 | - | 0 | 0 |
| Node 8  | - | - | 0 | 0 |
| Node 9  | - | - | 0 | 0 |


</details>

| Program    | Result   | Time         |
|------------|----------|--------------|
| Program 1  | Correct  | 157.400334ms |
| Program 2  | Mismatch | 159.229334ms |
| Program 3  | Correct  | 163.778667ms |
| Program 4  | Correct  | 160.552583ms |
| Program 5  | Mismatch | 165.197125ms |
| Program 6  | Mismatch | 171.804417ms |
| Program 7  | Mismatch | 171.459334ms |
| Program 8  | Error    | 201.34275ms  |
| Program 9  | Error    | 210.62025ms  |
| Program 10 | Mismatch | 168.214042ms |
## Security Analysis
<details><summary>
<strong>Program 1</strong> – 
Correct
</summary>



```py
d := b ;
c := d ;
b := -1 ;
b := d ;
if (68 <= -2) ->
   c := c
fi ;
do (b = 8) ->
   do (!(-29 <= c) | true) ->
      do !(b <= a) ->
         do ((false || (((-59 >= b) && !!!true) & true)) || false) ->
            c := -92
         od
      od
   od
od ;
d := d ;
if !(false & !(((a > b) || (!false || true)) | true)) ->
   d := c
fi ;
do !((-6 > d) & ((c >= b) && true)) ->
   d := -99
od ;
do !(a <= b) ->
   if (false & ((c <= a) || (d != -58))) ->
      do !(c <= -62) ->
         if !(-66 = a) ->
            do true ->
               c := a
            od
         fi
      od
   fi
od
```


### Input

Lattice: A < B, C < D

Classification: [c = D, b = D, a = A, d = D]


### Output 

```
SecurityAnalysisResult {
    actual: [
        Flow(a -> c),
        Flow(a -> d),
        Flow(b -> c),
        Flow(b -> d),
        Flow(c -> c),
        Flow(c -> d),
        Flow(d -> b),
        Flow(d -> c),
        Flow(d -> d),
    ],
    allowed: [
        Flow(a -> a),
        Flow(b -> b),
        Flow(b -> c),
        Flow(b -> d),
        Flow(c -> b),
        Flow(c -> c),
        Flow(c -> d),
        Flow(d -> b),
        Flow(d -> c),
        Flow(d -> d),
    ],
    violations: [
        Flow(a -> c),
        Flow(a -> d),
    ],
}
```


</details>
<details><summary>
<strong>Program 2</strong> – 
Correct
</summary>



```py
do (((b != -98) & (!(d = 45) | !!true)) & false) ->
   b := b
od ;
if (d >= c) ->
   c := c
fi ;
a := d ;
c := d ;
a := b ;
do false ->
   c := -49
od ;
a := c ;
c := 24 ;
a := a ;
a := 95
```


### Input

Lattice: A < B, C < D

Classification: [b = B, d = C, c = C, a = B]


### Output 

```
SecurityAnalysisResult {
    actual: [
        Flow(a -> a),
        Flow(b -> a),
        Flow(b -> b),
        Flow(c -> a),
        Flow(c -> c),
        Flow(d -> a),
        Flow(d -> b),
        Flow(d -> c),
    ],
    allowed: [
        Flow(a -> a),
        Flow(a -> b),
        Flow(b -> a),
        Flow(b -> b),
        Flow(c -> c),
        Flow(c -> d),
        Flow(d -> c),
        Flow(d -> d),
    ],
    violations: [
        Flow(c -> a),
        Flow(d -> a),
        Flow(d -> b),
    ],
}
```


</details>

| Program    | Result   | Time         |
|------------|----------|--------------|
| Program 1  | Correct  | 147.740166ms |
| Program 2  | Correct  | 150.244875ms |
| Program 3  | Correct  | 149.137416ms |
| Program 4  | Correct  | 152.814833ms |
| Program 5  | Correct  | 147.417ms    |
| Program 6  | Correct  | 154.80075ms  |
| Program 7  | Correct  | 160.353583ms |
| Program 8  | Mismatch | 169.276167ms |
| Program 9  | Correct  | 153.856709ms |
| Program 10 | Correct  | 147.482541ms |

## Result explanations

| Result              | Explanation                                                |
|---------------------|------------------------------------------------------------|
| Correct             | Nice job! :)                                               |
| Correct<sup>*</sup> | The program ran correctly for the first {iterations} steps |
| Mismatch            | The result did not match the expected output               |
| Error               | Unable to parse the output                                 |

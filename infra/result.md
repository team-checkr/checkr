# Group 2
## Step-wise Execution
<details><summary><strong>Program 1</strong> – Correct</summary>


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
<details><summary><strong>Program 2</strong> – Correct</summary>


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
| Program 1  | Correct             | 1.130401958s |
| Program 2  | Correct             | 155.337417ms |
| Program 3  | Correct             | 158.55375ms  |
| Program 4  | Correct             | 151.935667ms |
| Program 5  | Correct<sup>*</sup> | 151.245834ms |
| Program 6  | Correct             | 153.900167ms |
| Program 7  | Correct             | 156.7365ms   |
| Program 8  | Correct<sup>*</sup> | 155.936875ms |
| Program 9  | Correct             | 154.376791ms |
| Program 10 | Correct<sup>*</sup> | 153.204041ms |
## Detection of Signs Analysis
<details><summary><strong>Program 1</strong> – Correct</summary>


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

Memory: [a = +, b = -, c = -, d = +]



### Output 

| Node    | a | b | c | d |
|---------|---|---|---|---|
| Node 0  | + | - | - | + |
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
| Node 2  | + | - | - | - |
| Node 20 |   |   |   |   |
| Node 21 |   |   |   |   |
| Node 22 |   |   |   |   |
| Node 3  | + | - | - | - |
| Node 4  | + | - | - | - |
| Node 5  | + | - | - | - |
| Node 6  |   |   |   |   |
| Node 7  |   |   |   |   |
| Node 8  |   |   |   |   |
| Node 9  |   |   |   |   |



</details>
<details><summary><strong>Program 2</strong> – Correct</summary>


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

Memory: [a = -, b = 0, c = -, d = -]



### Output 

| Node    | a | b | c | d |
|---------|---|---|---|---|
| Node 0  | - | 0 | - | - |
| Node 1  | + | 0 | + | - |
| Node 10 |   |   |   |   |
| Node 11 | - | 0 | - | - |
| Node 12 | - | 0 | + | - |
| Node 13 | - | 0 | + | - |
| Node 2  | - | 0 | - | - |
| Node 3  |   |   |   |   |
| Node 4  | - | 0 | - | - |
| Node 5  | - | 0 | - | - |
| Node 6  | - | 0 | - | - |
| Node 7  | - | 0 | - | - |
| Node 8  | 0 | 0 | - | - |
| Node 9  | 0 | 0 | - | - |



</details>

| Program    | Result   | Time         |
|------------|----------|--------------|
| Program 1  | Correct  | 156.832083ms |
| Program 2  | Correct  | 158.838709ms |
| Program 3  | Correct  | 163.141584ms |
| Program 4  | Correct  | 157.354ms    |
| Program 5  | Mismatch | 159.313583ms |
| Program 6  | Mismatch | 161.370791ms |
| Program 7  | Mismatch | 161.368875ms |
| Program 8  | Error    | 369.247542ms |
| Program 9  | Error    | 194.616417ms |
| Program 10 | Mismatch | 160.874875ms |
## Security Analysis
<details><summary><strong>Program 1</strong> – Correct</summary>


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

Classification: [a = D, b = A, c = D, d = D]



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
        Flow(a -> c),
        Flow(a -> d),
        Flow(b -> b),
        Flow(c -> a),
        Flow(c -> c),
        Flow(c -> d),
        Flow(d -> a),
        Flow(d -> c),
        Flow(d -> d),
    ],
    violations: [
        Flow(b -> c),
        Flow(b -> d),
        Flow(d -> b),
    ],
}
```



</details>
<details><summary><strong>Program 2</strong> – Correct</summary>


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

Classification: [c = C, b = B, a = B, d = C]



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
| Program 1  | Correct  | 148.234084ms |
| Program 2  | Correct  | 148.252542ms |
| Program 3  | Correct  | 147.568458ms |
| Program 4  | Correct  | 150.286667ms |
| Program 5  | Correct  | 144.45175ms  |
| Program 6  | Correct  | 151.460125ms |
| Program 7  | Correct  | 150.036417ms |
| Program 8  | Mismatch | 153.813209ms |
| Program 9  | Correct  | 148.804375ms |
| Program 10 | Correct  | 145.565ms    |
## Graph (graphviz)
<details><summary><strong>Program 1</strong> – expected value at line 1 column 1</summary>


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

**Determinism:** Deterministic



<details><summary>`stdout`</summary>


```json
digraph G {
0[label="q▷"]
2[label="q2"]
3[label="q3"]
4[label="q4"]
5[label="q5"]
7[label="q7"]
6[label="q6"]
9[label="q9"]
10[label="q10"]
11[label="q11"]
12[label="q12"]
8[label="q8"]
13[label="q13"]
15[label="q15"]
14[label="q14"]
17[label="q17"]
16[label="q16"]
18[label="q18"]
19[label="q19"]
20[label="q20"]
21[label="q21"]
22[label="q22"]
1[label="q◀"]
0 -> 2[label="d := b"]
2 -> 3[label="c := d"]
3 -> 4[label="b := -1"]
4 -> 5[label="b := d"]
5 -> 7[label="(68 <= -2)"]
7 -> 6[label="c := c"]
6 -> 9[label="(b = 8)"]
9 -> 10[label="(!(-29 <= c) | true)"]
10 -> 11[label="!(b <= a)"]
11 -> 12[label="((false || (((-59 >= b) && !!!true) & true)) || false)"]
12 -> 11[label="c := -92"]
11 -> 10[label="!((false || (((-59 >= b) && !!!true) & true)) || false)"]
10 -> 9[label="!!(b <= a)"]
9 -> 6[label="!(!(-29 <= c) | true)"]
6 -> 8[label="!(b = 8)"]
8 -> 13[label="d := d"]
13 -> 15[label="!(false & !(((a > b) || (!false || true)) | true))"]
15 -> 14[label="d := c"]
14 -> 17[label="!((-6 > d) & ((c >= b) && true))"]
17 -> 14[label="d := -99"]
14 -> 16[label="!!((-6 > d) & ((c >= b) && true))"]
16 -> 18[label="!(a <= b)"]
18 -> 19[label="(false & ((c <= a) || (d != -58)))"]
19 -> 20[label="!(c <= -62)"]
20 -> 21[label="!(-66 = a)"]
21 -> 22[label="true"]
22 -> 21[label="c := a"]
21 -> 19[label="!true"]
19 -> 16[label="!!(c <= -62)"]
16 -> 1[label="!!(a <= b)"]
}

```



</details>

</details>
<details><summary><strong>Program 2</strong> – expected value at line 1 column 1</summary>


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

**Determinism:** Deterministic



<details><summary>`stdout`</summary>


```json
digraph G {
0[label="q▷"]
3[label="q3"]
2[label="q2"]
5[label="q5"]
4[label="q4"]
6[label="q6"]
7[label="q7"]
8[label="q8"]
10[label="q10"]
9[label="q9"]
11[label="q11"]
12[label="q12"]
13[label="q13"]
1[label="q◀"]
0 -> 3[label="(((b != -98) & (!(d = 45) | !!true)) & false)"]
3 -> 0[label="b := b"]
0 -> 2[label="!(((b != -98) & (!(d = 45) | !!true)) & false)"]
2 -> 5[label="(d >= c)"]
5 -> 4[label="c := c"]
4 -> 6[label="a := d"]
6 -> 7[label="c := d"]
7 -> 8[label="a := b"]
8 -> 10[label="false"]
10 -> 8[label="c := -49"]
8 -> 9[label="!false"]
9 -> 11[label="a := c"]
11 -> 12[label="c := 24"]
12 -> 13[label="a := a"]
13 -> 1[label="a := 95"]
}

```



</details>

</details>

| Program    | Result | Time        |
|------------|--------|-------------|
| Program 1  | Error  | 64.455916ms |
| Program 2  | Error  | 62.992709ms |
| Program 3  | Error  | 66.261958ms |
| Program 4  | Error  | 63.4485ms   |
| Program 5  | Error  | 64.242791ms |
| Program 6  | Error  | 64.536041ms |
| Program 7  | Error  | 63.487334ms |
| Program 8  | Error  | 64.219167ms |
| Program 9  | Error  | 63.421125ms |
| Program 10 | Error  | 64.155375ms |

## Result explanations

| Result              | Explanation                                                |
|---------------------|------------------------------------------------------------|
| Correct             | Nice job! :)                                               |
| Correct<sup>*</sup> | The program ran correctly for the first {iterations} steps |
| Mismatch            | The result did not match the expected output               |
| Error               | Unable to parse the output                                 |

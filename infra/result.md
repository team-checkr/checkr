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

| Input        |                                    |
|--------------|------------------------------------|
| Determinism: | **✓**                              |
| Memory:      | `a = 7`, `b = 4`, `c = 6`, `d = 8` |



### Output 

| Node      | a | b  | c | d |
|-----------|---|----|---|---|
| Node 0    | 7 | 4  | 6 | 8 |
| Node 2    | 7 | 4  | 6 | 4 |
| Node 3    | 7 | 4  | 4 | 4 |
| Node 4    | 7 | -1 | 4 | 4 |
| **Stuck** |   |    |   |   |



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

| Input        |                                     |
|--------------|-------------------------------------|
| Determinism: | **✓**                               |
| Memory:      | `a = 5`, `b = -4`, `c = 9`, `d = 9` |



### Output 

| Node                        | a  | b  | c  | d |
|-----------------------------|----|----|----|---|
| Node 0                      | 5  | -4 | 9  | 9 |
| Node 2                      | 5  | -4 | 9  | 9 |
| Node 5                      | 5  | -4 | 9  | 9 |
| Node 4                      | 5  | -4 | 9  | 9 |
| Node 6                      | 9  | -4 | 9  | 9 |
| Node 7                      | 9  | -4 | 9  | 9 |
| Node 8                      | -4 | -4 | 9  | 9 |
| Node 9                      | -4 | -4 | 9  | 9 |
| Node 11                     | 9  | -4 | 9  | 9 |
| Node 12                     | 9  | -4 | 24 | 9 |
| Node 13                     | 9  | -4 | 24 | 9 |
| **Terminated successfully** |    |    |    |   |



</details>

| Program    | Result              | Time         |
|------------|---------------------|--------------|
| Program 1  | Correct             | 156.293042ms |
| Program 2  | Correct             | 160.018416ms |
| Program 3  | Correct             | 169.632ms    |
| Program 4  | Correct             | 170.544459ms |
| Program 5  | Correct<sup>*</sup> | 180.671375ms |
| Program 6  | Correct             | 179.69ms     |
| Program 7  | Correct             | 162.366458ms |
| Program 8  | Correct<sup>*</sup> | 165.90925ms  |
| Program 9  | Correct             | 165.683959ms |
| Program 10 | Correct<sup>*</sup> | 160.013625ms |
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

| Input        |                                    |
|--------------|------------------------------------|
| Determinism: | **✕**                              |
| Memory:      | `a = +`, `b = +`, `c = -`, `d = -` |



### Output 

| Node    | a | b | c | d |
|---------|---|---|---|---|
| Node 0  | + | + | - | - |
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
| Node 2  | + | + | - | + |
| Node 20 |   |   |   |   |
| Node 21 |   |   |   |   |
| Node 22 |   |   |   |   |
| Node 3  | + | + | + | + |
| Node 4  | + | - | + | + |
| Node 5  | + | + | + | + |
| Node 6  |   |   |   |   |
| Node 7  |   |   |   |   |
| Node 8  |   |   |   |   |
| Node 9  |   |   |   |   |



</details>
<details><summary><strong>Program 2</strong> – Mismatch { reason: "Produced world which did not exist in reference" }</summary>


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

| Input        |                                    |
|--------------|------------------------------------|
| Determinism: | **✕**                              |
| Memory:      | `a = -`, `b = -`, `c = -`, `d = 0` |



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
| Program 1  | Correct  | 163.2025ms   |
| Program 2  | Mismatch | 179.736125ms |
| Program 3  | Correct  | 167.692333ms |
| Program 4  | Correct  | 165.771417ms |
| Program 5  | Mismatch | 173.754458ms |
| Program 6  | Mismatch | 167.677875ms |
| Program 7  | Mismatch | 165.533292ms |
| Program 8  | Error    | 218.302708ms |
| Program 9  | Error    | 207.238083ms |
| Program 10 | Mismatch | 168.234792ms |
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

| Input           |                                    |
|-----------------|------------------------------------|
| Lattice:        | `A < B`, `C < D`                   |
| Classification: | `b = D`, `c = D`, `d = D`, `a = A` |



### Output 

|            | Flows                                                                                    |
|------------|------------------------------------------------------------------------------------------|
| Actual     | `a → c`, `a → d`, `b → c`, `b → d`, `c → c`, `c → d`, `d → b`, `d → c`, `d → d`          |
| Allowed    | `a → a`, `b → b`, `b → c`, `b → d`, `c → b`, `c → c`, `c → d`, `d → b`, `d → c`, `d → d` |
| Violations | `a → c`, `a → d`                                                                         |
| Result     | **Insecure**                                                                             |



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

| Input           |                                    |
|-----------------|------------------------------------|
| Lattice:        | `A < B`, `C < D`                   |
| Classification: | `d = B`, `a = C`, `c = B`, `b = C` |



### Output 

|            | Flows                                                                  |
|------------|------------------------------------------------------------------------|
| Actual     | `a → a`, `b → a`, `b → b`, `c → a`, `c → c`, `d → a`, `d → b`, `d → c` |
| Allowed    | `a → a`, `a → b`, `b → a`, `b → b`, `c → c`, `c → d`, `d → c`, `d → d` |
| Violations | `c → a`, `d → a`, `d → b`                                              |
| Result     | **Insecure**                                                           |



</details>

| Program    | Result   | Time         |
|------------|----------|--------------|
| Program 1  | Correct  | 151.86725ms  |
| Program 2  | Correct  | 150.568417ms |
| Program 3  | Correct  | 149.345875ms |
| Program 4  | Correct  | 155.629708ms |
| Program 5  | Correct  | 147.513667ms |
| Program 6  | Correct  | 159.042208ms |
| Program 7  | Correct  | 155.294959ms |
| Program 8  | Mismatch | 156.818208ms |
| Program 9  | Correct  | 151.398375ms |
| Program 10 | Correct  | 148.575ms    |

## Result explanations

| Result              | Explanation                                                |
|---------------------|------------------------------------------------------------|
| Correct             | Nice job! :)                                               |
| Correct<sup>*</sup> | The program ran correctly for the first {iterations} steps |
| Mismatch            | The result did not match the expected output               |
| Error               | Unable to parse the output                                 |

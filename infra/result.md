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
| Program 1  | Correct             | 168.668625ms |
| Program 2  | Correct             | 158.750792ms |
| Program 3  | Correct             | 160.722166ms |
| Program 4  | Correct             | 153.455875ms |
| Program 5  | Correct<sup>*</sup> | 155.287959ms |
| Program 6  | Correct             | 159.334125ms |
| Program 7  | Correct             | 162.759916ms |
| Program 8  | Correct<sup>*</sup> | 160.298958ms |
| Program 9  | Correct             | 159.728541ms |
| Program 10 | Correct<sup>*</sup> | 160.706875ms |
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
| Memory:      | `a = +`, `b = -`, `c = -`, `d = +` |



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
| Memory:      | `a = -`, `b = -`, `c = 0`, `d = -` |



### Output 

| Node    | a | b | c | d |
|---------|---|---|---|---|
| Node 0  | - | - | 0 | - |
| Node 1  |   |   |   |   |
| Node 10 |   |   |   |   |
| Node 11 |   |   |   |   |
| Node 12 |   |   |   |   |
| Node 13 |   |   |   |   |
| Node 2  | - | - | 0 | - |
| Node 3  |   |   |   |   |
| Node 4  |   |   |   |   |
| Node 5  |   |   |   |   |
| Node 6  |   |   |   |   |
| Node 7  |   |   |   |   |
| Node 8  |   |   |   |   |
| Node 9  |   |   |   |   |



</details>

| Program    | Result   | Time         |
|------------|----------|--------------|
| Program 1  | Correct  | 161.741375ms |
| Program 2  | Mismatch | 165.2695ms   |
| Program 3  | Correct  | 165.090584ms |
| Program 4  | Correct  | 166.288875ms |
| Program 5  | Mismatch | 166.203416ms |
| Program 6  | Mismatch | 165.7065ms   |
| Program 7  | Mismatch | 167.391792ms |
| Program 8  | Error    | 198.979875ms |
| Program 9  | Error    | 204.383917ms |
| Program 10 | Mismatch | 167.532959ms |
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
| Classification: | `c = A`, `b = D`, `d = D`, `a = D` |



### Output 

|            | Flows                                                                                    |
|------------|------------------------------------------------------------------------------------------|
| Actual     | `a → c`, `a → d`, `b → c`, `b → d`, `c → c`, `c → d`, `d → b`, `d → c`, `d → d`          |
| Allowed    | `a → a`, `a → b`, `a → d`, `b → a`, `b → b`, `b → d`, `c → c`, `d → a`, `d → b`, `d → d` |
| Violations | `a → c`, `b → c`, `c → d`, `d → c`                                                       |
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
| Classification: | `b = C`, `c = C`, `d = B`, `a = B` |



### Output 

|            | Flows                                                                  |
|------------|------------------------------------------------------------------------|
| Actual     | `a → a`, `b → a`, `b → b`, `c → a`, `c → c`, `d → a`, `d → b`, `d → c` |
| Allowed    | `a → a`, `a → d`, `b → b`, `b → c`, `c → b`, `c → c`, `d → a`, `d → d` |
| Violations | `b → a`, `c → a`, `d → b`, `d → c`                                     |
| Result     | **Insecure**                                                           |



</details>

| Program    | Result   | Time         |
|------------|----------|--------------|
| Program 1  | Correct  | 149.324709ms |
| Program 2  | Correct  | 154.745917ms |
| Program 3  | Correct  | 158.2785ms   |
| Program 4  | Correct  | 160.3395ms   |
| Program 5  | Correct  | 150.735625ms |
| Program 6  | Correct  | 166.004ms    |
| Program 7  | Correct  | 159.269875ms |
| Program 8  | Mismatch | 166.0895ms   |
| Program 9  | Correct  | 160.702542ms |
| Program 10 | Correct  | 158.0585ms   |
## Program Verification
<details><summary><strong>Program 1</strong> – command failed</summary>


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

| Input         |            |
|---------------|------------|
| Postcondition | `Q = true` |



<details><summary>`stdout`</summary>


```json
Unknown analysis pv

```



</details>

</details>
<details><summary><strong>Program 2</strong> – command failed</summary>


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

| Input         |                                                   |
|---------------|---------------------------------------------------|
| Postcondition | `Q = ((((a > 0) ∧ (b = 0)) ∧ (c > 0)) ∧ (d < 0))` |



<details><summary>`stdout`</summary>


```json
Unknown analysis pv

```



</details>

</details>

| Program    | Result | Time        |
|------------|--------|-------------|
| Program 1  | Error  | 30.485167ms |
| Program 2  | Error  | 30.137042ms |
| Program 3  | Error  | 29.348917ms |
| Program 4  | Error  | 29.298625ms |
| Program 5  | Error  | 29.309083ms |
| Program 6  | Error  | 28.446542ms |
| Program 7  | Error  | 30.997041ms |
| Program 8  | Error  | 29.289166ms |
| Program 9  | Error  | 29.259667ms |
| Program 10 | Error  | 29.259209ms |

## Result explanations

| Result              | Explanation                                                |
|---------------------|------------------------------------------------------------|
| Correct             | Nice job! :)                                               |
| Correct<sup>*</sup> | The program ran correctly for the first {iterations} steps |
| Mismatch            | The result did not match the expected output               |
| Error               | Unable to parse the output                                 |

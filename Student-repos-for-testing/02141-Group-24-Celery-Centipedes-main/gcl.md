# Yet Another GCL Variant

The variant of the Guarded Command Language (GCL) that you have to consider throughout the mandatory assignment is a subset of the language used by [http://www.formalmethods.dk/fm4fun](http://www.formalmethods.dk/fm4fun).

*Throughout the mandatory assignment, we will refer to the language specified on this page as GCL.*

To be precise, we consider the variant of the Guarded Command Language given by the following BNF grammar:

```
C  ::=  x := a  |  A[a] := a  |  skip  |  C ; C  |  if GC fi  |  do GC od
GC ::=  b -> C  |  GC [] GC
a  ::=  n  |  x  |  A[a]  |  a + a  |  a - a  |  a * a  |  a / a  |  - a  |  a ^ a  |  (a)
b  ::=  true  |  false  |  b & b  |  b | b  |  b && b  |  b || b  |  ! b
     |  a = a  |  a != a  |  a > a  |  a >= a  |  a < a  |  a <= a  |  (b)
```
where `n` is an integer number, `x` is a program variable, and `A` is an array.

The syntax of variables and numbers, and the associativity and precedence of operators must be the same as in [http://www.formalmethods.dk/fm4fun](http://www.formalmethods.dk/fm4fun); you can find more details on FM4FUN by clicking on the question mark of besides "Examples".
We reproduce parts of the rules here for your convenience:

- Variables `x` and arrays `A` are strings matching the regular expression `[a-zA-Z][a-zA-Z\d_]*` and cannot be any of the language's keywords (e.g. no variable may be named `if` or `od`).
- Numbers `n` match the regular expression `\d+`. 
- We consider numbers as mathematical, i.e. unbounded, integers. In F#, this means that numbers should have the type `bigint` -  an abbreviation for [BigInteger in C#](https://learn.microsoft.com/en-us/dotnet/api/system.numerics.biginteger?view=net-7.0). 
- A whitespace matches the regular expression `[\u00A0 \n \r \t]`, with a mandatory whitespace after if, do, and before fi, od. Whitespaces are ignored anywhere else.
- Precedence and associativity rules:
    * In arithmetic expressions, precedence is highest for `-` (unary minus), then `^`, then `*` and `/`, and lowest for `+` and `-` (binary minus).
    * In boolean expressions, precedence is highest for `!`, then `&` and `&&`, and lowest for `|` and `||`.
    * Operators `*`, `/`, `+`, `-`, `&`, `|`, `&&`, and `||` are left-associative.
    * Operators `^`, `[]`, and `;` are right associative
- && and || do short-circut evaluation (i.e. a minimal number of operands are evaluated left to right), the rest of the operators do eager evaluation (i.e. all operands are evaluted).
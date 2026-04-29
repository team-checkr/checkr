// This file implements a module where we define a data type "expr"
// to store represent arithmetic expressions
module AST

type start_expression =
    |Start of expr

//Calculator expressions
and expr =
    | Num of bigint
    | Var of string
    | Array of string * expr 
    | TimesExpr of (expr * expr)
    | DivExpr of (expr * expr)
    | PlusExpr of (expr * expr)
    | MinusExpr of (expr * expr)
    | PowExpr of (expr * expr)
    | UMinusExpr of (expr)

// Boolean expressions
and bexpr =
    | True
    | False
    | AndExpr of (bexpr * bexpr)        // &  (eager)
    | OrExpr of (bexpr * bexpr)        // |  (eager)
    | AndAndExpr of (bexpr * bexpr)        // && (short-circuit)
    | OrOrExpr of (bexpr * bexpr)        // || (short-circuit)
    | NotExpr of bexpr
    | EqExpr of (expr * expr)          // a = a
    | NeqExpr of (expr * expr)          // a != a
    | GtExpr of (expr * expr)          // a > a
    | GteExpr of (expr * expr)          // a >= a
    | LtExpr of (expr * expr)          // a < a
    | LteExpr of (expr * expr)          // a <= a

//Guarded commands
and gc = 
    | Guard of (bexpr * command)            //b -> C
    | GCChoice of (gc * gc)                 // GC [] GC

// Commands
and command =
    | Assign of string * expr          // x := a
    | ArrAssign of string * expr * expr   // A[a] := a
    | Skip
    | Seq of (command * command)    // C ; C
    | If of gc                     // if GC fi
    | Do of gc                     // do GC od
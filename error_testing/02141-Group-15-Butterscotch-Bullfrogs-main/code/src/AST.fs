// This file implements a module where we define a data type "expr"
// to store represent arithmetic expressions
module AST

type expr =
    | Num of int
    | Var of string
    | Array of (string * expr)
    | TimesExpr of (expr * expr)
    | DivExpr of (expr * expr)
    | PlusExpr of (expr * expr)
    | MinusExpr of (expr * expr)
    | PowExpr of (expr * expr)
    | UMinusExpr of (expr)


type gCommand =
    | GCommand of (bExpr * commands)

and commands =
    | Skip
    | Sequence of (commands * commands)
    | Assign of (string * expr)
    | ArrayAssign of (string * expr * expr)
    | If of gCommand list
    | Do of gCommand list

and bExpr =
    | True
    | False
    | AndExpr    of (bExpr * bExpr) 
    | OrExpr     of (bExpr * bExpr)
    | AndAndExpr of (bExpr * bExpr)
    | OrOrExpr   of (bExpr * bExpr)
    | NotExpr    of bExpr
    | EqExpr     of (expr * expr)
    | NeqExpr    of (expr * expr)
    | GtExpr     of (expr * expr)
    | GteExpr    of (expr * expr)
    | LtExpr     of (expr * expr)
    | LteExpr    of (expr * expr)
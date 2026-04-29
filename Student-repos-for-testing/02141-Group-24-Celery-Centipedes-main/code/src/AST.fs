// This file implements a module where we define a data type "expr"
// to store represent arithmetic expressions
module AST

type expr =
    | Num of bigint
    | Var of string
    | ArrayAccess of (string * expr)
    | TimesExpr of (expr * expr)
    | DivExpr of (expr * expr)
    | PlusExpr of (expr * expr)
    | MinusExpr of (expr * expr)
    | PowExpr of (expr * expr)
    | UMinusExpr of (expr)

// b  ::=  true  |  false  |  b & b  |  b | b  |  b && b  |  b || b  |  ! b
//      |  a = a  |  a != a  |  a > a  |  a >= a  |  a < a  |  a <= a  |  (b)
type boolExpr =
    | True
    | False
    | AndExpr of (boolExpr * boolExpr)
    | OrExpr of (boolExpr * boolExpr)
    | ShortAndExpr of (boolExpr * boolExpr)
    | ShortOrExpr of (boolExpr * boolExpr)
    | NotExpr of boolExpr
    | EqExpr of (expr * expr)
    | NeqExpr of (expr * expr)
    | GtExpr of (expr * expr)
    | GteExpr of (expr * expr)
    | LtExpr of (expr * expr)
    | LteExpr of (expr * expr)

type command =
    | Skip
    | Sequence of (command * command)
    | Assign of (expr * expr)
    | If of guardedCommand
    | Do of guardedCommand

// GC ::=  b -> C  |  GC [] GC
and guardedCommand =
    | Arrow of (boolExpr * command)
    | Choice of (guardedCommand * guardedCommand)

type ASTNode =
    | E of expr
    | B of boolExpr
    | C of command
    | GC of guardedCommand

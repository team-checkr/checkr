// This file implements a module where we define a data type "expr"
// to store represent arithmetic expressions
module AST

type expr =
    | Num of float
    | TimesExpr of (expr * expr)
    | DivExpr of (expr * expr)
    | PlusExpr of (expr * expr)
    | MinusExpr of (expr * expr)
    | PowExpr of (expr * expr)
    | UPlusExpr of (expr)
    | UMinusExpr of (expr)

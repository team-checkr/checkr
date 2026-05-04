// This file implements a module where we define a data type "expr"
// to store represent arithmetic expressions
module AST


type expr =
    | Num of int
    | VarExpr of variable
    | TimesExpr of (expr * expr)
    | DivExpr of (expr * expr)
    | PlusExpr of (expr * expr)
    | MinusExpr of (expr * expr)
    | PowExpr of (expr * expr)
    | UMinusExpr of (expr)

and variable =
    | Var of string
    | List of string * expr

and boolexpr =
    | Bool of bool
    | And of (boolexpr * boolexpr)
    | Or of (boolexpr * boolexpr)
    | BitAnd of (boolexpr * boolexpr)
    | BitOr of (boolexpr * boolexpr)
    | Equal of (expr * expr)
    | SmallerThan of (expr * expr)
    | GreaterThan of (expr * expr)
    | SmallerEq of (expr * expr)
    | GreaterEq of (expr * expr)
    | Not of boolexpr
    | NotEq of (expr * expr)

type command =  
    | Skip
    | Semi of (command * command)
    | Assign of (variable * expr)
    | If of (boolexpr * command) list
    | Loop of (boolexpr * command) list

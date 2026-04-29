// This file implements a module where we define a data type "expr"
// to store represent arithmetic expressions
module AST

type Ident = string

type expr =
    | Num of int
    | Variable of Ident
    | UMinusExpr of (expr)
    | Array of (Ident * expr)
    | TimesExpr of (expr * expr)
    | DivExpr of (expr * expr)
    | PlusExpr of (expr * expr)
    | MinusExpr of (expr * expr)
    | PowExpr of (expr * expr)
    | ParenExpr of expr

type booleanExpr =
    | True
    | False
    | Neg of (booleanExpr)
    | And of (booleanExpr * booleanExpr)
    | Or of (booleanExpr * booleanExpr)
    | Scand of (booleanExpr * booleanExpr)
    | Scor of (booleanExpr * booleanExpr)
    | Eq of (expr * expr)
    | Neq of (expr * expr)
    | Lt of (expr * expr)
    | Gt of (expr * expr)
    | Leq of (expr * expr)
    | Geq of (expr * expr)
    | ParenBool of (booleanExpr)

type command =
    | SequenceCommand of (command * command)
    | ArrayAssignment of (Ident * expr * expr)
    | VariableAssignment of (Ident * expr)
    | Skip
    | IfCommand of guard
    | DoCommand of guard

and guard =
    | SequenceGuard of (guard * guard)
    | Conditional of (booleanExpr * command)

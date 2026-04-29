module BiGCL

open Io.BiGCL
open AST

let binGC gc n = failwith "binGC not yet implemented"

let rec simplifyCompSide (e: expr) (n: int) : command option * expr * int =
    match e with
    | Num _
    | Var _ -> None, e, n // already an atom, nothing to do
    | _ ->
        let pre, e', n = binExpr e n
        let tmp = Var("t" + string n)
        // assign the simplified expr to a fresh temp, then use the temp
        match pre with
        | None -> Some(Assign(tmp, e')), tmp, n + 1
        | Some p -> Some(Sequence(p, Assign(tmp, e'))), tmp, n + 1
and binExpr expr n = 
    match expr with
    | Num _ -> simplifyCompSide expr n
    | ArrayAccess (name, idx) -> 
            let pre, idx', n' = binExpr idx n
            pre, ArrayAccess (name, idx'), n'    
    // | TimesExpr
    // | DivExpr
    // | PlusExpr
    // | MinusExpr
    // | PowExpr
    // | UMinusExpr

let simplifyCmp wrap a b n =
    let leftCmd, leftAtom, n = simplifyCompSide a n
    let rightCmd, rightAtom, n = simplifyCompSide b n

    let prefixCmd =
        match leftCmd, rightCmd with
        | None, x
        | x, None -> x
        | Some p1, Some p2 -> Some(Sequence(p1, p2))

    prefixCmd, wrap (leftAtom, rightAtom), n

let rec binBool (b: boolExpr) (n: int) : command option * boolExpr * int =
    match b with
    | True -> None, True, n
    | False -> None, False, n

    | NotExpr inner ->
        let pre, inner', n = binBool inner n
        pre, NotExpr inner', n

    | LtExpr (a, b) -> simplifyCmp LtExpr a b n
    | LteExpr (a, b) -> simplifyCmp LteExpr a b n
    | EqExpr (a, b) -> simplifyCmp EqExpr a b n
    | GtExpr (a, b) -> simplifyCmp GtExpr a b n
    | GteExpr (a, b) -> simplifyCmp GteExpr a b n
    | NeqExpr (a, b) -> simplifyCmp NeqExpr a b n
    // Binary boolean operations are handled during guardedCommand
    | AndExpr _
    | OrExpr _
    | ShortAndExpr _
    | ShortOrExpr _ -> None, b, n

let rec transpile (c: command) (n: int) : command * int =
    match c with
    | Skip -> Skip, n
    | Sequence (c1, c2) ->
        let c1', n = transpile c1 n
        let c2', n = transpile c2 n
        Sequence(c1', c2'), n
    | Assign (lhs, rhs) ->
        // TODO: binarise rhs
        Assign(lhs, rhs), n
    | If gc ->
        // TODO: binarise guarded command
        If gc, n
    | Do gc ->
        // TODO: binarize guarded command
        Do gc, n

// Simplifies one side of a comparison to an atom using binExpr,
// returning any prefix assignments needed and the simplified expr.

let analysis (input: Input) : Output =
    match Compiler.parse Grammar.start_command input.commands with
    | Error _ -> { binary = "parse error" }
    | Ok ast ->
        let transformed, _ = transpile ast 0
        { binary = Parser.prettyPrint (C transformed) }

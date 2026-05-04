module BiGCL
open Io.BiGCL
open AST
open Parser

// ── Fresh tmp variable generator ─────────────────────────────────────────────
let mutable tmpCount = 0
let freshTmp () =
    tmpCount <- tmpCount + 1
    sprintf "tmp%d_" tmpCount

// ── Helpers ───────────────────────────────────────────────────────────────────
let isAtom = function Num _ | Var _ -> true | _ -> false

let seqOf (cmds: command list) (final: command) : command =
    List.foldBack (fun c acc -> Seq(c, acc)) cmds final

// ── Arithmetic flattening ─────────────────────────────────────────────────────
let rec flattenExpr (e: expr) : command list * expr =
    match e with
    | Num _ | Var _ -> ([], e)
    | UMinusExpr inner ->
        let (cmds, s) = flattenExpr inner
        let t = freshTmp ()
        (cmds @ [Assign(t, UMinusExpr s)], Var t)
    | PlusExpr(l, r)  -> flattenBinOp l r PlusExpr
    | MinusExpr(l, r) -> flattenBinOp l r MinusExpr
    | TimesExpr(l, r) -> flattenBinOp l r TimesExpr
    | DivExpr(l, r)   -> flattenBinOp l r DivExpr
    | PowExpr(l, r)   -> flattenBinOp l r PowExpr
    | Array(a, i) ->
        let (cmds, s) = flattenExpr i
        (cmds, Array(a, s))

and flattenBinOp (l: expr) (r: expr) (mk: expr * expr -> expr) : command list * expr =
    let (lC, lS) = flattenExpr l
    let (rC, rS) = flattenExpr r
    let t = freshTmp ()
    (lC @ rC @ [Assign(t, mk(lS, rS))], Var t)

// Flatten arithmetic inside comparisons only, leave boolean structure intact
let flattenCmp (l: expr) (r: expr) (mk: expr * expr -> bexpr) : command list * bexpr =
    let (lC, lS) = flattenExpr l
    let (rC, rS) = flattenExpr r
    (lC @ rC, mk(lS, rS))

let rec flattenArithInBexpr (b: bexpr) : command list * bexpr =
    match b with
    | True | False -> ([], b)
    | NotExpr inner ->
        let (c, b') = flattenArithInBexpr inner
        (c, NotExpr b')
    | AndExpr(l, r) | AndAndExpr(l, r) ->
        let (lC, lB) = flattenArithInBexpr l
        let (rC, rB) = flattenArithInBexpr r
        (lC @ rC, AndExpr(lB, rB))
    | OrExpr(l, r) | OrOrExpr(l, r) ->
        let (lC, lB) = flattenArithInBexpr l
        let (rC, rB) = flattenArithInBexpr r
        (lC @ rC, OrExpr(lB, rB))
    | EqExpr(l, r)  -> flattenCmp l r EqExpr
    | NeqExpr(l, r) -> flattenCmp l r NeqExpr
    | GtExpr(l, r)  -> flattenCmp l r GtExpr
    | GteExpr(l, r) -> flattenCmp l r GteExpr
    | LtExpr(l, r)  -> flattenCmp l r LtExpr
    | LteExpr(l, r) -> flattenCmp l r LteExpr

// ── Assignment flattening ─────────────────────────────────────────────────────
let flattenAssign (x: string) (e: expr) : command =
    match e with
    | Num _ | Var _ -> Assign(x, e)
    | UMinusExpr inner when isAtom inner -> Assign(x, e)
    | _ ->
        let (cmds, s) = flattenExpr e
        seqOf cmds (Assign(x, s))

// ── Command flattening ────────────────────────────────────────────────────────
let rec flattenCmd (cmd: command) : command =
    match cmd with
    | Assign(x, e)       -> flattenAssign x e
    | ArrAssign(a, i, e) ->
        let (iC, iS) = flattenExpr i
        let (eC, eS) = flattenExpr e
        seqOf (iC @ eC) (ArrAssign(a, iS, eS))
    | Skip               -> Skip
    | Seq(c1, c2)        -> Seq(flattenCmd c1, flattenCmd c2)
    | If(gc) ->
        let (prefixCmds, gc') = processGC gc
        seqOf prefixCmds (If gc')
    | Do(gc) ->
        Do(processGCDo gc)

// ── GC processing ─────────────────────────────────────────────────────────────
//
// Key insight from reference:
// - For a single Guard(b, c): split b into nested ifs, add stuck else branch
// - For GCChoice(gc1, gc2): process each independently, 
//   gc2 gets !(gc1's original top guard) prepended — just ONE negation level,
//   NOT accumulated negations
// - No De Morgan, no normalization — keep ! as-is

// Process a GC for use in If (arithmetic prefix can be lifted out)
and processGC (gc: gc) : command list * gc =
    match gc with
    | Guard(b, c) ->
        let (arithCmds, b2) = flattenArithInBexpr b
        let c' = flattenCmd c
        let split = splitBool b2 c'
        let withStuck = addStuck split
        (arithCmds, withStuck)
    | GCChoice(gc1, gc2) ->
        // Process gc1
        let (cmds1, gc1') = processGC gc1
        // Get the ORIGINAL top guard of gc1 (before processing) for negation
        let neg1 = NotExpr (originalTopGuard gc1)
        // Process gc2 with neg1 prepended
        let (cmds2, gc2') = processGCWithNeg neg1 gc2
        // Can't lift different prefix cmds from branches, fold back in
        let gc1Final = match cmds1 with
                       | [] -> gc1'
                       | _  -> Guard(True, seqOf cmds1 (If gc1'))
        let gc2Final = match cmds2 with
                       | [] -> gc2'
                       | _  -> Guard(True, seqOf cmds2 (If gc2'))
        ([], GCChoice(gc1Final, gc2Final))

// Process a GC for use inside Do (arithmetic stays inside)
and processGCDo (gc: gc) : gc =
    match gc with
    | Guard(b, c) ->
        let (arithCmds, b2) = flattenArithInBexpr b
        let c' = flattenCmd c
        let split = splitBool b2 c'
        let withStuck = addStuck split
        match arithCmds with
        | [] -> withStuck
        | _  -> Guard(True, seqOf arithCmds (If withStuck))
    | GCChoice(gc1, gc2) ->
        let gc1' = processGCDo gc1
        let neg1 = NotExpr (originalTopGuard gc1)
        let gc2' = processGCDoWithNeg neg1 gc2
        GCChoice(gc1', gc2')

// Process gc2 with a negation prepended (for GCChoice else branch)
and processGCWithNeg (neg: bexpr) (gc: gc) : command list * gc =
    match gc with
    | Guard(b, c) ->
        let combined = AndExpr(neg, b)
        let (arithCmds, b2) = flattenArithInBexpr combined
        let c' = flattenCmd c
        let split = splitBool b2 c'
        let withStuck = addStuck split
        (arithCmds, withStuck)
    | GCChoice(gc1, gc2) ->
        // For nested GCChoice in else position:
        // gc1 gets neg prepended
        // gc2 gets neg AND !(gc1's original guard) prepended
        let (cmds1, gc1') = processGCWithNeg neg gc1
        let neg2 = AndExpr(neg, NotExpr(originalTopGuard gc1))
        let (cmds2, gc2') = processGCWithNeg neg2 gc2
        let gc1Final = match cmds1 with
                       | [] -> gc1'
                       | _  -> Guard(True, seqOf cmds1 (If gc1'))
        let gc2Final = match cmds2 with
                       | [] -> gc2'
                       | _  -> Guard(True, seqOf cmds2 (If gc2'))
        ([], GCChoice(gc1Final, gc2Final))

and processGCDoWithNeg (neg: bexpr) (gc: gc) : gc =
    match gc with
    | Guard(b, c) ->
        let combined = AndExpr(neg, b)
        let (arithCmds, b2) = flattenArithInBexpr combined
        let c' = flattenCmd c
        let split = splitBool b2 c'
        let withStuck = addStuck split
        match arithCmds with
        | [] -> withStuck
        | _  -> Guard(True, seqOf arithCmds (If withStuck))
    | GCChoice(gc1, gc2) ->
        let gc1' = processGCDoWithNeg neg gc1
        let neg2 = AndExpr(neg, NotExpr(originalTopGuard gc1))
        let gc2' = processGCDoWithNeg neg2 gc2
        GCChoice(gc1', gc2')

// Split a bexpr into nested gc using only simple guards
// And -> nested if, Or -> GCChoice with !(b1) else
// Does NOT add stuck here — that's done by the caller
and splitBool (b: bexpr) (c: command) : gc =
    match b with
    | AndExpr(l, r) ->
        // b1 & b2 -> C  becomes  b1 -> if b2 -> C fi
        let inner = splitBool r c
        splitBool l (If inner)
    | OrExpr(l, r) ->
        // b1 | b2 -> C  becomes  b1 -> C  []  !(b1) -> b2 -> C
        let branch1 = splitBool l c
        let negL = NotExpr l
        let branch2 = splitBool (AndExpr(negL, r)) c
        GCChoice(branch1, branch2)
    | _ ->
        Guard(b, c)

// Add stuck else branch: GCChoice(gc, !(topGuard(gc)) -> stuck_)
and addStuck (gc: gc) : gc =
    let neg = NotExpr(splitTopGuard gc)
    GCChoice(gc, Guard(neg, Assign("stuck_", DivExpr(Num 1I, Num 0I))))

// Get the top-level guard of a (already split) gc for addStuck negation
and splitTopGuard (gc: gc) : bexpr =
    match gc with
    | Guard(b, _)     -> b
    | GCChoice(g1, _) -> splitTopGuard g1

// Get the ORIGINAL top guard before any processing (for GCChoice negation)
and originalTopGuard (gc: gc) : bexpr =
    match gc with
    | Guard(b, _)     -> b
    | GCChoice(g1, _) -> originalTopGuard g1

// ── Entry point ───────────────────────────────────────────────────────────────
let analysis (input: Input) : Output =
    tmpCount <- 0
    match Parser.parse Grammar.start_commands input.commands with
    | Ok ast ->
        let result = flattenCmd ast
        { binary = Parser.prettyPrint result }
    | Error e ->
        { binary = sprintf "// Parse error: %A" e }
module RiscV
open Io.RiscV
open AST
open Parser

// ── BiGCL flattening (self-contained copy) ────────────────────────────────────

let mutable tmpCount = 0
let freshTmp () =
    tmpCount <- tmpCount + 1
    sprintf "tmp%d_" tmpCount

let isAtom = function Num _ | Var _ -> true | _ -> false

let seqOf (cmds: command list) (final: command) : command =
    List.foldBack (fun c acc -> Seq(c, acc)) cmds final

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

let flattenAssign (x: string) (e: expr) : command =
    match e with
    | Num _ | Var _ -> Assign(x, e)
    | UMinusExpr inner when isAtom inner -> Assign(x, e)
    | _ ->
        let (cmds, s) = flattenExpr e
        seqOf cmds (Assign(x, s))

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

and processGC (gc: gc) : command list * gc =
    match gc with
    | Guard(b, c) ->
        let (arithCmds, b2) = flattenArithInBexpr b
        let c' = flattenCmd c
        let split = splitBool b2 c'
        let withStuck = addStuck split
        (arithCmds, withStuck)
    | GCChoice(gc1, gc2) ->
        let (cmds1, gc1') = processGC gc1
        let neg1 = NotExpr (originalTopGuard gc1)
        let (cmds2, gc2') = processGCWithNeg neg1 gc2
        let gc1Final = match cmds1 with
                       | [] -> gc1'
                       | _  -> Guard(True, seqOf cmds1 (If gc1'))
        let gc2Final = match cmds2 with
                       | [] -> gc2'
                       | _  -> Guard(True, seqOf cmds2 (If gc2'))
        ([], GCChoice(gc1Final, gc2Final))

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

and splitBool (b: bexpr) (c: command) : gc =
    match b with
    | AndExpr(l, r) ->
        let inner = splitBool r c
        splitBool l (If inner)
    | OrExpr(l, r) ->
        let branch1 = splitBool l c
        let negL = NotExpr l
        let branch2 = splitBool (AndExpr(negL, r)) c
        GCChoice(branch1, branch2)
    | _ -> Guard(b, c)

and addStuck (gc: gc) : gc =
    let neg = NotExpr(splitTopGuard gc)
    GCChoice(gc, Guard(neg, Assign("stuck_", DivExpr(Num 1I, Num 0I))))

and splitTopGuard (gc: gc) : bexpr =
    match gc with
    | Guard(b, _)     -> b
    | GCChoice(g1, _) -> splitTopGuard g1

and originalTopGuard (gc: gc) : bexpr =
    match gc with
    | Guard(b, _)     -> b
    | GCChoice(g1, _) -> originalTopGuard g1

// ── Variable collection ───────────────────────────────────────────────────────

let rec collectVarsExpr = function
    | Var x -> Set.singleton x
    | Num _ -> Set.empty
    | Array(a, i) -> Set.add a (collectVarsExpr i)
    | UMinusExpr e -> collectVarsExpr e
    | PlusExpr(l,r)|MinusExpr(l,r)|TimesExpr(l,r)|DivExpr(l,r)|PowExpr(l,r) ->
        Set.union (collectVarsExpr l) (collectVarsExpr r)

let rec collectVarsBexpr = function
    | True | False -> Set.empty
    | NotExpr b -> collectVarsBexpr b
    | AndExpr(l,r)|OrExpr(l,r)|AndAndExpr(l,r)|OrOrExpr(l,r) ->
        Set.union (collectVarsBexpr l) (collectVarsBexpr r)
    | EqExpr(l,r)|NeqExpr(l,r)|GtExpr(l,r)|GteExpr(l,r)|LtExpr(l,r)|LteExpr(l,r) ->
        Set.union (collectVarsExpr l) (collectVarsExpr r)

let rec collectVarsCmd = function
    | Assign(x, e)       -> Set.add x (collectVarsExpr e)
    | ArrAssign(a, i, e) -> Set.add a (Set.union (collectVarsExpr i) (collectVarsExpr e))
    | Skip               -> Set.empty
    | Seq(c1, c2)        -> Set.union (collectVarsCmd c1) (collectVarsCmd c2)
    | If(gc) | Do(gc)    -> collectVarsGC gc

and collectVarsGC = function
    | Guard(b, c)      -> Set.union (collectVarsBexpr b) (collectVarsCmd c)
    | GCChoice(g1, g2) -> Set.union (collectVarsGC g1) (collectVarsGC g2)

// ── Program Graph ─────────────────────────────────────────────────────────────

type Node = string

type Label =
    | LAssign    of string * expr
    | LArrAssign of string * expr * expr
    | LSkip
    | LBool      of bexpr

type Edge = Node * Label * Node

let mutable nodeCount = 0
let freshNode () =
    nodeCount <- nodeCount + 1
    sprintf "q%d" nodeCount

let rec buildEdges (cmd: command) (qs: Node) (qe: Node) : Edge list =
    match cmd with
    | Assign(x, e)       -> [(qs, LAssign(x, e), qe)]
    | ArrAssign(a, i, e) -> [(qs, LArrAssign(a, i, e), qe)]
    | Skip               -> [(qs, LSkip, qe)]
    | Seq(c1, c2) ->
        let qm = freshNode ()
        buildEdges c1 qs qm @ buildEdges c2 qm qe
    | If(gc)  -> buildEdgesGC gc qs qe
    | Do(gc)  -> buildEdgesGC gc qs qs @ doneEdges gc qs qe

and buildEdgesGC (gc: gc) (qs: Node) (qe: Node) : Edge list =
    match gc with
    | Guard(b, c) ->
        let qm = freshNode ()
        (qs, LBool b, qm) :: buildEdges c qm qe
    | GCChoice(gc1, gc2) ->
        buildEdgesGC gc1 qs qe @ buildEdgesGC gc2 qs qe

and doneEdges (gc: gc) (qs: Node) (qe: Node) : Edge list =
    [(qs, LBool(doneGuard gc), qe)]

and doneGuard = function
    | Guard(b, _)      -> NotExpr b
    | GCChoice(g1, g2) -> AndExpr(doneGuard g1, doneGuard g2)

// ── RISC-V emission ───────────────────────────────────────────────────────────

let vname (x: string) = "v" + x

let mutable inlineLabelCount = 0
let freshLabel (prefix: string) =
    inlineLabelCount <- inlineLabelCount + 1
    sprintf "%s_%d" prefix inlineLabelCount

let loadAtom (reg: string) (e: expr) : string list =
    match e with
    | Num n -> [sprintf "    li %s, %s" reg (string n)]
    | Var x ->
        [ sprintf "    la t2, %s" (vname x)
          sprintf "    lw %s, 0(t2)" reg ]
    | _ -> ["    # BUG: expected atom"]

let storeVar (reg: string) (x: string) : string list =
    [ sprintf "    la t2, %s" (vname x)
      sprintf "    sw %s, 0(t2)" reg ]

let emitAssign (x: string) (e: expr) : string list =
    match e with
    | Num _ | Var _ ->
        loadAtom "t0" e @ storeVar "t0" x
    | UMinusExpr inner ->
        loadAtom "t0" inner @ ["    neg t0, t0"] @ storeVar "t0" x
    | PlusExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        ["    add t0, t0, t1"] @ storeVar "t0" x
    | MinusExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        ["    sub t0, t0, t1"] @ storeVar "t0" x
    | TimesExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        ["    mul t0, t0, t1"] @ storeVar "t0" x
    | DivExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        ["    div t0, t0, t1"] @ storeVar "t0" x
    | PowExpr(l, r) ->
        let loopLabel = freshLabel "pow_loop"
        let endLabel  = freshLabel "pow_end"
        loadAtom "t0" l @ loadAtom "t1" r @
        [ "    li t3, 1"
          "    li t4, 0"
          sprintf "%s:" loopLabel
          sprintf "    bge t4, t1, %s" endLabel
          "    mul t3, t3, t0"
          "    addi t4, t4, 1"
          sprintf "    j %s" loopLabel
          sprintf "%s:" endLabel ] @
        storeVar "t3" x
    | _ -> [sprintf "    # Unhandled assign for %s" x]

// Emit branch: jump to tgt if bexpr true, else jump to fls
// After BiGCL, guards are simple comparisons, Not(comparison), True, False,
// or AndExpr (from doneGuard of Do loops)
let rec emitBranch (b: bexpr) (tgt: string) (fls: string) : string list =
    match b with
    | True  -> [sprintf "    j %s" tgt]
    | False -> [sprintf "    j %s" fls]
    | NotExpr True  -> [sprintf "    j %s" fls]
    | NotExpr False -> [sprintf "    j %s" tgt]
    | AndExpr(l, r) ->
        let midLabel = freshLabel "and_mid"
        emitBranch l midLabel fls @
        [sprintf "%s:" midLabel] @
        emitBranch r tgt fls
    | OrExpr(l, r) ->
        let midLabel = freshLabel "or_mid"
        emitBranch l tgt midLabel @
        [sprintf "%s:" midLabel] @
        emitBranch r tgt fls
    | EqExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    beq t0, t1, %s" tgt; sprintf "    j %s" fls]
    | NeqExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    bne t0, t1, %s" tgt; sprintf "    j %s" fls]
    | LtExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    blt t0, t1, %s" tgt; sprintf "    j %s" fls]
    | GteExpr(l, r) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    bge t0, t1, %s" tgt; sprintf "    j %s" fls]
    | GtExpr(l, r) ->
        loadAtom "t0" r @ loadAtom "t1" l @
        [sprintf "    blt t0, t1, %s" tgt; sprintf "    j %s" fls]
    | LteExpr(l, r) ->
        loadAtom "t0" r @ loadAtom "t1" l @
        [sprintf "    bge t0, t1, %s" tgt; sprintf "    j %s" fls]
    | NotExpr(EqExpr(l, r)) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    bne t0, t1, %s" tgt; sprintf "    j %s" fls]
    | NotExpr(NeqExpr(l, r)) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    beq t0, t1, %s" tgt; sprintf "    j %s" fls]
    | NotExpr(LtExpr(l, r)) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    bge t0, t1, %s" tgt; sprintf "    j %s" fls]
    | NotExpr(GtExpr(l, r)) ->
        loadAtom "t0" r @ loadAtom "t1" l @
        [sprintf "    bge t0, t1, %s" tgt; sprintf "    j %s" fls]
    | NotExpr(GteExpr(l, r)) ->
        loadAtom "t0" l @ loadAtom "t1" r @
        [sprintf "    blt t0, t1, %s" tgt; sprintf "    j %s" fls]
    | NotExpr(LteExpr(l, r)) ->
        loadAtom "t0" r @ loadAtom "t1" l @
        [sprintf "    blt t0, t1, %s" tgt; sprintf "    j %s" fls]
    | _ ->
        [sprintf "    # Unhandled bexpr"; sprintf "    j %s" fls]

// ── Assembly layout ───────────────────────────────────────────────────────────

let groupBySource (edges: Edge list) : Map<Node, (Label * Node) list> =
    edges
    |> List.groupBy (fun (q, _, _) -> q)
    |> List.map (fun (k, es) -> k, List.map (fun (_, l, q') -> (l, q')) es)
    |> Map.ofList

let allNodes (qs: Node) (qe: Node) (edges: Edge list) : Node list =
    let all =
        edges
        |> List.collect (fun (q, _, q') -> [q; q'])
        |> List.distinct
    let middle = all |> List.filter (fun n -> n <> qs && n <> qe)
    [qs] @ middle @ [qe]

let generateAsm (qs: Node) (qe: Node) (edges: Edge list) (vars: Set<string>) : string =
    let sb = System.Text.StringBuilder()
    let emit (s: string) = sb.AppendLine(s) |> ignore

    emit ".data"
    for v in vars do
        emit (sprintf "%s: .word 0" (vname v))
    emit ""
    emit ".text"
    emit ".globl main"
    emit "main:"
    emit (sprintf "    j %s" qs)
    emit ""

    let grouped = groupBySource edges
    let nodes   = allNodes qs qe edges

    for node in nodes do
        emit (sprintf "%s:" node)
        if node = qe then
            emit "    li a0, 0"
            emit "    li a7, 10"
            emit "    ecall"
        else
            match Map.tryFind node grouped with
            | None ->
                emit (sprintf "    j %s" qe)

            | Some [(LAssign(x, e), next)] ->
                for i in emitAssign x e do emit i
                emit (sprintf "    j %s" next)

            | Some [(LArrAssign(a, _, _), next)] ->
                emit (sprintf "    # array assign %s not supported" a)
                emit (sprintf "    j %s" next)

            | Some [(LSkip, next)] ->
                emit (sprintf "    j %s" next)

            | Some [(LBool b, next)] ->
                for i in emitBranch b next qe do emit i

            | Some [(LBool b1, next1); (LBool b2, next2)] ->
                for i in emitBranch b1 next1 next2 do emit i

            | Some outgoing ->
                // Multiple outgoing edges: emit sequentially
                // Each boolean edge branches to its target or falls to next check
                let rec emitOutgoing remaining =
                    match remaining with
                    | [] -> ()
                    | [(label, next)] ->
                        match label with
                        | LAssign(x, e) ->
                            for i in emitAssign x e do emit i
                            emit (sprintf "    j %s" next)
                        | LSkip -> emit (sprintf "    j %s" next)
                        | LBool b ->
                            for i in emitBranch b next qe do emit i
                        | LArrAssign(a, _, _) ->
                            emit (sprintf "    # array assign %s" a)
                            emit (sprintf "    j %s" next)
                    | (label, next) :: rest ->
                        let nextCheck =
                            match rest.Head with
                            | (_, n) -> n
                        match label with
                        | LBool b ->
                            for i in emitBranch b next nextCheck do emit i
                        | LAssign(x, e) ->
                            for i in emitAssign x e do emit i
                            emit (sprintf "    j %s" next)
                        | LSkip -> emit (sprintf "    j %s" next)
                        | LArrAssign(a, _, _) ->
                            emit (sprintf "    # array assign %s" a)
                            emit (sprintf "    j %s" next)
                        emitOutgoing rest
                emitOutgoing outgoing
        emit ""

    sb.ToString()

// ── Entry point ───────────────────────────────────────────────────────────────

let analysis (input: Input) : Output =
    tmpCount <- 0
    nodeCount <- 0
    inlineLabelCount <- 0
    match Parser.parse Grammar.start_commands input.commands with
    | Ok ast ->
        let simplified = flattenCmd ast
        let vars       = collectVarsCmd simplified
        let qs = "q0"
        let qe = "qf"
        let edges = buildEdges simplified qs qe
        let asm   = generateAsm qs qe edges vars
        { assembly = asm }
    | Error e ->
        { assembly = sprintf "# Parse error: %A" e }
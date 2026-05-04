module RiscV
open Io.RiscV
open AST

let parse src =
    let lexbuf = FSharp.Text.Lexing.LexBuffer<char>.FromString src
    try Ok(Grammar.start_commands Lexer.tokenize lexbuf)
    with e -> Error e

let indent s = "\t" + s


let rec collectVarsInExpr (e: expr) : Set<string> =
    match e with
    | Var x -> Set.singleton x
    | Num _ -> Set.empty
    | PlusExpr(e1,e2) | MinusExpr(e1,e2) | TimesExpr(e1,e2) 
    | DivExpr(e1,e2)  | PowExpr(e1,e2) -> 
        Set.union (collectVarsInExpr e1) (collectVarsInExpr e2)
    | UMinusExpr e -> collectVarsInExpr e
    | Array(a, e) -> Set.add a (collectVarsInExpr e)

let rec collectVarsInBExpr (b: bExpr) : Set<string> =
    match b with
    | True | False -> Set.empty
    | NotExpr b -> collectVarsInBExpr b
    | EqExpr(e1,e2) | NeqExpr(e1,e2) | GtExpr(e1,e2) 
    | GteExpr(e1,e2) | LtExpr(e1,e2) | LteExpr(e1,e2) ->
        Set.union (collectVarsInExpr e1) (collectVarsInExpr e2)
    | AndExpr(b1,b2) | OrExpr(b1,b2) | AndAndExpr(b1,b2) | OrOrExpr(b1,b2) ->
        Set.union (collectVarsInBExpr b1) (collectVarsInBExpr b2)
        
let rec collectVars (cmd: commands) : Set<string> =
    match cmd with
    | Skip -> Set.empty
    | Assign(x,e) ->
        Set.add x (collectVarsInExpr e)
    | ArrayAssign(a,i,e) ->
        Set.add a (Set.union (collectVarsInExpr i) (collectVarsInExpr e))
    | Sequence(c1,c2) ->
        Set.union (collectVars c1) (collectVars c2)
    | If gcs | Do gcs ->
        gcs
        |> List.map (fun (GCommand(b,c)) ->
            Set.union (collectVarsInBExpr b) (collectVars c))
        |> List.fold Set.union Set.empty

let qCounter = ref 0
let freshQ () =
    qCounter := !qCounter + 1
    $"q{!qCounter}"

let compileStuck (qIn: string) (qOut: string) : string list =
    [ $"{qIn}:"
      indent $"la t0, vstuck_"
      indent $"li t1, 1"
      indent $"li t2, 0"
      indent $"div t1, t1, t2"
      indent $"sw t1, 0(t0)"
      indent $"j {qOut}" ]

let loadLeaf (e: expr) (reg: string) : string list =
    match e with
    | Num n -> [ indent $"li {reg}, {n}" ]
    | Var x -> [ indent $"lw {reg}, v{x}" ]
    | _ -> failwith $"loadLeaf not a leaf {e}"

let compileExpr (e: expr) (reg: string) : string list =
    match e with
    | Num _ | Var _ -> loadLeaf e reg

    | PlusExpr(l1, l2) ->
        loadLeaf l1 "t1"
        @ loadLeaf l2 "t2"
        @ [ indent $"add {reg}, t1, t2" ]

    | MinusExpr(l1, l2) ->
        loadLeaf l1 "t1"
        @ loadLeaf l2 "t2"
        @ [ indent $"sub {reg}, t1, t2" ]

    | TimesExpr(l1, l2) ->
        loadLeaf l1 "t1"
        @ loadLeaf l2 "t2"
        @ [ indent $"mul {reg}, t1, t2" ]

    | DivExpr(l1, l2) ->
        loadLeaf l1 "t1"
        @ loadLeaf l2 "t2"
        @ [ indent $"div {reg}, t1, t2" ]

    | UMinusExpr(Num n) ->
        [ indent $"li {reg}, {n}"
          indent $"neg {reg}, {reg}" ]

    | UMinusExpr l ->
        loadLeaf l "t1"
        @ [ indent $"neg {reg}, t1" ]

    | Array(a, i) ->
        loadLeaf i "t1"
        @ [ indent $"la t6, v{a}"
            indent $"slli t1, t1, 2"
            indent $"add t6, t6, t1"
            indent $"lw {reg}, 0(t6)" ]

    | PowExpr(_, _) ->
        failwith "PowExpr not supported in RiscV backend"

let rec compileCond (b: bExpr) (qTrue: string) (qFalse: string) : string list =
    match b with
    | True ->
        [ indent $"j {qTrue}" ]

    | False ->
        [ indent $"j {qFalse}" ]

    | EqExpr(l1, l2) ->
        loadLeaf l1 "t0"
        @ loadLeaf l2 "t1"
        @ [ indent $"beq t0, t1, {qTrue}"
            indent $"j {qFalse}" ]

    | NeqExpr(l1, l2) ->
        loadLeaf l1 "t0"
        @ loadLeaf l2 "t1"
        @ [ indent $"bne t0, t1, {qTrue}"
            indent $"j {qFalse}" ]

    | LtExpr(l1, l2) ->
        loadLeaf l1 "t0"
        @ loadLeaf l2 "t1"
        @ [ indent $"blt t0, t1, {qTrue}"
            indent $"j {qFalse}" ]

    | GtExpr(l1, l2) ->
        loadLeaf l2 "t0"
        @ loadLeaf l1 "t1"
        @ [ indent $"blt t0, t1, {qTrue}"
            indent $"j {qFalse}" ]

    | LteExpr(l1, l2) ->
        loadLeaf l2 "t0"
        @ loadLeaf l1 "t1"
        @ [ indent $"blt t0, t1, {qFalse}"
            indent $"j {qTrue}" ]

    | GteExpr(l1, l2) ->
        loadLeaf l1 "t0"
        @ loadLeaf l2 "t1"
        @ [ indent $"blt t0, t1, {qFalse}"
            indent $"j {qTrue}" ]

    | NotExpr b1 ->
        compileCond b1 qFalse qTrue

    | AndExpr(b1, b2)
    | AndAndExpr(b1, b2) ->
        let qMid = freshQ()
        compileCond b1 qMid qFalse
        @ [ $"{qMid}:" ]
        @ compileCond b2 qTrue qFalse

    | OrExpr(b1, b2)
    | OrOrExpr(b1, b2) ->
        let qMid = freshQ()
        compileCond b1 qTrue qMid
        @ [ $"{qMid}:" ]
        @ compileCond b2 qTrue qFalse

and compileCmd (cmd: commands) (qIn: string) (qOut: string) : string list =
    match cmd with
    | Skip ->
        [ $"{qIn}:"
          indent $"j {qOut}" ]

    | Assign(x, e) ->
        [ $"{qIn}:"
          indent $"la t0, v{x}" ]
        @ compileExpr e "t1"
        @ [ indent $"sw t1, 0(t0)"
            indent $"j {qOut}" ]

    | ArrayAssign(a, i, e) ->
        [ $"{qIn}:" ]
        @ compileExpr i "t1"
        @ [ indent $"la t6, v{a}"
            indent $"slli t1, t1, 2"
            indent $"add t6, t6, t1" ]
        @ compileExpr e "t2"
        @ [ indent $"sw t2, 0(t6)"
            indent $"j {qOut}" ]

    | Sequence(c1, c2) ->
        let qMid = freshQ()
        compileCmd c1 qIn qMid
        @ compileCmd c2 qMid qOut

    | If gcs ->
        let qStuck = freshQ()
        compileIfGCs gcs qIn qOut qStuck
        @ compileStuck qStuck qOut

    | Do gcs ->
        compileDoGCs gcs qIn qOut

and compileIfGCs (gcs: gCommand list) (qIn: string) (qOut: string) (qStuck: string) : string list =
    match gcs with
    | [] ->
        [ $"{qIn}:"
          indent $"j {qStuck}" ]

    | [GCommand(b, c)] ->
        let qBody = freshQ()
        [ $"{qIn}:" ]
        @ compileCond b qBody qStuck
        @ compileCmd c qBody qOut

    | GCommand(b, c) :: rest ->
        let qBody = freshQ()
        let qNext = freshQ()
        [ $"{qIn}:" ]
        @ compileCond b qBody qNext
        @ compileCmd c qBody qOut
        @ compileIfGCs rest qNext qOut qStuck

and compileDoGCs (gcs: gCommand list) (qIn: string) (qOut: string) : string list =
    match gcs with
    | [] ->
        [ $"{qIn}:"
          indent $"j {qOut}" ]

    | [GCommand(b, c)] ->
        let qBody = freshQ()
        [ $"{qIn}:" ]
        @ compileCond b qBody qOut
        @ compileCmd c qBody qIn

    | GCommand(b, c) :: rest ->
        let qBody = freshQ()
        let qNext = freshQ()
        [ $"{qIn}:" ]
        @ compileCond b qBody qNext
        @ compileCmd c qBody qIn
        @ compileDoGCs rest qNext qOut

let analysis (input: Input) : Output =
    qCounter := 0
    BiGCL.tmp := 0

    let ast =
        match parse input.commands with
        | Ok a -> a
        | Error e -> failwith $"Parse error {e}"

    let binaryAst = BiGCL.BinCom ast

    let vars =
        collectVars binaryAst
        |> Set.add "stuck_"
        |> Set.toList

    let dataSection =
        ".data" :: (vars |> List.map (fun v -> $"v{v}:\t\t.word 0"))

    let textSection =
        [ ".text" ]
        @ compileCmd binaryAst "qStart" "qFinal"
        @ [ "qFinal:"
            indent "li a7, 10"
            indent "ecall" ]

    { assembly = String.concat "\n" (dataSection @ textSection) }  

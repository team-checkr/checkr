module BiGCL
open Io.BiGCL
open AST
open Grammar
open Parser

let tmp = ref 0
let freshTmp () = 
    let n = !tmp
    tmp := !tmp + 1 
    "tmp" + string n + "_"

let noSkip c1 c2 =
    match c1,c2 with
    | Skip, c -> c
    | c, Skip -> c
    | _ -> Sequence(c1,c2)

let IsLeaf L =
    match L with
    | Num _ 
    | Var _
    | UMinusExpr(Num _) -> true    
    | _ -> false

let rec removeNot = function
    | NotExpr True  -> False
    | NotExpr False -> True
    | NotExpr (NotExpr b)           -> removeNot b
    | NotExpr (AndExpr(b1,b2))      -> OrExpr    (removeNot (NotExpr b1), removeNot (NotExpr b2))
    | NotExpr (OrExpr(b1,b2))       -> AndExpr   (removeNot (NotExpr b1), removeNot (NotExpr b2))
    | NotExpr (AndAndExpr(b1,b2))   -> OrOrExpr  (removeNot (NotExpr b1), removeNot (NotExpr b2))
    | NotExpr (OrOrExpr(b1,b2))     -> AndAndExpr(removeNot (NotExpr b1), removeNot (NotExpr b2))
    | NotExpr (EqExpr(e1,e2))       -> NeqExpr(e1,e2)
    | NotExpr (NeqExpr(e1,e2))      -> EqExpr(e1,e2)
    | NotExpr (LtExpr(e1,e2))       -> GteExpr(e1,e2)
    | NotExpr (GtExpr(e1,e2))       -> LteExpr(e1,e2)
    | NotExpr (LteExpr(e1,e2))      -> GtExpr(e1,e2)
    | NotExpr (GteExpr(e1,e2))      -> LtExpr(e1,e2)
    | AndExpr(b1,b2)    -> AndExpr   (removeNot b1, removeNot b2)
    | OrExpr(b1,b2)     -> OrExpr    (removeNot b1, removeNot b2)
    | AndAndExpr(b1,b2) -> AndAndExpr(removeNot b1, removeNot b2)
    | OrOrExpr(b1,b2)   -> OrOrExpr  (removeNot b1, removeNot b2)
    | b -> b  

let rec toLeaf (e : expr) : commands * expr =
    match e with
    | UMinusExpr(Num n) -> (Skip, Num(-n))  // Treat as a leaf, return simplified form
    | _ when IsLeaf e -> (Skip, e)
    | _ -> 
        let (pre, simpl) = Binexpr e
        let t = freshTmp ()
        (noSkip pre (Assign(t, simpl)), Var t)

and Binexpr expr =
    match expr with
    | Num n-> (Skip, expr)
    | Var v -> (Skip, expr)
    | Array(a, i) ->
        let (pre, leaf) = toLeaf i
        (pre, Array(a, leaf))
    | UMinusExpr(Num n) -> (Skip, Num(-n))
    | UMinusExpr inner ->
        let (pre, leaf) = toLeaf inner
        (pre, MinusExpr(Num 0, leaf))
    | PlusExpr (e1, e2) -> binBop PlusExpr e1 e2
    | MinusExpr(e1,e2) -> binBop MinusExpr e1 e2
    | TimesExpr(e1,e2) -> binBop TimesExpr e1 e2
    | DivExpr(e1,e2)   -> binBop DivExpr   e1 e2
    | PowExpr(e1,e2)   -> binBop PowExpr   e1 e2       

and binBop com e1 e2 =
    let (p1, l1) = toLeaf e1
    let (p2, l2) = toLeaf e2
    (noSkip p1 p2, com(l1,l2))

let binAtomicBool (b: bExpr) : commands * bExpr =
    let go ctor e1 e2 =
        let (p1, l1) = toLeaf e1
        let (p2, l2) = toLeaf e2
        (noSkip p1 p2, ctor(l1, l2))
    match b with
    | True | False        -> (Skip, b)
    | EqExpr(e1,e2)       -> go EqExpr  e1 e2
    | NeqExpr(e1,e2)      -> go NeqExpr e1 e2
    | LtExpr(e1,e2)       -> go LtExpr  e1 e2
    | GtExpr(e1,e2)       -> go GtExpr  e1 e2
    | LteExpr(e1,e2)      -> go LteExpr e1 e2
    | GteExpr(e1,e2)      -> go GteExpr e1 e2
    | _ -> failwith "binAtomicBool error"

let binTwo e1 e2 =
    let (p1, l1) = toLeaf e1
    let (p2, l2) = toLeaf e2
    (noSkip p1 p2, l1, l2)

let negateGuard = function
    | NotExpr b -> b       
    | b         -> NotExpr b

let rec boolToFlag (b: bExpr) (f: string) : commands =
    match b with  
    | True  -> Assign(f, Num 1)   
    | False -> Assign(f, Num 0)   
    | NotExpr b' ->
        noSkip (boolToFlag b' f)
               (Assign(f, MinusExpr(Num 1, Var f)))   
        
    | AndAndExpr(b1, b2) ->
        let (pre, sb1) = binGuardBool b1
        noSkip pre
                (If [GCommand(sb1,          boolToFlag b2 f);
                    GCommand(negateGuard  sb1,  Assign(f, Num 0))])
        
    | AndExpr(b1,b2)  ->
        let f1 = freshTmp()
        let f2 = freshTmp()
        noSkip (boolToFlag b1 f1)                          
                (noSkip (boolToFlag b2 f2)                 
                        (Assign(f, TimesExpr(Var f1, Var f2))))

    | OrExpr(b1,b2) ->
        let f1 = freshTmp()
        let f2 = freshTmp()
        noSkip (boolToFlag b1 f1)
                (noSkip (boolToFlag b2 f2)
                (noSkip (Assign(f, PlusExpr(Var f1, Var f2)))  
                (noSkip (Assign(f, PlusExpr(Var f, Num 1)))    
                            (Assign(f, DivExpr(Var f, Num 2))))))  
    
    | OrOrExpr(b1,b2) ->
        let (pre, sb1) = binGuardBool b1
        noSkip pre
                    (If [GCommand(sb1,          Assign(f, Num 1));
                             GCommand(negateGuard  sb1,  boolToFlag b2 f)])

    | b' ->
        let (pre, sb) = binAtomicBool b'
        noSkip pre
               (If [GCommand(sb,          Assign(f, Num 1));
                    GCommand(NotExpr sb,  Assign(f, Num 0))])

and binGuardBool (b: bExpr) : commands * bExpr =
    match b with
    | True | False -> (Skip, b)
    | NotExpr(NotExpr b') -> binGuardBool b'  
    | NotExpr b' ->
        let (pre, sb) = binGuardBool b'
        (pre, NotExpr sb)
    | EqExpr(e1,e2)  -> let (p,l1,l2) = binTwo e1 e2 in (p, EqExpr(l1,l2))
    | NeqExpr(e1,e2) -> let (p,l1,l2) = binTwo e1 e2 in (p, NeqExpr(l1,l2))
    | LtExpr(e1,e2)  -> let (p,l1,l2) = binTwo e1 e2 in (p, LtExpr(l1,l2))
    | GtExpr(e1,e2)  -> let (p,l1,l2) = binTwo e1 e2 in (p, GtExpr(l1,l2))
    | LteExpr(e1,e2) -> let (p,l1,l2) = binTwo e1 e2 in (p, LteExpr(l1,l2))
    | GteExpr(e1,e2) -> let (p,l1,l2) = binTwo e1 e2 in (p, GteExpr(l1,l2))
    | _ -> 
        let f = freshTmp()
        let pre = boolToFlag b f
        (pre, EqExpr(Var f, Num 1))
   
let isAtomicGuard = function
    | True | False
    | EqExpr _ | NeqExpr _
    | LtExpr _ | GtExpr _
    | LteExpr _ | GteExpr _ -> true
    | _ -> false

let rec anyGuardTrue (gcs: gCommand list) (f: string) : commands =
    match gcs with
    | [] -> Assign(f, Num 0)
    | [GCommand(b, _)] ->
        boolToFlag b f    
    | GCommand(b, _) :: rest ->
        let flagCode = boolToFlag b f
        noSkip flagCode
               (If [GCommand(EqExpr(Var f, Num 1),
                              Assign(f, Num 1));          
                    GCommand(NotExpr(EqExpr(Var f, Num 1)),
                              anyGuardTrue rest f)])      
let rec binGCsToChain (gcs: gCommand list) : commands =
    match gcs with
    | [] ->
        Assign("stuck_", DivExpr(Num 1, Num 0))  
    | [GCommand(b, c)] when isAtomicGuard b ->
        let (pre, sb) = binAtomicBool b
        let c' = BinCom c
        noSkip pre
               (If [GCommand(sb,          c');
                    GCommand(NotExpr sb,  Assign("stuck_", DivExpr(Num 1, Num 0)))])

    | [GCommand(b, c)] ->
        let f = freshTmp()
        noSkip (boolToFlag b f)
               (If [GCommand(EqExpr(Var f, Num 1),  BinCom c);
                    GCommand(NotExpr(EqExpr(Var f, Num 1)),  Assign("stuck_", DivExpr(Num 1, Num 0)))])

    | GCommand(b, c) :: rest ->
        let restCode = binGCsToChain rest
        let f = freshTmp()
        noSkip (boolToFlag b f)
               (If [GCommand(EqExpr(Var f, Num 1),  BinCom c);
                    GCommand(NotExpr(EqExpr(Var f, Num 1)),  restCode)])   

and BinCom (c: commands) : commands =
    match c with
    | Skip -> Skip
    | Assign(x, e) ->
        let (pre, leaf) = toLeaf e
        noSkip pre (Assign(x, leaf))
    | Sequence(c1, c2) -> noSkip (BinCom c1) (BinCom c2)
    | If gcs ->
        binGCsToChain gcs    
    | Do gcs ->
        let f = freshTmp()
        let flagCode = anyGuardTrue gcs f
        let body = 
                noSkip (binGCsToChain gcs) flagCode
        noSkip flagCode (Do [GCommand(EqExpr(Var f, Num 1), body)])
    | ArrayAssign (a, i, e) ->
        let (preI, leafI) = toLeaf i
        let (preE, leafE) = toLeaf e
        noSkip preI (noSkip preE (ArrayAssign(a, leafI, leafE)))

let analysis (input: Input) : Output =
    tmp := 0
    match parse Grammar.start_commands input.commands with
        | Ok ast -> { binary = prettyPrint (BinCom ast) }
        | Error e -> failwith "Parser error :("
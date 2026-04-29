module BiGCL
open Io.BiGCL
open FSharp.Text.Lexing
open System
open AST

exception ParseError of Position * string * Exception

let parse parser src =
    let lexbuf = LexBuffer<char>.FromString src

    let parser = parser Lexer.tokenize

    try
        Ok(parser lexbuf)
    with
    | e ->
        let pos = lexbuf.EndPos
        let lastToken = new String(lexbuf.Lexeme)
        eprintf "Parse failed at line %d, column %d:\n" pos.Line pos.Column
        eprintf "Last token: %s\n" lastToken
        Error(ParseError(pos, lastToken, e))

let commandListToSemi (commands: command list) : command =
    match commands with
    | [] -> Skip
    | head :: tail -> List.fold (fun acc cmd -> Semi(acc, cmd)) head tail

let rec exprVariables (expression: expr) : Set<string> =
    match expression with
    | Num _ -> Set.empty
    | VarExpr variable -> variableNames variable
    | TimesExpr (left, right)
    | DivExpr (left, right)
    | PlusExpr (left, right)
    | MinusExpr (left, right)
    | PowExpr (left, right) ->
        Set.union (exprVariables left) (exprVariables right)
    | UMinusExpr inner ->
        exprVariables inner

and variableNames (variable: variable) : Set<string> =
    match variable with
    | Var name -> Set.singleton name
    | List (name, index) ->
        Set.add name (exprVariables index)

let rec boolVariables (guard: boolexpr) : Set<string> =
    match guard with
    | Bool _ -> Set.empty
    | And (left, right)
    | Or (left, right)
    | BitAnd (left, right)
    | BitOr (left, right) ->
        Set.union (boolVariables left) (boolVariables right)
    | Equal (left, right)
    | NotEq (left, right)
    | SmallerThan (left, right)
    | GreaterThan (left, right)
    | SmallerEq (left, right)
    | GreaterEq (left, right) ->
        Set.union (exprVariables left) (exprVariables right)
    | Not inner ->
        boolVariables inner

let rec commandVariables (cmd: command) : Set<string> =
    match cmd with
    | Skip -> Set.empty
    | Assign (target, expression) ->
        Set.union (variableNames target) (exprVariables expression)
    | Semi (first, second) ->
        Set.union (commandVariables first) (commandVariables second)
    | If guards
    | Loop guards ->
        guards
        |> List.fold (fun names (guard, body) ->
            names
            |> Set.union (boolVariables guard)
            |> Set.union (commandVariables body)
        ) Set.empty

let initialTempCounter (cmd: command) : int =
    commandVariables cmd
    |> Seq.choose (fun name ->
        if name.StartsWith("tmp_") then
            match Int32.TryParse(name.Substring(4)) with
            | true, value -> Some(value + 1)
            | false, _ -> None
        elif name.StartsWith("tmp") && name.EndsWith("_") then
            match Int32.TryParse(name.Substring(3, name.Length - 4)) with
            | true, value -> Some(value + 1)
            | false, _ -> None
        else
            None
    )
    |> Seq.fold max 0

let freshTemp (counter: int) : variable * int =
    Var(sprintf "tmp%d_" counter), counter + 1

let freshStuckName (cmd: command) : string =
    let usedNames = commandVariables cmd
    if not (Set.contains "stuck_" usedNames) then
        "stuck_"
    else
        Seq.initInfinite (fun index -> sprintf "stuck_%d" index)
        |> Seq.find (fun name -> not (Set.contains name usedNames))

let stuckCommand (name: string) : command =
    Assign(Var name, DivExpr(Num 1, Num 0))

let rec isNonNegativeAtomicExpr (expr: expr) : bool =
    match expr with
    | Num value when value >= 0 -> true
    | VarExpr _ -> true
    | _ -> false

let rec isSimpleExpr (expr: expr) : bool =
    match expr with
    | Num _
    | VarExpr _ -> true
    | UMinusExpr inner -> isSimpleExpr inner
    | _ -> false

let isAtomicExpr (expr: expr) : bool =
    match expr with
    | Num _
    | VarExpr _ -> true
    | _ -> false

let isDirectArithmeticExpr (expr: expr) : bool =
    match expr with
    | Num _
    | VarExpr _ -> true
    | UMinusExpr inner -> isAtomicExpr inner
    | TimesExpr (left, right)
    | DivExpr (left, right)
    | PlusExpr (left, right)
    | MinusExpr (left, right)
    | PowExpr (left, right) ->
        isAtomicExpr left && isAtomicExpr right
    | _ -> false

let rec isDirectGuard (guard: boolexpr) : bool =
    match guard with
    | Bool _ -> true
    | Equal (left, right)
    | NotEq (left, right)
    | SmallerThan (left, right)
    | GreaterThan (left, right)
    | SmallerEq (left, right)
    | GreaterEq (left, right) ->
        isNonNegativeAtomicExpr left && isNonNegativeAtomicExpr right
    | Not _ ->
        false
    | And _
    | Or _
    | BitAnd _
    | BitOr _ -> false

let rec simplifyExpr (expr: expr) (counter: int) : command list * expr * int =
    match expr with
    | Num _
    | VarExpr _ ->
        [], expr, counter
    | UMinusExpr inner ->
        let prefix, simplifiedInner, nextCounter = simplifyExpr inner counter
        match simplifiedInner with
        | Num value ->
            prefix, Num(-value), nextCounter
        | UMinusExpr nested ->
            prefix, nested, nextCounter
        | VarExpr _ ->
            prefix, UMinusExpr simplifiedInner, nextCounter
        | _ ->
            let tmp, finalCounter = freshTemp nextCounter
            prefix @ [Assign(tmp, simplifiedInner)], UMinusExpr(VarExpr tmp), finalCounter
    | TimesExpr (left, right) ->
        simplifyBinaryExpr TimesExpr left right counter
    | DivExpr (left, right) ->
        simplifyBinaryExpr DivExpr left right counter
    | PlusExpr (left, right) ->
        simplifyBinaryExpr PlusExpr left right counter
    | MinusExpr (left, right) ->
        simplifyBinaryExpr MinusExpr left right counter
    | PowExpr (left, right) ->
        simplifyBinaryExpr PowExpr left right counter

and simplifyBinaryExpr
    (mkExpr: expr * expr -> expr)
    (left: expr)
    (right: expr)
    (counter: int)
    : command list * expr * int =
    let leftPrefix, simpleLeft, counter2 = ensureDirectArithmeticExpr left counter
    let rightPrefix, simpleRight, counter4 = ensureDirectArithmeticExpr right counter2
    leftPrefix @ rightPrefix, mkExpr(simpleLeft, simpleRight), counter4

and ensureSimpleExpr (expr: expr) (counter: int) : command list * expr * int =
    let prefix, simplifiedExpr, nextCounter = simplifyExpr expr counter
    if isSimpleExpr simplifiedExpr then
        prefix, simplifiedExpr, nextCounter
    else
        let tmp, finalCounter = freshTemp nextCounter
        prefix @ [Assign(tmp, simplifiedExpr)], VarExpr tmp, finalCounter

and ensureDirectArithmeticExpr (expr: expr) (counter: int) : command list * expr * int =
    let prefix, simplifiedExpr, nextCounter = simplifyExpr expr counter
    if isNonNegativeAtomicExpr simplifiedExpr then
        prefix, simplifiedExpr, nextCounter
    else
        let tmp, finalCounter = freshTemp nextCounter
        prefix @ [Assign(tmp, simplifiedExpr)], VarExpr tmp, finalCounter

and simplifyTargetVariable (variable: variable) (counter: int) : command list * variable * int =
    match variable with
    | Var _ -> [], variable, counter
    | List (name, index) ->
        let prefix, simpleIndex, nextCounter = ensureDirectArithmeticExpr index counter
        prefix, List(name, simpleIndex), nextCounter

let rec binarizeExprInto (target: variable) (expr: expr) (counter: int) : command list * int =
    let targetPrefix, simplifiedTarget, counter1 = simplifyTargetVariable target counter
    let exprPrefix, simpleExpr, nextCounter = simplifyExpr expr counter1
    if isAtomicExpr simpleExpr || (List.isEmpty exprPrefix && isDirectArithmeticExpr simpleExpr) then
        targetPrefix @ exprPrefix @ [Assign(simplifiedTarget, simpleExpr)], nextCounter
    else
        let tmp, finalCounter = freshTemp nextCounter
        targetPrefix @ exprPrefix @ [Assign(tmp, simpleExpr); Assign(simplifiedTarget, VarExpr tmp)], finalCounter

let rec simplifyComparisonOperands
    (left: expr)
    (right: expr)
    (counter: int)
    : command list * expr * expr * int =
    let leftPrefix, directLeft, counter2 = ensureDirectArithmeticExpr left counter
    let rightPrefix, directRight, counter3 = ensureDirectArithmeticExpr right counter2
    leftPrefix @ rightPrefix, directLeft, directRight, counter3

let appendCommand (first: command) (second: command) : command =
    match first, second with
    | Skip, _ -> second
    | _, Skip -> first
    | _ -> Semi(first, second)

let guardFromFlag (flagVar: variable) : boolexpr =
    Equal(VarExpr flagVar, Num 1)

let branchOnFlag (flagVar: variable) (onTrue: command) (onFalse: command) : command =
    let guard = guardFromFlag flagVar
    If [ (guard, onTrue); (Not guard, onFalse) ]

let rec boolToFlag (guard: boolexpr) (counter: int) : command list * variable * int =
    match guard with
    | Bool true ->
        let flagVar, nextCounter = freshTemp counter
        [Assign(flagVar, Num 1)], flagVar, nextCounter
    | Bool false ->
        let flagVar, nextCounter = freshTemp counter
        [Assign(flagVar, Num 0)], flagVar, nextCounter
    | Equal (left, right) ->
        comparisonToFlag Equal left right counter
    | NotEq (left, right) ->
        comparisonToFlag NotEq left right counter
    | SmallerThan (left, right) ->
        comparisonToFlag SmallerThan left right counter
    | GreaterThan (left, right) ->
        comparisonToFlag GreaterThan left right counter
    | SmallerEq (left, right) ->
        comparisonToFlag SmallerEq left right counter
    | GreaterEq (left, right) ->
        comparisonToFlag GreaterEq left right counter
    | Not inner ->
        let commands, flagVar, nextCounter = boolToFlag inner counter
        commands @ [Assign(flagVar, MinusExpr(Num 1, VarExpr flagVar))], flagVar, nextCounter
    | And (left, right) ->
        shortCircuitAndToFlag left right counter
    | BitAnd (left, right) ->
        andToFlag left right counter
    | Or (left, right) ->
        shortCircuitOrToFlag left right counter
    | BitOr (left, right) ->
        orToFlag left right counter

and comparisonToFlag
    (mkBool: expr * expr -> boolexpr)
    (left: expr)
    (right: expr)
    (counter: int)
    : command list * variable * int =
    let flagVar, counter1 = freshTemp counter
    let prefix, leftExpr, rightExpr, counter2 = simplifyComparisonOperands left right counter1
    let comparison = mkBool(leftExpr, rightExpr)
    let guardCommand =
        If [
            (comparison, Assign(flagVar, Num 1))
            (Not comparison, Assign(flagVar, Num 0))
        ]
    prefix @ [guardCommand], flagVar, counter2

and andToFlag (left: boolexpr) (right: boolexpr) (counter: int) : command list * variable * int =
    let leftCommands, leftFlag, counter1 = boolToFlag left counter
    let rightCommands, rightFlag, counter2 = boolToFlag right counter1
    let resultFlag, counter3 = freshTemp counter2
    let commands =
        leftCommands
        @ rightCommands
        @ [Assign(resultFlag, TimesExpr(VarExpr leftFlag, VarExpr rightFlag))]
    commands, resultFlag, counter3

and assignFlagFromGuard
    (guard: boolexpr)
    (flagVar: variable)
    (counter: int)
    : command * int =
    if isDirectGuard guard then
        If [ (guard, Assign(flagVar, Num 1)); (Not guard, Assign(flagVar, Num 0)) ], counter
    else
        let guardCommands, guardFlag, nextCounter = boolToFlag guard counter
        appendCommand
            (commandListToSemi guardCommands)
            (branchOnFlag guardFlag (Assign(flagVar, Num 1)) (Assign(flagVar, Num 0))),
        nextCounter

and shortCircuitAndToFlag
    (left: boolexpr)
    (right: boolexpr)
    (counter: int)
    : command list * variable * int =
    let resultFlag, counter1 = freshTemp counter
    let rightBranch, counter2 = assignFlagFromGuard right resultFlag counter1
    let falseBranch = Assign(resultFlag, Num 0)
    if isDirectGuard left then
        [If [ (left, rightBranch); (Not left, falseBranch) ]], resultFlag, counter2
    else
        let leftCommands, leftFlag, counter3 = boolToFlag left counter1
        leftCommands @ [branchOnFlag leftFlag rightBranch falseBranch], resultFlag, max counter2 counter3

and orToFlag (left: boolexpr) (right: boolexpr) (counter: int) : command list * variable * int =
    let leftCommands, leftFlag, counter1 = boolToFlag left counter
    let rightCommands, rightFlag, counter2 = boolToFlag right counter1
    let resultFlag, counter3 = freshTemp counter2
    let commands =
        leftCommands
        @ rightCommands
        @ [Assign(resultFlag, PlusExpr(VarExpr leftFlag, VarExpr rightFlag))]
        @ [Assign(resultFlag, PlusExpr(VarExpr resultFlag, Num 1))]
        @ [Assign(resultFlag, DivExpr(VarExpr resultFlag, Num 2))]
    commands, resultFlag, counter3

and shortCircuitOrToFlag
    (left: boolexpr)
    (right: boolexpr)
    (counter: int)
    : command list * variable * int =
    let resultFlag, counter1 = freshTemp counter
    let trueBranch = Assign(resultFlag, Num 1)
    let falseBranch, counter2 = assignFlagFromGuard right resultFlag counter1
    if isDirectGuard left then
        [If [ (left, trueBranch); (Not left, falseBranch) ]], resultFlag, counter2
    else
        let leftCommands, leftFlag, counter3 = boolToFlag left counter1
        leftCommands @ [branchOnFlag leftFlag trueBranch falseBranch], resultFlag, max counter2 counter3

let lowerComplexGuard
    (guard: boolexpr)
    (onTrue: command)
    (onFalse: command)
    (counter: int)
    : command * int =
    let guardCommands, guardFlag, nextCounter = boolToFlag guard counter
    appendCommand
        (commandListToSemi guardCommands)
        (branchOnFlag guardFlag onTrue onFalse),
    nextCounter

let rec lowerGuardedCommands
    (guards: (boolexpr * command) list)
    (fallback: command)
    (counter: int)
    : command * int =
    match guards with
    | [] -> fallback, counter
    | (guard, body) :: rest ->
        let restCommand, counter1 = lowerGuardedCommands rest fallback counter
        if isDirectGuard guard then
            If [ (guard, body); (Not guard, restCommand) ], counter1
        else
            lowerComplexGuard guard body restCommand counter1

let buildLoopContinueFlag
    (flagVar: variable)
    (guards: (boolexpr * command) list)
    (counter: int)
    : command * int =
    let continueGuards =
        guards
        |> List.map (fun (guard, _) -> guard, Assign(flagVar, Num 1))
    lowerGuardedCommands continueGuards (Assign(flagVar, Num 0)) counter

let rec binarizeCommand (stuckName: string) (cmd: command) (counter: int) : command * int =
    let binarizeGuardBodies
        (guards: (boolexpr * command) list)
        (currentCounter: int)
        : (boolexpr * command) list * int =
        let reversedGuards, nextCounter =
            guards
            |> List.fold (fun (acc, nextCounter) (guard, body) ->
                let binaryBody, updatedCounter = binarizeCommand stuckName body nextCounter
                (guard, binaryBody) :: acc, updatedCounter
            ) ([], currentCounter)
        List.rev reversedGuards, nextCounter

    match cmd with
    | Skip -> Skip, counter
    | Assign (target, expr) ->
        let commands, nextCounter = binarizeExprInto target expr counter
        commandListToSemi commands, nextCounter
    | Semi (first, second) ->
        let firstCommand, counter1 = binarizeCommand stuckName first counter
        let secondCommand, counter2 = binarizeCommand stuckName second counter1
        Semi(firstCommand, secondCommand), counter2
    | If guards ->
        let orderedGuards, counter1 = binarizeGuardBodies guards counter
        match orderedGuards with
        | [ (guard, body) ] when isDirectGuard guard ->
            If [ (guard, body); (Not guard, stuckCommand stuckName) ], counter1
        | [ (guard, body) ] ->
            lowerComplexGuard guard body (stuckCommand stuckName) counter1
        | _ ->
            lowerGuardedCommands orderedGuards (stuckCommand stuckName) counter1
    | Loop guards ->
        let orderedGuards, counter1 = binarizeGuardBodies guards counter
        match orderedGuards with
        | [ (guard, body) ] when isDirectGuard guard ->
            Loop [ (guard, body) ], counter1
        | [ (guard, body) ] ->
            let guardCommands, guardFlag, counter2 = boolToFlag guard counter1
            let selectionCommand = commandListToSemi guardCommands
            let continueGuard = Equal(VarExpr guardFlag, Num 1)
            let loopBody = appendCommand body selectionCommand
            Semi(selectionCommand, Loop [ (continueGuard, loopBody) ]), counter2
        | _ ->
            let flagVar, counter2 = freshTemp counter1
            let selectionCommand, counter3 = buildLoopContinueFlag flagVar orderedGuards counter2
            let dispatchCommand, counter4 = lowerGuardedCommands orderedGuards (stuckCommand stuckName) counter3
            let continueGuard = Equal(VarExpr flagVar, Num 1)
            let loopBody = appendCommand dispatchCommand selectionCommand
            Semi(selectionCommand, Loop [ (continueGuard, loopBody) ]), counter4

let analysis (input: Input) : Output =
    match parse Grammar.start_command input.commands with
    | Ok ast ->
        let binaryAst, _ = binarizeCommand (freshStuckName ast) ast (initialTempCounter ast)
        { binary = Parser.prettyPrint binaryAst }
    | Error e ->
        { binary = String.Format("Parse error: {0}", e) }

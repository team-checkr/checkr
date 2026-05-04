module RiscV
open Io.RiscV

open System
open AST
open FSharp.Text.Lexing

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
        Error(ParseError(pos, lastToken, e))

let variableLabel (name: string) : string =
    "v" + name

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
    | List _ -> failwith "Arrays are not supported in the RiscV task"

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

and boolVariables (guard: boolexpr) : Set<string> =
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

let addOrderedNames (ordered: string list) (seen: Set<string>) (names: Set<string>) : string list * Set<string> =
    names
    |> Set.toList
    |> List.sort
    |> List.fold (fun (orderedNames, seenNames) name ->
        if Set.contains name seenNames then
            orderedNames, seenNames
        else
            orderedNames @ [name], Set.add name seenNames
    ) (ordered, seen)

let rec commandVariableOrder (cmd: command) : string list =
    let rec collectCommand (ordered: string list) (seen: Set<string>) (command: command) : string list * Set<string> =
        match command with
        | Skip ->
            ordered, seen
        | Assign (target, expression) ->
            let ordered, seen = addOrderedNames ordered seen (variableNames target)
            addOrderedNames ordered seen (exprVariables expression)
        | Semi (first, second) ->
            let ordered, seen = collectCommand ordered seen first
            collectCommand ordered seen second
        | If guards
        | Loop guards ->
            guards
            |> List.fold (fun (ordered, seen) (guard, body) ->
                let ordered, seen = addOrderedNames ordered seen (boolVariables guard)
                collectCommand ordered seen body
            ) (ordered, seen)

    fst (collectCommand [] Set.empty cmd)

let rec isDirectRiscVExpr (expression: expr) : bool =
    match expression with
    | Num _
    | VarExpr (Var _) ->
        true
    | VarExpr (List _) ->
        false
    | UMinusExpr inner ->
        isDirectRiscVAtomic inner
    | PlusExpr (left, right)
    | MinusExpr (left, right)
    | TimesExpr (left, right)
    | DivExpr (left, right) ->
        isDirectRiscVAtomic left && isDirectRiscVAtomic right
    | PowExpr _ ->
        false

and isDirectRiscVAtomic (expression: expr) : bool =
    match expression with
    | Num _
    | VarExpr (Var _) -> true
    | _ -> false

let rec isSimpleDirectRiscVBool (guard: boolexpr) : bool =
    match guard with
    | Bool _ -> true
    | Equal (left, right)
    | NotEq (left, right)
    | SmallerThan (left, right)
    | GreaterThan (left, right)
    | SmallerEq (left, right)
    | GreaterEq (left, right) ->
        isDirectRiscVAtomic left && isDirectRiscVAtomic right
    | Not inner ->
        match inner with
        | Bool _ -> true
        | Equal _
        | NotEq _
        | SmallerThan _
        | GreaterThan _
        | SmallerEq _
        | GreaterEq _ ->
            isSimpleDirectRiscVBool inner
        | _ ->
            false
    | And _
    | Or _
    | BitAnd _
    | BitOr _ ->
        false

let rec isSimpleDirectRiscVCommand (cmd: command) : bool =
    match cmd with
    | Skip -> true
    | Assign (Var _, expression) ->
        isDirectRiscVExpr expression
    | Assign (List _, _) ->
        false
    | Semi (first, second) ->
        isSimpleDirectRiscVCommand first && isSimpleDirectRiscVCommand second
    | If guards ->
        guards.Length <= 2
        && guards |> List.forall (fun (guard, body) -> isSimpleDirectRiscVBool guard && isSimpleDirectRiscVCommand body)
    | Loop guards ->
        guards.Length = 1
        && guards |> List.forall (fun (guard, body) -> isSimpleDirectRiscVBool guard && isSimpleDirectRiscVCommand body)

let loadImmediate (register: string) (value: int) : string list =
    [sprintf "\tli %s, %d" register value]

let loadVariable (register: string) (name: string) : string list =
    [sprintf "\tlw %s, %s" register (variableLabel name)]

let rec emitAtomicExpr (expression: expr) (targetReg: string) : string list =
    match expression with
    | Num value ->
        loadImmediate targetReg value
    | VarExpr (Var name) ->
        loadVariable targetReg name
    | VarExpr (List _) ->
        failwith "Arrays are not supported in the RiscV task"
    | _ ->
        failwithf "Expected atomic expression, got %s" (Parser.prettyPrintExp expression)

and emitExprInto (expression: expr) (targetReg: string) (scratch1: string) (labelSeed: string) : string list =
    match expression with
    | Num _
    | VarExpr _ ->
        emitAtomicExpr expression targetReg
    | UMinusExpr inner ->
        emitExprInto inner targetReg scratch1 labelSeed
        @ [sprintf "\tneg %s, %s" targetReg targetReg]
    | PlusExpr (left, right) ->
        emitExprInto left targetReg scratch1 labelSeed
        @ emitExprInto right scratch1 targetReg labelSeed
        @ [sprintf "\tadd %s, %s, %s" targetReg targetReg scratch1]
    | MinusExpr (left, right) ->
        emitExprInto left targetReg scratch1 labelSeed
        @ emitExprInto right scratch1 targetReg labelSeed
        @ [sprintf "\tsub %s, %s, %s" targetReg targetReg scratch1]
    | TimesExpr (left, right) ->
        emitExprInto left targetReg scratch1 labelSeed
        @ emitExprInto right scratch1 targetReg labelSeed
        @ [sprintf "\tmul %s, %s, %s" targetReg targetReg scratch1]
    | DivExpr (left, right) ->
        emitExprInto left targetReg scratch1 labelSeed
        @ emitExprInto right scratch1 targetReg labelSeed
        @ [ sprintf "\tdiv %s, %s, %s" targetReg targetReg scratch1 ]
    | PowExpr (left, right) ->
        emitExprInto left targetReg scratch1 labelSeed
        @ emitExprInto right scratch1 targetReg labelSeed
        @ [ "\tli a7, 10"
            "\tecall" ]

and emitExpr (expression: expr) (targetReg: string) (labelSeed: string) : string list =
    emitExprInto expression targetReg "t2" labelSeed

let emitAssignment (target: variable) (expression: expr) (labelSeed: string) : string list =
    match target with
    | Var name ->
        [sprintf "\tla a6, %s" (variableLabel name)]
        @ emitExpr expression "t1" labelSeed
        @ [ "\tsw t1, 0(a6)" ]
    | List _ ->
        failwith "Arrays are not supported in the RiscV task"

let rec branchInstructionWithFalse (guard: boolexpr) (trueTarget: string) (falseTarget: string option) (labelSeed: string) : string list =
    let falseLabel =
        match falseTarget with
        | Some label -> label
        | None -> sprintf "%s_false" labelSeed

    let conditionalBranch opcode left right =
        match falseTarget with
        | Some label ->
            emitAtomicExpr left "t0"
            @ emitAtomicExpr right "t1"
            @ [ sprintf "\t%s t0, t1, %s" opcode trueTarget
                sprintf "\tj %s" label ]
        | None ->
            emitAtomicExpr left "t0"
            @ emitAtomicExpr right "t1"
            @ [ sprintf "\t%s t0, t1, %s" opcode trueTarget ]

    let addFalseLabelIfNeeded (instructions: string list) =
        match falseTarget with
        | Some _ -> instructions
        | None -> instructions @ [sprintf "%s:" falseLabel]

    match guard with
    | Bool true ->
        [sprintf "\tj %s" trueTarget]
    | Bool false ->
        match falseTarget with
        | Some label -> [sprintf "\tj %s" label]
        | None -> []
    | Equal (left, right) ->
        conditionalBranch "beq" left right
        |> addFalseLabelIfNeeded
    | NotEq (left, right) ->
        conditionalBranch "bne" left right
        |> addFalseLabelIfNeeded
    | SmallerThan (left, right) ->
        conditionalBranch "blt" left right
        |> addFalseLabelIfNeeded
    | GreaterThan (left, right) ->
        match falseTarget with
        | Some label ->
            emitAtomicExpr left "t0"
            @ emitAtomicExpr right "t1"
            @ [ sprintf "\tblt t1, t0, %s" trueTarget
                sprintf "\tj %s" label ]
        | None ->
            emitAtomicExpr left "t0"
            @ emitAtomicExpr right "t1"
            @ [ sprintf "\tblt t1, t0, %s" trueTarget ]
        |> addFalseLabelIfNeeded
    | SmallerEq (left, right) ->
        emitAtomicExpr left "t0"
        @ emitAtomicExpr right "t1"
        @ [ sprintf "\tblt t1, t0, %s" falseLabel
            sprintf "\tj %s" trueTarget ]
        |> addFalseLabelIfNeeded
    | GreaterEq (left, right) ->
        emitAtomicExpr left "t0"
        @ emitAtomicExpr right "t1"
        @ [ sprintf "\tblt t0, t1, %s" falseLabel
            sprintf "\tj %s" trueTarget ]
        |> addFalseLabelIfNeeded
    | Not inner ->
        match inner with
        | Bool true ->
            match falseTarget with
            | Some label -> [sprintf "\tj %s" label]
            | None -> []
        | Bool false -> [sprintf "\tj %s" trueTarget]
        | Equal (left, right) ->
            branchInstructionWithFalse (NotEq (left, right)) trueTarget falseTarget labelSeed
        | NotEq (left, right) ->
            branchInstructionWithFalse (Equal (left, right)) trueTarget falseTarget labelSeed
        | SmallerThan (left, right) ->
            branchInstructionWithFalse (GreaterEq (left, right)) trueTarget falseTarget labelSeed
        | GreaterThan (left, right) ->
            branchInstructionWithFalse (SmallerEq (left, right)) trueTarget falseTarget labelSeed
        | SmallerEq (left, right) ->
            branchInstructionWithFalse (GreaterThan (left, right)) trueTarget falseTarget labelSeed
        | GreaterEq (left, right) ->
            branchInstructionWithFalse (SmallerThan (left, right)) trueTarget falseTarget labelSeed
        | _ ->
            failwithf "Unsupported boolean expression: %s" (Parser.prettyPrintBool guard)
    | _ ->
        failwithf "Unsupported boolean expression: %s" (Parser.prettyPrintBool guard)

let branchInstruction (guard: boolexpr) (target: string) (labelSeed: string) : string list =
    branchInstructionWithFalse guard target None labelSeed

let nodeSortKey (label: string) : int * int =
    if label = "qStart" then
        0, 0
    elif label = "qFinal" then
        2, 0
    elif label.StartsWith("q") then
        match Int32.TryParse(label.Substring(1)) with
        | true, value -> 1, value
        | false, _ -> 1, Int32.MaxValue
    else
        1, Int32.MaxValue

let remapLabels (groupedEdges: Map<string, Compiler.Edge list>) : Map<string, string> =
    let rec visit (label: string) (visited: Set<string>) (ordered: string list) : Set<string> * string list =
        if label = "qFinal" || Set.contains label visited || not (Map.containsKey label groupedEdges) then
            visited, ordered
        else
            let visited = Set.add label visited
            let ordered = ordered @ [label]
            match groupedEdges.[label] with
            | [edge] ->
                visit edge.target visited ordered
            | firstEdge :: secondEdge :: [] ->
                let visited, ordered = visit secondEdge.target visited ordered
                visit firstEdge.target visited ordered
            | _ ->
                visited, ordered

    let _, ordered = visit "qStart" Set.empty []
    let numbered =
        ordered
        |> List.filter (fun label -> label <> "qStart")
        |> List.mapi (fun index label -> label, sprintf "q%d" (index + 1))
        |> Map.ofList

    numbered
    |> Map.add "qStart" "qStart"
    |> Map.add "qFinal" "qFinal"

let remapLabelsByNumericOrder (groupedEdges: Map<string, Compiler.Edge list>) : Map<string, string> =
    let numbered =
        groupedEdges
        |> Map.toList
        |> List.map fst
        |> List.filter (fun label -> label <> "qStart" && label <> "qFinal")
        |> List.sortBy nodeSortKey
        |> List.mapi (fun index label -> label, sprintf "q%d" (index + 1))
        |> Map.ofList

    numbered
    |> Map.add "qStart" "qStart"
    |> Map.add "qFinal" "qFinal"

let tryCollapseTempCopy
    (source: string)
    (outgoing: Compiler.Edge list)
    (groupedEdges: Map<string, Compiler.Edge list>)
    (incomingCounts: Map<string, int>)
    : (variable * expr * string * string) option =
    match outgoing with
    | [ edge ] ->
        match edge.label with
        | Compiler.CommandLabel (Assign(Var tempName, expression))
            when tempName.StartsWith("tmp") && incomingCounts.TryFind(edge.target) = Some 1 ->
            match groupedEdges.TryFind edge.target with
            | Some [ nextEdge ] ->
                match nextEdge.label with
                | Compiler.CommandLabel (Assign(target, VarExpr(Var copiedName))) when copiedName = tempName ->
                    Some(target, expression, nextEdge.target, tempName)
                | _ ->
                    None
            | _ ->
                None
        | _ ->
            None
    | _ ->
        None

let emitNode
    (source: string)
    (outgoing: Compiler.Edge list)
    (groupedEdges: Map<string, Compiler.Edge list>)
    (incomingCounts: Map<string, int>)
    (labelMap: Map<string, string>)
    : string list =
    let rename label = Map.find label labelMap
    let header = [sprintf "%s:" (rename source)]
    match tryCollapseTempCopy source outgoing groupedEdges incomingCounts with
    | Some(target, expression, nextTarget, _) ->
        header
        @ emitAssignment target expression source
        @ [sprintf "\tj %s" (rename nextTarget)]
    | None ->
        match outgoing with
        | [] ->
            header @ [ "\tj qFinal" ]
        | [ edge ] ->
            match edge.label with
            | Compiler.CommandLabel Skip ->
                header @ [sprintf "\tj %s" (rename edge.target)]
            | Compiler.CommandLabel (Assign(target, expression)) ->
                header
                @ emitAssignment target expression source
                @ [sprintf "\tj %s" (rename edge.target)]
            | Compiler.BoolLabel guard ->
                header @ branchInstruction guard (rename edge.target) source
            | Compiler.CommandLabel _ ->
                failwith "Unsupported command in program graph"
        | firstEdge :: secondEdge :: [] ->
            match firstEdge.label, secondEdge.label with
            | Compiler.BoolLabel (Bool true), Compiler.BoolLabel _ ->
                header @ [sprintf "\tj %s" (rename firstEdge.target)]
            | Compiler.BoolLabel (Bool false), Compiler.BoolLabel _ ->
                header @ [sprintf "\tj %s" (rename secondEdge.target)]
            | Compiler.BoolLabel firstGuard, Compiler.BoolLabel _ ->
                header
                @ branchInstructionWithFalse firstGuard (rename firstEdge.target) (Some (rename secondEdge.target)) source
            | _ ->
                failwith "Expected binary branching node in program graph"
        | _ ->
            failwith "Program graph is not binary"

type LabelStrategy =
    | DepthFirstFalseFirst
    | NumericOrder

type EmitOptions = {
    labelStrategy: LabelStrategy
    collapseTempCopies: bool
    preserveVariableOrder: bool
}

let emitAssemblyWithOptions (options: EmitOptions) (binaryAst: command) : string =
    let edges, _ = Compiler.edges binaryAst "qStart" "qFinal" 0 Io.GCL.NonDeterministic
    let groupedEdges =
        edges
        |> List.groupBy (fun edge -> edge.source)
        |> Map.ofList

    let labelMap =
        match options.labelStrategy with
        | DepthFirstFalseFirst -> remapLabels groupedEdges
        | NumericOrder -> remapLabelsByNumericOrder groupedEdges

    let incomingCounts =
        edges
        |> List.countBy (fun edge -> edge.target)
        |> Map.ofList

    let collapsedCopies =
        if options.collapseTempCopies then
            groupedEdges
            |> Map.toList
            |> List.choose (fun (source, outgoing) ->
                tryCollapseTempCopy source outgoing groupedEdges incomingCounts
                |> Option.map (fun (_, _, _, tempName) ->
                    let skippedLabel =
                        match outgoing with
                        | [ edge ] -> edge.target
                        | _ -> failwith "Unexpected non-singleton collapsed copy"
                    skippedLabel, tempName
                )
            )
        else
            []

    let skippedLabels =
        collapsedCopies
        |> List.map fst
        |> Set.ofList

    let skippedTemps =
        collapsedCopies
        |> List.map snd
        |> Set.ofList

    let labels =
        groupedEdges
        |> Map.toList
        |> List.map fst
        |> List.filter (fun label -> not (Set.contains label skippedLabels))
        |> List.sortBy nodeSortKey

    let variables =
        let orderedVariables =
            if options.preserveVariableOrder then
                commandVariableOrder binaryAst
            else
                commandVariables binaryAst |> Set.toList |> List.sort

        orderedVariables
        |> List.filter (fun name -> not (Set.contains name skippedTemps))

    let dataSection =
        [".data"]
        @ (variables |> List.map (fun name -> sprintf "%s:\t\t.word 0" (variableLabel name)))

    let textSection =
        [".text"]
        @ (labels |> List.collect (fun label -> emitNode label groupedEdges.[label] groupedEdges incomingCounts labelMap))
        @ [ "qFinal:"
            "\tli a7, 10"
            "\tecall" ]

    String.concat "\n" (dataSection @ ["" ] @ textSection)

let emitAssembly (binaryAst: command) : string =
    emitAssemblyWithOptions
        { labelStrategy = DepthFirstFalseFirst
          collapseTempCopies = true
          preserveVariableOrder = false }
        binaryAst

let analysis (input: Input) : Output =
    match parse Grammar.start_command input.commands with
    | Ok originalAst when isSimpleDirectRiscVCommand originalAst ->
        { assembly = emitAssembly originalAst }
    | Ok _ ->
        let binaryOutput: Io.BiGCL.Output = BiGCL.analysis { commands = input.commands }
        let binaryProgram = binaryOutput.binary

        match parse Grammar.start_command binaryProgram with
        | Ok binaryAst ->
            { assembly =
                emitAssemblyWithOptions
                    { labelStrategy = NumericOrder
                      collapseTempCopies = false
                      preserveVariableOrder = true }
                    binaryAst }
        | Error e ->
            { assembly = String.Format("Parse error: {0}", e) }
    | Error e ->
        { assembly = String.Format("Parse error: {0}", e) }

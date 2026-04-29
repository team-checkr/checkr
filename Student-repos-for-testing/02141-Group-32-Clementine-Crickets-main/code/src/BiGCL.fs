module BiGCL

open Io.BiGCL
open State
open AST

type Node = string
//TODO: Can we solve it like this, without variable list in state? the list doesnt change so it doesnt really make sense
let GenerateNode: State<int * list<string>, Node> =
    State (fun (i, variables) ->
        // check that result is not
        let result i = "tmp" + string i + "_"

        let rec ensure_uniquness (i: int) : (string * int) =
            let prop_name = result i

            if List.exists (fun name -> name = prop_name) variables then
                ensure_uniquness (i + 1)
            else
                prop_name, i

        let name, i = ensure_uniquness i
        name, ((i + 1), variables))

let failState msg : State<'s, 'r> = State(fun _ -> failwith msg)

let state = StatefulBuilder()

/// Returns constructor and operands for binary arithmatic operations
let (|TwoOpArithmetic|_|) (e: expr) =
    match e with
    | TimesExpr (a, b) -> Some(TimesExpr, a, b)
    | DivExpr (a, b) -> Some(DivExpr, a, b)
    | PlusExpr (a, b) -> Some(PlusExpr, a, b)
    | MinusExpr (a, b) -> Some(MinusExpr, a, b)
    | PowExpr (a, b) -> Some(PowExpr, a, b)
    | _ -> None

/// returns true for binary operations. Nums and variables return false, as these are handeled before.
let rec isBinaryOp (expr: expr) : bool =
    match expr with
    | TwoOpArithmetic (_, Num _, Num _)
    | TwoOpArithmetic (_, Num _, Variable _)
    | TwoOpArithmetic (_, Variable _, Num _)
    | TwoOpArithmetic (_, Variable _, Variable _) -> true
    | ParenExpr (expr) -> isBinaryOp expr
    | Array (_) -> false
    | _ -> false


/// Active pattern match to match with binary boolean operators. Returns the constructor aswell as the operands.
let (|TwoOpBool|_|) (bexpr: booleanExpr) =
    match bexpr with
    | And (a, b) -> Some(And, a, b)
    | Or (a, b) -> Some(Or, a, b)
    | Scand (a, b) -> Some(Scand, a, b)
    | Scor (a, b) -> Some(Scor, a, b)
    | _ -> None

/// Returns constructor and operands for boolean expressions regarding equality.
let (|EqualityBool|_|) (bexpr: booleanExpr) =
    match bexpr with
    | Eq (a, b) -> Some(Eq, a, b)
    | Neq (a, b) -> Some(Neq, a, b)
    | Lt (a, b) -> Some(Lt, a, b)
    | Gt (a, b) -> Some(Gt, a, b)
    | Leq (a, b) -> Some(Leq, a, b)
    | Geq (a, b) -> Some(Geq, a, b)
    | _ -> None

let rec isSimpleBool (bexpr: booleanExpr) =
    match bexpr with
    | True -> true
    | False -> true
    | ParenBool (inside) -> isSimpleBool inside
    | EqualityBool (_, a, b) ->
        match (a, b) with
        | Variable _, Variable _
        | Variable _, Num _
        | Num _, Variable _
        | Num _, Num _ -> true
        | _ -> false
    | _ -> false


let combineCmdOption (leftCmd: option<command>) (rightCmd: option<command>) : option<command> =
    match (leftCmd, rightCmd) with
    | None, None -> None
    | (Some (c), None)
    | None, Some (c) -> Some(c)
    | Some (left), Some (right) -> Some(SequenceCommand(left, right))

let threadCmdOption (optional_command: option<command>) (cmd: command) : command =
    match optional_command with
    | None -> cmd
    | Some (c) -> SequenceCommand(c, cmd)



/// Takes an expression e, and returns the maybe commands to process it, aswell as the last expression containing the equivalent binary expression to e.
let rec handleExpressions e : State<int * list<string>, option<command> * expr> =
    state {
        match e with
        | Num _
        | Variable _ -> return None, e

        | e when isBinaryOp e ->
            // This branch should only be hit through processing of booleans etc... For variable assignments we should check before delegating to this function
            let! node = GenerateNode
            return Some(VariableAssignment(Ident node, e)), Variable node

        | TwoOpArithmetic (ctor, left, right) ->
            let! (leftCmd, leftExpr) = processOperand left
            let! (rightCmd, rightExpr) = processOperand right
            let! node = GenerateNode

            let previous_commands = combineCmdOption leftCmd rightCmd
            let assignment = VariableAssignment(node, (ctor (leftExpr, rightExpr)))

            return Some(threadCmdOption previous_commands assignment), Variable node

        | UMinusExpr (inner) ->
            let! (cmd, innerExpr) = processOperand inner
            let! node = GenerateNode
            let assignment = VariableAssignment(node, UMinusExpr innerExpr)
            return Some(threadCmdOption cmd assignment), Variable node

        | ParenExpr (inner) -> return! handleExpressions inner

        | Array (_) -> return None, e
        | _ -> return! failState "should be unreachable? i think we get warning for no reason"

    }

/// helper function, takes an operand (expr) and process it. If it is simple, just return it. If not return processing commands and the final expression it is contained in.
and processOperand (operand: expr) : State<int * list<string>, option<command> * expr> =
    // returns commands needed to use operand and returns the expression.
    state {
        match operand with
        | ParenExpr (inner) -> return! processOperand inner
        | Num _
        | Variable _ -> return None, operand
        | _ ->
            let! (cmd, ident) = handleExpressions operand
            return cmd, ident
    }

let handleVariableAssignment (ident: Ident) (expr: expr) : State<int * list<string>, command> =
    state {
        if isBinaryOp expr then
            return VariableAssignment(ident, expr)
        else
            let! (cmd, lastExpr) = handleExpressions expr
            let assignment = VariableAssignment(ident, lastExpr)
            return threadCmdOption cmd assignment
    }


let handleEqualBool ctor l r =
    state {
        // for any a = b, we process a and b, recreate the condition by storing 1 in a result value if true and 0 otherwise.
        // Returns the preprocessing commands, alongside the last expression, which is the result_node = 1, indicating true on the original equality

        // First process left and right expressions
        let! result_node = GenerateNode
        let! leftCmd, leftIdent = handleExpressions l
        let! rightCmd, rightIdent = handleExpressions r

        // Recreate original condition with the simplified expressions
        let condition = (ctor (leftIdent, rightIdent))

        let assignment num =
            VariableAssignment((Ident result_node, Num num))

        let success_branch = Conditional(condition, assignment 1)
        let failure_branch = Conditional(Neg condition, assignment 0)
        let if_command = IfCommand(SequenceGuard(success_branch, failure_branch))
        let booleanCmds = combineCmdOption leftCmd rightCmd
        let result_commands = threadCmdOption booleanCmds if_command
        return Some(result_commands), (Eq(Variable result_node, Num 1))
    }

/// Helper function. Takes a optional preprocess commands, and a binary expression, and enforces that the result of the binary expression is stored in a variable.
/// Extra steps are threaded in among the optional preprocess commands.
let ensureVariableBoolean
    (cmd: option<command>)
    (b: booleanExpr)
    : State<int * list<string>, (option<command> * Ident)> =
    state {
        match b with
        | True ->
            let! result_node = GenerateNode
            let assignment = VariableAssignment(Ident result_node, Num 1)
            let result_command = threadCmdOption cmd assignment
            return (Some result_command, result_node)

        | False ->
            let! result_node = GenerateNode
            let assignment = VariableAssignment(Ident result_node, Num 0)
            let result_command = threadCmdOption cmd assignment
            return (Some result_command, result_node)

        | _ ->
            // Need to materialize: create if bExpr then tmp := 1 else tmp := 0
            let! result_node = GenerateNode

            let assignment num =
                VariableAssignment(Ident result_node, Num num)

            let if_command =
                IfCommand(SequenceGuard(Conditional(b, assignment 1), Conditional(Neg b, assignment 0)))

            let fullCmd = threadCmdOption cmd if_command
            return (Some fullCmd, result_node)
    }




/// Returns a optional desicribing the commands needed to simplify the boolean expression, as well as the final expr containing the result of the boolean expression
/// Fx for a = b returns None, Eq(a, b)
let rec handleBoolean (expr: booleanExpr) : State<int * list<string>, option<command> * booleanExpr> =
    state {
        match expr with
        | b when isSimpleBool b -> return None, expr
        | ParenBool (b) -> return! handleBoolean b
        | Neg (b) ->
            let! (cmd, bExpr) = handleBoolean b
            // Force to a variable and negate it
            let! (setupCmd, ident) = ensureVariableBoolean cmd bExpr
            let! negNode = GenerateNode
            let negation = VariableAssignment(Ident negNode, MinusExpr(Num 1, Variable ident))
            let fullCmd = threadCmdOption setupCmd negation
            return (Some fullCmd, Eq(Variable negNode, Num 1))

        | TwoOpBool (_) -> return! handleBinaryBool expr

        | EqualityBool (ctor, l, r) -> return! handleEqualBool ctor l r

    }

and handleBinaryBool (bExpr: booleanExpr) =
    state {
        match bExpr with
        | And (l, r)
        | Or (l, r) ->
            // non-short curcuit, thus we eval left and right boolean first
            let! (leftCmds, leftExpr) = handleBoolean l
            let! (rightCmds, rightExpr) = handleBoolean r

            let! (lCmds, lIdent) = ensureVariableBoolean leftCmds leftExpr
            let! (rCmds, rIdent) = ensureVariableBoolean rightCmds rightExpr


            let preCommands = combineCmdOption lCmds rCmds
            let! resultNode = GenerateNode

            let resultCommands =
                match bExpr with
                | And _ -> VariableAssignment(Ident resultNode, (TimesExpr(Variable lIdent, Variable rIdent)))

                | Or _ ->
                    let tmp_init =
                        VariableAssignment(Ident resultNode, (PlusExpr(Variable lIdent, Variable rIdent)))

                    let plus_one =
                        VariableAssignment(Ident resultNode, (PlusExpr(Variable resultNode, Num 1)))

                    let result_tmp =
                        VariableAssignment(Ident resultNode, DivExpr(Variable resultNode, Num 2))

                    SequenceCommand(tmp_init, SequenceCommand(plus_one, result_tmp))


            let result = threadCmdOption preCommands resultCommands

            return Some(result), Eq(Variable resultNode, Num 1)


        | Scor (l, r) ->
            // If l is true -> result = 1 else evaluate innerblock (right)
            let! resultNode = GenerateNode

            let! (lCmd, lExpr) = handleBoolean l
            let! (rCmd, rExpr) = handleBoolean r

            // if r -> result := 1 [] !r -> result := 0 fi
            let innerIf =
                IfCommand(
                    SequenceGuard(
                        Conditional(rExpr, VariableAssignment(Ident resultNode, Num 1)),
                        Conditional(Neg rExpr, VariableAssignment(Ident resultNode, Num 0))
                    )
                )

            // Combine with r's setup commands
            let innerBlock = threadCmdOption rCmd innerIf

            // if l -> result := 1 [] !l -> innerBlock fi
            let outerIf =
                IfCommand(
                    SequenceGuard(
                        Conditional(lExpr, VariableAssignment(Ident resultNode, Num 1)),
                        Conditional(Neg lExpr, innerBlock)
                    )
                )

            // Combine with l's setup commands
            let fullCmd = threadCmdOption lCmd outerIf

            return Some(fullCmd), Eq(Variable resultNode, Num 1)

        | Scand (l, r) ->
            // If l is true -> evaluate innerblock (right) else result = 0
            let! resultNode = GenerateNode
            let! (lCmd, lExpr) = handleBoolean l
            let! (rCmd, rExpr) = handleBoolean r

            let innerIf =
                IfCommand(
                    SequenceGuard(
                        Conditional(rExpr, VariableAssignment(Ident resultNode, Num 1)),
                        Conditional(Neg rExpr, VariableAssignment(Ident resultNode, Num 0))
                    )
                )

            let innerBlock = threadCmdOption rCmd innerIf

            let outerIf =
                IfCommand(
                    SequenceGuard(
                        Conditional(lExpr, innerBlock),
                        Conditional(Neg lExpr, VariableAssignment(Ident resultNode, Num 0))
                    )
                )

            let fullCmd = threadCmdOption lCmd outerIf
            return Some(fullCmd), Eq(Variable resultNode, Num 1)



        | _ -> return! failState "unreachable"
    }

and handleCommand (command: command) : State<int * list<string>, command> =
    state {
        match command with
        | VariableAssignment (ident, expr) -> return! handleVariableAssignment ident expr
        | SequenceCommand (c1, c2) ->
            let! result_left = handleCommand c1
            let! result_right = handleCommand c2
            return SequenceCommand(result_left, result_right) //TODO: verify that ths is correct?

        | ArrayAssignment (_) -> return command
        | Skip -> return Skip
        | IfCommand (guard) -> return! handleIfCommand guard
        | DoCommand (guard) -> return! doCommandHelper guard
    }

and handleIfCommand (guard: guard) : State<int * list<string>, command> =
    state {
        match guard with
        | Conditional (b, c) -> return! finalGuardHelper b c

        | SequenceGuard (Conditional gc1, gc2) ->
            // Pattern: GC1 [] GC2 becomes:
            // if b1 -> C1 [] !b1 -> (recursively handle GC2) fi

            let (b1, C1) = gc1
            let! booleanCmds1, bTmp1 = handleBoolean b1
            let! cmd1 = handleCommand C1
            let! cmdgc2 = handleIfCommand gc2


            // 6. Build: preprocessing ; if b1 -> C1 [] !b1 -> (result of gc2) fi
            let sequence =
                IfCommand(SequenceGuard(Conditional(bTmp1, cmd1), Conditional(Neg bTmp1, cmdgc2)))

            return threadCmdOption booleanCmds1 sequence

        | SequenceGuard (SequenceGuard (first, second), third) ->
            // Flatten nested SequenceGuard: (a [] b) [] c  ==>  a [] (b [] c)
            // This handles the case where the parser creates a left-nested structure
            return! handleIfCommand (SequenceGuard(first, SequenceGuard(second, third)))

    }


and doCommandHelper (guard: guard) : State<int * list<string>, command> =
    // do GC od ->
    // if gc -> tmp = 1, if not maybe next conditional and so on...
    // add one do tmp = 1 -> c ; recalculate tmp od
    state {

        let! loopVar = GenerateNode

        // check "any guard is true" and store in loopVar
        let! guardChecker = buildGuardOrCheck guard loopVar

        // Process the original guard body
        let! guardBody =
            match guard with
            | SequenceGuard (_) -> handleIfCommand guard
            | Conditional (_, c) -> handleCommand c

        // Loop body: execute guard + re-check
        let loopBody = SequenceCommand(guardBody, guardChecker)

        // The loop condition
        let loopGuard = Conditional(Eq(Variable loopVar, Num 1), loopBody)

        // Initial check + loop
        return SequenceCommand(guardChecker, DoCommand(loopGuard))
    }

and buildGuardOrCheck (guard: guard) (resultVar: Ident) : State<int * list<string>, command> =
    state {
        match guard with
        | Conditional (b, _) ->
            // Single guard: if b -> resultVar := 1 [] !b -> resultVar := 0 fi
            let! (cmd, bExpr) = handleBoolean b

            let ifCmd =
                IfCommand(
                    SequenceGuard(
                        Conditional(bExpr, VariableAssignment(Ident resultVar, Num 1)),
                        Conditional(Neg bExpr, VariableAssignment(Ident resultVar, Num 0))
                    )
                )

            return threadCmdOption cmd ifCmd

        | SequenceGuard (Conditional (b1, _), restGuard) ->
            // Multiple guards: if b1 -> resultVar := 1 [] !b1 -> [check rest] fi
            let! (b1Cmd, b1Expr) = handleBoolean b1

            // Recursively build the check for remaining guards
            let! restCheck = buildGuardOrCheck restGuard resultVar

            // Build the outer if: if b1 -> resultVar := 1 [] !b1 -> restCheck fi
            let outerIf =
                IfCommand(
                    SequenceGuard(
                        Conditional(b1Expr, VariableAssignment(Ident resultVar, Num 1)),
                        Conditional(Neg b1Expr, restCheck)
                    )
                )

            return threadCmdOption b1Cmd outerIf

        | SequenceGuard (SequenceGuard (first, second), third) ->
            // Flatten nested SequenceGuard: (a [] b) [] c  ==>  a [] (b [] c)
            return! buildGuardOrCheck (SequenceGuard(first, SequenceGuard(second, third))) resultVar
    }

/// Returns the preprocessing commands to handle the LAST guard. This inlcudes the stuck_ phase!
and finalGuardHelper (b: booleanExpr) (c: command) : State<int * list<string>, command> =
    state {
        let! booleanCmds, simpB = handleBoolean b
        let! cmd = handleCommand c

        let last_check =
            IfCommand(
                SequenceGuard(
                    Conditional(simpB, cmd),
                    (Conditional((Neg simpB), VariableAssignment((Ident "stuck_"), DivExpr(Num 1, Num 0))))
                )
            )

        match booleanCmds with
        | None -> return last_check
        | Some (booleanCmds) -> return SequenceCommand(booleanCmds, last_check)
    }

let rec add_variable_to_list (program: command) (variables: list<string>) : list<string> =
    let rec guard_helper guard list : list<string> =
        match guard with
        | SequenceGuard (l, r) -> guard_helper r (guard_helper l list)
        | Conditional (_, c) -> add_variable_to_list c list

    let rec expr_helper expr variables : list<string> =
        match expr with
        | Variable v -> v :: variables
        | UMinusExpr (expr) -> expr_helper expr variables
        | Array (ident, expr) -> expr_helper expr (ident :: variables)
        | TwoOpArithmetic (_, l, r) ->
            (expr_helper l variables)
            @ (expr_helper r variables)
        | ParenExpr (expr) -> expr_helper expr variables
        | _ -> variables

    match program with
    | SequenceCommand (l, r) -> add_variable_to_list r (add_variable_to_list l variables)
    | ArrayAssignment (ident, _, expr) -> expr_helper expr (ident :: variables)
    | VariableAssignment (ident, expr) -> expr_helper expr (ident :: variables)
    | IfCommand (guard)
    | DoCommand (guard) -> guard_helper guard variables
    | _ -> variables


let analysis (input: Input) : Output =
    // let ast = Parser.parse_string input.commands
    // let variables = add_variable_to_list ast []
    // let (transpiled, _) = handleCommand ast |> State.run (0, variables)
    // { binary = Parser.prettyPrint transpiled }

    let astResult =
        try
            Some(Parser.parse_string input.commands)
        with
        | ex -> None

    match astResult with
    | None -> { binary = "parse failed" }
    | Some ast ->
        let variables = add_variable_to_list ast []
        let (transpiled, _) = handleCommand ast |> State.run (0, variables)
        { binary = Parser.prettyPrint transpiled }

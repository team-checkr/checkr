module State

// heavily inspired by
// https://dev.to/shimmer/the-state-monad-in-f-3ik0

// State monads type
type State<'state, 'result> =
  State of ('state -> ('result * 'state))


module State =
  let run (state: 'state) (State f) = f state
  // Lets define the two functions every monad has: return and bind
  // returns brings value up into monadic context, in our case we simply bind it to a function returning it alongside unaltared state
  let ret value = 
    State (fun state -> (value, state))

  // bind "binds" two computation together, each returning same state type but possibly different results
  let bind (binder: 'T -> State<'state, 'U>) (state: State<'state, 'T>) : State<'state, 'U> =
    State (fun s -> 
      let result, s' = state |> run s 
      binder result |> run s'
    )

type StatefulBuilder() =
    let (>>=) stateful binder = State.bind binder stateful
    member __.Return(result) = State.ret result
    member __.ReturnFrom(stateful) = stateful
    member __.Bind(stateful, binder) = stateful >>= binder
    member __.Zero() = State.ret ()
    member __.Combine(statefulA, statefulB) =
        statefulA >>= (fun _ -> statefulB)
    member __.Delay(f) = f ()



# mcltl-rs

An experimental model checker for LTL written in Rust.
That uses the theory of automata apply to linear temporal logic as a unifying paradigm for program specification, verification,and synthesis. The model checker takes in parameter a [Kripke structure](https://en.wikipedia.org/wiki/Kripke_structure_(model_checking)) which represents a (reactive) program and a PLTL formula.

:warning: This model checker is in beta version and experimental. Please don't use it for production use case! :warning:

## How it work

Translation of the original problem to a problem in automata theory:

* Original problem: `S |= P`. Does property P hold for every run of program/system S?
* Transform the Kripke model `Ma` in a Büchi automaton: `Sa` with language L(SA).
* Transform the property PLTL `ϕp` in a Büchi automaton PA: `B¬ϕp` with language L(PA).
* Construct the equivalent problem: `A⊗ = L(Sa) ∩ L(Pa)`.
* Final Problem `L(A⊗) = ∅`
    * Check whether the language of this automaton is empty.
    * Look for a word `w` accepted by this automaton.
        * If no such w exists, then `S |= P`.
        * If such a `w = w(r)` exists, then `r` is a counterexample, i.e. a run of S such that `r ⊯ P`.

This algorithm has a time and space complexity equal to: `O(|M| x 2^|ϕ|)`.
Model checking and satisfiability problem against an LTL formula is **PSPACE-complete**.

## Inspirations

* Vardi, Moshe. (1996). An Automata-Theoretic Approach to Linear Temporal Logic. 10.1007/3-540-60915-6_6.

* Gerth, Rob & Dolech, Den & Peled, Doron & Vardi, Moshe & Wolper, Pierre. (1995). Simple On-the-Fly Automatic Verification of Linear Temporal Logic. Proceedings of the 6th Symposium on Logic in Computer Science. 10.1007/978-0-387-34892-6_1.

* Courcoubetis, Costas & Vardi, Moshe & Wolper, Pierre & Yannakakis, Mihalis. (2006). Memory-Efficient Algorithms for the Verification of Temporal Properties. 10.1007/BFb0023737.

* Wolper, Pierre. (2001). Constructing Automata from Temporal Logic Formulas: A Tutorial. LNCS. 2090. 10.1007/3-540-44667-2_7.

You can find this publications in the [doc](https://github.com/NotBad4U/mcltl-rs/tree/master/doc) folder.

## Overview

To build the code just clone the repo and execute

```bash
cargo build --bin mcltl
```

To run the code just run the command mcltl like this:

```bash
./mcltl -k ./tests/test-data/program3.kripke -p 'a U (b or c)'`
Loading kripke file                                                        [OK]
Parsing kripke program                                                     [OK]
Parsing LTL property                                                       [OK]
Converting LTL property in NNF                                             [OK]
Constructing the graph of the LTL property                                 [OK]
Extracting a generalized Buchi automaton                                   [OK]
converting the generalized Buchi automaton into classic Buchi automaton    [OK]
Constructing the product of program and property automata                  [OK]

Result: LTL property does not hold
Cycle containing an accepting state:

INIT → n1: a → n2: a
```
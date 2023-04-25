# Changelog

All notable changes to this project will be documented in this file.

## [0.1.21] - 2023-04-25

### Bug Fixes

- Do short-circuiting correctly in sign analysis
- Comparing validating security results without target

### Features

- Add progress bar to checko
- Make test execution async and add a timeout
- Show entire error description in checko UI

### Refactor

- Pass input to test container over stdin instead of args

## [0.1.20] - 2023-04-04

### Miscellaneous Tasks

- Use just the ref name instead of path for displaying version in inspectify
- Release

## [0.1.19] - 2023-04-04

### Bug Fixes

- Try to use GITHUB_REF env for determining version number

### Miscellaneous Tasks

- Release

## [0.1.18] - 2023-04-04

### Bug Fixes

- Mount a copy of checko to prevent writing to it from the container

### Features

- Improve logging and reporting of compile errors in failing groups
- Introduce batches an organized way to run tests
- Consistently number nodes
- Display input parse errors in inspectify instead of crashing
- Display invalid input and output in more cases
- Include version number in inspectify

### Miscellaneous Tasks

- Release

### Refactor

- Make graph construction follow the book more closely (#39)
- Use host checko binary in docker
- Remove checko dep from inspectify
- Move CLI out of checkr main
- Regenerate input on generate program

## [0.1.17] - 2023-03-28

### Bug Fixes

- Fix array lowering into egg
- Correctly substitute in `fac` and `fib`

### Miscellaneous Tasks

- Release

### Refactor

- Improve error reporting in checko a bit more
- Print errors using debug in checko for backtrace

## [0.1.16] - 2023-03-28

### Bug Fixes

- Ensure quantifier normalization does not use already present names

### Features

- Print out errors with context in checko evaluation

### Miscellaneous Tasks

- Fix clippy warnings
- Update lalrpop
- Release

## [0.1.15] - 2023-03-27

### Features

- Implement evaluation of `AExpr::Function`s

### Miscellaneous Tasks

- Cleanup the Justfile and remove Dockerfile.dev
- Release

### Refactor

- Abstract integer type into a common `Int` type
- Improve checko logging and reliability

## [0.1.14] - 2023-03-24

### Bug Fixes

- Change associativity of power to right (#30)
- Make unary minus checked (#32)

### Documentation

- Add a note about developing on Windows

### Features

- Allow empty ProgramsConfig TOML files

### Miscellaneous Tasks

- Update fsharp-starter ref
- Release

### Refactor

- Unary minus refactor (#33)
- Remove `#![feature(try_blocks)]` so we compile on stable!
- Wrap untyped IO and program config allows per env input

## [0.1.13] - 2023-03-23

### Bug Fixes

- Recompute the graph in inspectify after recompilation
- Remove copy of removed rust-toolchain.toml into Dockerfile
- Do not crash if one group did not produce a run result in checko
- Update definition vc(b -> C) to include b
- Produce the correct number of configurations in interpreter
- Correct set spans in checko
- Do not crash if pushing results to one group fails

### Documentation

- Update development requirements for inspectify
- Change `typeshare` to `typeshare-cli` in unix setup script
- Finish a sentence in inspectify readme

### Features

- Randomize determinism in interpreter input
- Initial draft of predicates and enriched commands
- Add substitution to enriched expressions
- Implement SP and VC for program verification analysis
- Generate annotated programs
- Validation for PV and remove all WASM
- Use WebSockets for compilation status in inspectify
- Allow specifying which programs are shown in results individually
- Display invalid output in inspectify
- Improve graph debugging inspectify
- Do not crash if we could not pull from the result branch

### Miscellaneous Tasks

- Add a license to Cargo.toml
- Do not open F# panel on startup in VSCode
- Test commit to see build times with smtlib and z3
- Remove z3 dep again, and correct pv output format
- Update fsharp-starter ref
- Update Cargo.lock
- Add a comment to the Justfile for running inspectify against fsharp-starter
- Release

### Refactor

- Improve inspectify internals
- Split inspectify backend up into multiple files
- Catch all errors from single groups to not crash when pushing results

## [0.1.12] - 2023-03-16

### Bug Fixes

- Swap & & && in when formatting to be correct
- Correctly error on empty and programs with trailing `;`

### Features

- Add ignore option to `run.toml`
- Update to the new IO for interpreter

### Miscellaneous Tasks

- Remove rust-toolchain.toml
- Remove some unused deps
- Remove #![feature(box_patterns, box_syntax)]
- Update fsharp-starter ref
- Add easy to use option to run inspectify against fsharp-starter
- Update fsharp-starter ref
- Prepare changelog for release
- Release

## [0.1.11] - 2023-03-11

### Bug Fixes

- Correct printing of | and || which were swapped
- Update binswap with new swapping procedure to hopefully fix win issues

### Miscellaneous Tasks

- Add cliff config
- Bump wasm lock file
- Revert versions to be inherited from workspace
- Add CHANGELOG
- Release

## [0.1.9] - 2023-03-10

### Miscellaneous Tasks

- Release

## [0.1.8] - 2023-03-10

### Miscellaneous Tasks

- Release

## [0.1.7] - 2023-03-09

### Miscellaneous Tasks

- Release

## [0.1.6] - 2023-03-08

### Miscellaneous Tasks

- Release

## [0.1.5] - 2023-02-21

### Miscellaneous Tasks

- Release

## [0.1.4] - 2023-02-21

### Miscellaneous Tasks

- Release

## [0.1.3] - 2023-02-13

### Miscellaneous Tasks

- Release

## [0.1.2] - 2023-02-13

### Miscellaneous Tasks

- Release
- Release

<!-- generated by git-cliff -->

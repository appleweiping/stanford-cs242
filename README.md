# Stanford CS242 — Programming Languages

> Lambda calculus, type systems, and language implementation — an independent,
> from-skeleton implementation of the programming assignments of
> **CS242 — Programming Languages** (Stanford, Will Crichton), part of a
> [csdiy.wiki](https://csdiy.wiki/) full-catalog build.

![status](https://img.shields.io/badge/status-4%20assignments%20verified-brightgreen)
![language](https://img.shields.io/badge/OCaml%20%2B%20Rust-informational)
![license](https://img.shields.io/badge/license-MIT-blue)

## Overview

Stanford CS242 first develops classical PL theory — the untyped and typed lambda
calculus, type systems, operational semantics — and then grounds it in real
systems programming with OCaml and Rust. This repository implements the
programming assignments **from the official course skeleton**, keeping the
course's own module layout, test harnesses, and (where they exist) graders.

Four assignments are **fully implemented and verified against the course's own
tests** on this machine:

| # | Assignment | Language | What it exercises |
|---|---|---|---|
| assign3 | Karel the Robot + N-gram text model | OCaml (Core) | recursion, pattern matching, higher-order functions, a small imperative interpreter, Markov generation |
| assign4 | System-F interpreter + typechecker (`Lam1`) | OCaml (Core) | STLC + sums/products, universals, existentials, iso-recursive types, `fix`; capture-avoiding substitution; CBV small-step semantics |
| assign7 | Async futures library | Rust | future combinators as state machines, single- and multi-threaded executors, async file IO, a smart pointer |
| assign8 | Typestate shopping cart | Rust | encoding a state machine in the type system so illegal transitions are **compile-time** errors |

The remaining course units (`assign1`, `assign2`, `assign5`, `assign6`, the labs,
and the four `final/` project options) are retained **as the course ships them**
— unmodified starter skeletons — and are *not* claimed as implemented here. See
[Scope & honesty](#scope--honesty).

## Results (measured on this machine — Windows + WSL2 Ubuntu, CPU-only)

All numbers below are **measured**, and the raw captures live in
[`results/`](results/).

| Assignment | Verification | Result (measured) |
|---|---|---|
| **assign3** | inline `assert` suites + real runs | Karel navigates & picks the beeper; checkerboard correct on 6×6, 5×5, 7×3, 1×1, 8×4; N-gram generates coherent Markov text from `hamlet.txt` / `hott.txt` |
| **assign4** | 8 example programs + inline test suites | **8/8** programs typecheck and evaluate to the expected value (`fact 5 = 120`, `poly = 1`, `counter = 2`, …) |
| **assign7** | `cargo test` | **27/27** tests pass (asyncio 4, executors 14, futures 7, usecount 2) |
| **assign8** | `cargo test` (incl. `trybuild` compile-fail) | **8/8** tests pass, including the test that asserts `.checkout()` on an empty cart is a **type error** |

### Sample: assign3 Karel checkerboard (6×6) and N-gram output

```
B.B.B.        ----- hamlet.txt, ngram=3, nwords=25 -----
.B.B.B        with the King, As England was his faithful tributary, As love
B.B.B.        between them like the kind life-rend'ring pelican, Repast them
.B.B.B        with my blood. King. Why,
B.B.B.
vB.B.B        (v = Karel's final position/heading; B = beeper)
```

### Sample: assign8 typestate safety (the key result)

The `cart_fail` test uses `trybuild` to confirm the compiler *rejects* an illegal
state transition — calling `checkout()` before adding any item:

```
error[E0599]: no method named `checkout` found for struct `Cart<Empty>`
  = note: the method was found for `Cart<NonEmpty>`
```

That compile error *is* the assignment passing: the shopping-cart state machine
is enforced statically, not with runtime checks.

## Implemented assignments

- [x] **assign3 — Karel + N-gram** (`assign3/program/src/karel_impl.ml`, `ngram_impl.ml`)
  Grid renderer, predicate evaluation (`FrontIs`/`Facing`/… with off-grid = wall),
  the `Move`/`Turn`/`Pick`/`Put`/`While`/`If` interpreter, and a coordinate-free
  checkerboard algorithm. N-gram: sliding windows, prefix→successor multiset map,
  empirical distribution, inverse-CDF sampling, context-sliding generation.
- [x] **assign4 — System-F interpreter + typechecker** (`assign4/src/{ast_util,typecheck,interpreter}.ml`)
  Capture-avoiding substitution over types and terms, de Bruijn conversion for
  alpha-equivalence, the full typing relation, and CBV single-step reduction for
  every form (`lam`/`app`, `pair`/`project`, `inj`/`case`, `fix`, `tyfun`/`tyapp`,
  `fold`/`unfold`, existential `export`/`import`).
- [x] **assign7 — Async futures** (`assign7/src/{future,executor,asyncio,usecount}.rs`)
  `Join` and `AndThen` combinators, a cooperative single-thread executor and an
  `mpsc` + worker-pool multi-thread executor, a background-thread async
  `FileReader`, and a `Deref`-counting smart pointer.
- [x] **assign8 — Typestate cart** (`assign8/program/src/cart.rs`)
  `Cart<S>` with phantom state markers `Empty`/`NonEmpty`/`Checkout`; each `impl`
  block exposes only the operations legal in that state, so the type checker
  enforces the cart's state machine.

## Project structure

```
stanford-cs242/
├── assign1/  assign2/          course written/lambda skeletons (unmodified)
├── assign3/  program/src/      IMPLEMENTED — Karel + N-gram (OCaml)
├── assign4/  src/ examples/    IMPLEMENTED — System-F interpreter (OCaml)
├── assign5/  assign6/          WebAssembly skeletons (unmodified)
├── assign7/  src/ tests/       IMPLEMENTED — async futures (Rust)
├── assign8/  program/src/      IMPLEMENTED — typestate cart (Rust)
├── final/    dlang lean nfstar rlu   final-project option skeletons (unmodified)
├── lab1/  lab2/                lab skeletons (unmodified)
├── results/                    captured test output for the 4 implemented units
├── LICENSE                     MIT (covers our own implementation code only)
└── README.md
```

## How to run

The four implemented units use two toolchains. On Windows the Rust GNU
(`mingw`) toolchain cannot link the `trybuild` proc-macro dependency, so the
Rust tests are run inside **WSL2 Ubuntu** (repo visible at `/mnt/d/...`); the
OCaml units are also easiest under a `4.14 + Core v0.16` opam switch in WSL.

```bash
# ---- assign7: async futures (Rust) ----
cd assign7
cargo test                 # -> 27/27 pass

# ---- assign8: typestate cart (Rust) ----
cd assign8/program
cargo test                 # -> 8/8 pass, incl. the trybuild compile-fail test

# ---- assign3: Karel + N-gram (OCaml, Core v0.16) ----
cd assign3/program
make && ./karel.native     # runs the Karel problems + inline asserts
# N-gram: ./ngram.native hamlet.txt -ngram 3 -nwords 25

# ---- assign4: System-F interpreter (OCaml, Core v0.16) ----
cd assign4
make
./main.native -v -t examples/fact.lam   # Type: num, value 120
```

> Toolchain-compat note: the course was written for the OCaml-4.02 / f19-era
> Core. The implementations here were adapted to **OCaml 4.14 + Core v0.16**
> (`Command.run → Command_unix.run`, `Map.find/set`, `String.equal`/`Poly.equal`
> for Core's shadowed `=`, and a `Flags → Lam_flags` rename to avoid a
> `core_kernel.Flags` clash). The shipped `reference.byte` is 4.02 bytecode and
> will not run under 4.14, so assign4 is verified against its own inline test
> suites plus the known result of each example program.

## Verification

- **assign7** — `cargo test` in WSL: `27 passed; 0 failed`. Raw output:
  [`results/assign7_futures_test.txt`](results/assign7_futures_test.txt).
- **assign8** — `cargo test` in WSL: `8 passed; 0 failed`, including the
  `trybuild` compile-fail test that proves the typestate is statically enforced.
  Raw output: [`results/assign8_cart_test.txt`](results/assign8_cart_test.txt).
- **assign4** — every example program is type-checked and evaluated; 8/8 match
  the expected value; the `ast_util`/`typecheck`/`interpreter` inline suites pass.
  Raw output: [`results/assign4_interpreter_results.txt`](results/assign4_interpreter_results.txt).
- **assign3** — inline `assert`s (`remove_last`, `compute_ngrams`,
  `ngram_map_add`) pass at module load; Karel and N-gram runs captured in
  [`results/assign3_karel_ngram_results.txt`](results/assign3_karel_ngram_results.txt).

## Scope & honesty

Per the project's no-toys / no-fabrication rule, this README claims only what is
actually implemented and verified. **Four** assignments (assign3, assign4,
assign7, assign8) are fully implemented in dedicated feature commits and pass the
course's own tests, as captured in `results/`. The other units — `assign1`,
`assign2`, `assign5`, `assign6`, `lab1`, `lab2`, and all four `final/` project
options (`dlang`, `lean`, `nfstar`, `rlu`) — are present **only as the
unmodified official skeleton** imported in the first commit; their solution slots
still contain the course's `unimplemented!()` / `sorry` / `fill this in`
placeholders and are **not** claimed as done here.

## Tech stack

- **OCaml 4.14** with **Core v0.16** — assign3, assign4
- **Rust** (1.96, edition 2015/2018) with `rand`, `trybuild` — assign7, assign8
- Verification host: Windows 11 + **WSL2 Ubuntu**, CPU-only

## Key ideas / what I learned

- **Types as proofs of protocol.** The typestate cart (assign8) turns a runtime
  state machine into a compile-time one — an illegal transition is simply not a
  callable method. The compile-fail test is the proof.
- **Implementing a type system.** assign4 makes the System-F typing rules and CBV
  semantics concrete, including the subtle bits: capture-avoiding substitution
  over *both* type and term binders and alpha-equivalence via de Bruijn indices.
- **Futures are state machines.** assign7 builds `Join`/`AndThen` and the
  executors from scratch, making explicit what `async`/`await` compiles down to.
- **A tiny interpreter, twice.** Karel (assign3) is an imperative interpreter
  over a grid; the N-gram model is a probabilistic one over text — the same
  "walk a structure, thread state" shape in two very different guises.

## Credits & license

Based on the assignments of **Stanford CS242 — Programming Languages** by
**Will Crichton** (Stanford University). This repository is an independent
educational reimplementation; all course materials, handouts, starter skeletons,
and reference binaries belong to their original authors. The original
implementation code in this repository (the four implemented assignments) is
released under the [MIT License](LICENSE).

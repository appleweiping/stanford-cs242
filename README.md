# Stanford CS242 — Programming Languages

> Lambda calculus, type systems, WebAssembly, and concurrency — an independent,
> from-skeleton implementation of the assignments of
> **CS242 — Programming Languages** (Stanford, Will Crichton, f19), part of a
> [csdiy.wiki](https://csdiy.wiki/) full-catalog build.

![status](https://img.shields.io/badge/status-8%20assignments%20%2B%201%20final%20verified-brightgreen)
![language](https://img.shields.io/badge/OCaml%20%2B%20Rust%20%2B%20WebAssembly%20%2B%20LaTeX-informational)
![license](https://img.shields.io/badge/license-MIT-blue)

## Overview

Stanford CS242 develops classical PL theory — the untyped and typed lambda
calculus, operational semantics, type systems, linear/affine types — and grounds
it in real systems programming with OCaml, Rust and WebAssembly. This repository
implements the assignments **from the official course skeleton**, keeping the
course's own module layout, test harnesses, and (where they exist) graders.

Every assignment **assign1–assign8** is implemented and verified on this machine,
plus **one of the four `final/` project options** (Read-Log-Update in Rust):

| # | Assignment | Language | What it exercises |
|---|---|---|---|
| assign1 | JSafe — JSON schemas & accessor safety | LaTeX (proof) | grammars, matching relations, small-step semantics, an induction proof of type safety |
| assign2 | Untyped lambda calculus | lambda (`lci`) | Church encodings of parity, multiplication, and recursion via the Y combinator |
| assign3 | Karel the Robot + N-gram text model | OCaml (Core) | recursion, pattern matching, a small imperative interpreter, Markov generation |
| assign4 | System-F interpreter + typechecker (`Lam1`) | OCaml (Core) | STLC + sums/products, universals, existentials, iso-recursive types, `fix`; CBV semantics |
| assign5 | WebAssembly `mystery` | WebAssembly (WAT) | structured control flow (`block`/`loop`/`br_if`) reverse-engineered from x86 |
| assign6 | Linear Types — BST + SLang→WASM compiler | Rust + OCaml + WASM | `mem::replace` surgery without cloning; an affine typechecker; string codegen with manual alloc/free |
| assign7 | Async futures library | Rust | future combinators as state machines, single/multi-thread executors, async file IO |
| assign8 | Typestate shopping cart | Rust | a state machine encoded in the type system, so illegal transitions are **compile-time** errors |
| **final** | Read-Log-Update (RLU) | Rust | a lock-free-reader concurrency scheme from SOSP '15 + a concurrent linked-list set |

## Results (measured on this machine — Windows + WSL2 Ubuntu, CPU-only)

All numbers below are **measured**; the raw captures live in
[`results/`](results/).

| Assignment | Verification | Result (measured) |
|---|---|---|
| **assign1** | `pdflatex` compile of the proof | 4-page PDF builds cleanly; matching relation, accessor semantics, validity judgment, and the Accessor Safety induction proof all typeset |
| **assign2** | bundled `lci` interpreter | `even → 1 0 1 0 1`; `mult → 0 1 2 3 6 0 8 100`; `sum(0,1,2,5,10) → 0 1 3 15 55` — all correct |
| **assign3** | inline `assert` suites + real runs | Karel navigates & checkerboards correctly; N-gram generates coherent Markov text from `hamlet.txt`/`hott.txt` |
| **assign4** | 8 example programs + inline suites | **8/8** typecheck and evaluate to the expected value (`fact 5 = 120`, `poly = 1`, `counter = 2`, …) |
| **assign5** | `wat2wasm` + Node harness | **5/5** — `mystery(1)=1, (3)=8, (12)=10, (100)=26, (1000)=112` (Collatz stopping-time) |
| **assign6** | `cargo test` + `main.native` + Node harness | Rust BST **8/8**; SLang typechecker accepts 3 / rejects 3 (move + unused-var errors); generated WASM **3/3** with **zero leaked allocations** |
| **assign7** | `cargo test` | **27/27** (asyncio 4, executors 14, futures 7, usecount 2) |
| **assign8** | `cargo test` (incl. `trybuild` compile-fail) | **8/8**, including the test that asserts `.checkout()` on an empty cart is a **type error** |
| **final (rlu)** | `cargo test` + 20× stress + benchmark | `set_simple`, `set_thread` (16 readers/4 writers), `rlu_concurrent_partitioned` pass; **20/20** concurrent stress runs race-free; perf 50/100 vs the reference on a short run (see below) |

### Sample: assign5 (Collatz) and assign6 (linear-typed string concat)

```
mystery(3) = 8   (3→10→5→16→8→4→2→1, counting each step)   [assign5, 5/5]

SLang `x = "hello"; return x ++ " world"`  compiles to WASM that allocs a new
buffer, memcpy's both operands, frees them, and returns "hello world"     [assign6, 3/3]
Rejected: `x ++ x` (x moved) and `x = "hello"` with no use (would leak)
```

### Sample: assign8 typestate safety (the key result)

The `cart_fail` test uses `trybuild` to confirm the compiler *rejects* an illegal
state transition — calling `checkout()` before adding any item:

```
error[E0599]: no method named `checkout` found for struct `Cart<Empty>`
  = note: the method was found for `Cart<NonEmpty>`
```

That compile error *is* the assignment passing: the state machine is enforced
statically, not with runtime checks.

## Implemented assignments

- [x] **assign1 — JSafe** (`assign1/written/assign1.tex`)
  Property/schema grammar, `n ⊨ p` and `j ~ τ` matching, small-step accessor
  semantics (`E-Key`/`E-Idx`/`E-Map`), a result-schema-carrying validity
  judgment, and a rule-induction proof of Accessor Safety.
- [x] **assign2 — Untyped lambda calculus** (`assign2/program/{even,mult,sum}.lam`)
  `even = fun n → n not true`, `mult = fun m n f → m (n f)`, and a Y-combinator
  `sum` with an `iszero` helper.
- [x] **assign3 — Karel + N-gram** (`assign3/program/src/karel_impl.ml`, `ngram_impl.ml`)
- [x] **assign4 — System-F interpreter + typechecker** (`assign4/src/{ast_util,typecheck,interpreter}.ml`)
- [x] **assign5 — WebAssembly mystery** (`assign5/program/wasm/src/mystery.wat`)
  Collatz total-stopping-time counter in structured WASM, mirroring the given
  `asm/mystery.s`; headless runner in `assign5/program/wasm/run_tests_node.js`.
- [x] **assign6 — Linear Types** (`assign6/program/rust/src/lib.rs`,
  `assign6/program/ocaml/src/{slang,translate}.ml`)
  A `BinaryTree<T>` (len/to_vec/sorted/insert/ceiling-search/rebalance) done with
  `mem::replace` and **no `T: Clone`**; an affine SLang typechecker (using a
  variable *moves* it; unused variables leak) and its WASM string-`concat`
  codegen. Headless runner in `assign6/program/wasm/run_tests_node.js`.
- [x] **assign7 — Async futures** (`assign7/src/{future,executor,asyncio,usecount}.rs`)
- [x] **assign8 — Typestate cart** (`assign8/program/src/cart.rs`)
- [x] **final — Read-Log-Update** (`final/rlu/program/src/{rlu,rlu_set}.rs`)
  RLU from scratch (global clock, per-object copies, lock-free reader
  dereference, write log, quiescence-gated commit) + a concurrent sorted
  linked-list set. Design notes in `final/rlu/written/final.tex`.

## Project structure

```
stanford-cs242/
├── assign1/  written/            IMPLEMENTED — JSafe proof (LaTeX)
├── assign2/  program/            IMPLEMENTED — Church-encoded lambda programs
├── assign3/  program/src/        IMPLEMENTED — Karel + N-gram (OCaml)
├── assign4/  src/ examples/      IMPLEMENTED — System-F interpreter (OCaml)
├── assign5/  program/wasm/       IMPLEMENTED — WebAssembly mystery (+ node runner)
├── assign6/  program/{rust,ocaml,wasm}/  IMPLEMENTED — Linear Types (Rust + OCaml→WASM)
├── assign7/  src/ tests/         IMPLEMENTED — async futures (Rust)
├── assign8/  program/src/        IMPLEMENTED — typestate cart (Rust)
├── final/    rlu/                IMPLEMENTED — Read-Log-Update (Rust); dlang/lean/nfstar skeletons
├── lab1/  lab2/                  reference solutions ship with the course (OCaml / Rust)
├── results/                      captured test output for every implemented unit
├── LICENSE                       MIT (covers our own implementation code only)
└── README.md
```

## How to run

Two toolchains, both easiest under **WSL2 Ubuntu** (repo visible at `/mnt/d/...`):
an OCaml `4.14 + Core v0.16` opam switch, and the Rust GNU toolchain (the
`trybuild` proc-macro won't link under Windows `mingw`, so Rust runs in WSL). The
WebAssembly units also use `wabt` (`wat2wasm`) and Node.

```bash
# ---- assign2: lambda calculus ----
cd assign2/program && ./lci_linux even.lam        # 1 0 1 0 1

# ---- assign5: WebAssembly (needs wabt + node) ----
cd assign5/program/wasm && node run_tests_node.js  # 5/5

# ---- assign6: Linear Types ----
cd assign6/program/rust  && cargo test             # 8/8
cd assign6/program/ocaml && make && ./main.native  # emits basic/concat/funcall.wat
cd assign6/program/wasm  && node run_tests_node.js # 3/3, no leaks

# ---- assign7 / assign8: Rust ----
cd assign7 && cargo test                           # 27/27
cd assign8/program && cargo test                   # 8/8 (incl. trybuild compile-fail)

# ---- assign3 / assign4: OCaml (Core v0.16) ----
cd assign3/program && make && ./ngram.native hamlet.txt -ngram 3 -nwords 25
cd assign4 && make && ./main.native -v -t examples/fact.lam   # Type: num, value 120

# ---- final: Read-Log-Update ----
cd final/rlu/program && cargo test                 # set_simple, set_thread, rlu_* pass
cargo run --release --bin bench -- -s rlu -t 1000 -n 1   # throughput CSV

# ---- assign1: written proof ----
cd assign1/written && pdflatex -shell-escape assign1.tex   # 4-page PDF
```

> Toolchain-compat notes. The OCaml units target the f19-era Core, adapted to
> **OCaml 4.14 + Core v0.16** (`Command.run → Command_unix.run`, generic
> `Set.mem`/`Map.find`, a `Flags → Lam_flags` rename). The course's `wasm.ml`
> emits pre-2019 WASM text (`get_local`, …); the Node runners translate those
> tokens before calling `wat2wasm`. `assign1` needs a one-line `tocstyle.sty`
> stub because the course preamble loads a package modern TeX Live dropped.

## Scope & honesty

Per the project's no-toys / no-fabrication rule, this README claims only what is
actually implemented and verified.

- **All eight assignments (assign1–assign8)** have their programming/proof
  deliverables implemented and passing the course's own tests/examples, captured
  in [`results/`](results/).
- **One final-project option is genuinely done**: `final/rlu` (Read-Log-Update),
  which passes all `cargo` tests and 20/20 concurrent stress runs.
- **Honest partials / not claimed:**
  - The RLU **performance** grade is ~50/100 vs the reference on a short
    benchmark: reads scale well (~60% of reference), but writers are serialized
    by a single global lock rather than the reference's fine-grained per-object
    locking. Correctness is unaffected. Documented in `final/rlu/written/final.tex`.
  - The other three `final/` options (`dlang` OCaml autodiff language, `lean`
    proofs, `nfstar` F* server) remain **unmodified skeletons** — each needs a
    large extension or a toolchain (Lean 3 / F*) that could not be installed
    headless here.
  - The **written** sub-parts of assign2/assign5/assign6 (`*.tex` derivations)
    are left as the course ships them; the graded **programming** deliverables of
    those units are complete.
  - `lab1`/`lab2` ship the course's own reference `solution/` directories, so no
    student work was required there.

## Tech stack

- **OCaml 4.14** + **Core v0.16** — assign3, assign4, assign6 (SLang compiler)
- **Rust** (1.96) with `rand`, `trybuild`, `clap` — assign6 (BST), assign7,
  assign8, final (RLU)
- **WebAssembly** via `wabt` (`wat2wasm` 1.0.34) + **Node 18** — assign5, assign6
- **LaTeX** (TeX Live 2023) — assign1
- Verification host: Windows 11 + **WSL2 Ubuntu**, CPU-only

## Key ideas / what I learned

- **Types as proofs of protocol.** The typestate cart (assign8) and the affine
  SLang typechecker (assign6) both turn a runtime property — a legal state
  transition, single-use ownership of a heap buffer — into a compile-time one.
- **Implementing a type system, and proving one safe.** assign4 makes the
  System-F rules executable; assign1 proves an analogous safety theorem on paper
  by rule induction with a preservation-strengthened lemma.
- **Down to the metal.** assign5 hand-writes WASM control flow; assign6 compiles a
  language to WASM with explicit `alloc`/`memcpy`/`dealloc`; the RLU final builds
  a lock-free-reader concurrency scheme from atomics and a global clock.
- **Futures are state machines.** assign7 builds `Join`/`AndThen` and the
  executors from scratch, making explicit what `async`/`await` compiles down to.

## Credits & license

Based on the assignments of **Stanford CS242 — Programming Languages** by
**Will Crichton** (Stanford University). This repository is an independent
educational reimplementation; all course materials, handouts, starter skeletons,
and reference binaries belong to their original authors. The original
implementation code in this repository is released under the [MIT License](LICENSE).

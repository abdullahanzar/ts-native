# TS-Native

**TS-Native** is an experimental, specification-first programming language that preserves the syntax and ergonomics of TypeScript while defining **explicit, deterministic semantics suitable for native compilation**.

TS-Native is compiled **ahead-of-time (AOT)** to **LLVM Intermediate Representation (LLVM IR)**, and then lowered to optimized machine code for multiple architectures such as x86-64, ARM64, and RISC-V.

This repository currently focuses on the **language specification**, not a production-ready compiler.

---

## Why TS-Native?

TypeScript today is an extraordinarily productive language, but its runtime story is inherited from JavaScript:

- Types are erased at runtime
- Execution depends on JavaScript semantics
- Implicit coercions and dynamic behavior are common
- Runtime behavior can be difficult to reason about precisely

TS-Native explores a different direction:

> What if TypeScript’s *shape* came with **native, explicit, and inspectable semantics**?

In TS-Native:

- Types are **semantically meaningful**
- Runtime behavior is **fully specified**
- Every value has a **defined representation and lifetime**
- Unspecified behavior is rejected by default

---

## What This Project Is

TS-Native is:

- A **compiled language**, not a transpiler
- **Ahead-of-time**, not JIT-based
- **Native**, with no JavaScript runtime dependency
- **TypeScript-shaped**, preserving familiar syntax and inference
- **LLVM-backed**, delegating optimization and code generation to LLVM

Conceptual compilation pipeline:

```
TypeScript-like Source
        ↓
 Parsing & Type Checking
        ↓
   TS-Native IR (semantic lowering)
        ↓
        LLVM IR
        ↓
   LLVM Optimizer
        ↓
 Native Machine Code
```

Language semantics are explicitly separated from code generation, allowing the language to evolve independently of target architectures.

---

## What This Project Is Not

TS-Native is **not**:

- JavaScript-compatible at runtime
- A drop-in replacement for the existing TypeScript compiler (`tsc`)
- Required to preserve JavaScript edge-case behavior (truthy/falsy, implicit coercions, etc.)
- A browser-first language or scripting environment

Where JavaScript behavior conflicts with native determinism, TS-Native intentionally chooses **clarity and explicitness**.

---

## Status

- 🧪 **Experimental**
- 📜 **Specification-first**
- 🔨 Compiler implementation: *future work*
- 💥 Breaking changes expected until a stable v1

Current focus areas:

- Defining a minimal, coherent semantic core
- Making runtime behavior precise and implementable
- Ensuring clean lowering to LLVM IR

---

## Repository Structure (Current / Planned)

```
/spec
  ts-native-spec-v0.md
  sections/
    01-introduction.md
    02-types.md
    03-values.md
    04-memory.md
    ...

/notes
  design-decisions.md
  rejected-ideas.md

/compiler   (future)
```

---

## Who Is This For?

This project is intended for:

- Compiler and runtime implementers
- Language designers and researchers
- Advanced TypeScript users curious about native compilation
- People who enjoy carefully arguing about semantics

No prior LLVM expertise is required to read or contribute to the specification.

---

## Contributing

Contributions are welcome, especially:

- Specification clarifications
- Edge-case analysis
- Alternative semantic proposals
- Proof-of-concept implementations
- Thoughtful, well-reasoned criticism

This is an exploratory project. Curiosity and rigor are valued over certainty.

A `CONTRIBUTING.md` will be added as the project evolves.

---

## License

Apache-2.0

---

## Philosophy (Short Version)

> Explicit semantics over implicit magic.  
> Familiar syntax, unfamiliar rigor.  
> Defaults with escape hatches.  
> Minimal core, extensible surface.


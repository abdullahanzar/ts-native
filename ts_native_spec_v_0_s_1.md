# TS‑Native Specification v0 — Section 1: Introduction

## 1.1 Purpose

This document defines the initial specification for **TS‑Native**, a statically compiled programming language that preserves the *syntax, ergonomics, and developer experience* of TypeScript while providing **explicit, concrete runtime semantics** suitable for native execution.

TS‑Native source code is compiled ahead‑of‑time (AOT) into **LLVM Intermediate Representation (LLVM IR)**. The generated LLVM IR is then passed to LLVM toolchains to produce optimized machine code for multiple target architectures (such as x86‑64, ARM64, and RISC‑V).

Unlike TypeScript today, which erases types and relies on a JavaScript runtime, TS‑Native treats types as **semantically meaningful at runtime**. Every value has a well‑defined representation, lifetime, and behavior that can be reasoned about deterministically.

---

## 1.2 What TS‑Native Is

TS‑Native is:

- A **compiled language**, not a transpiler
- **Ahead‑of‑time**, not JIT‑based
- **Native**, with no dependency on a JavaScript runtime
- **TypeScript‑shaped**, preserving familiar syntax, inference, and structural typing concepts
- **LLVM‑backed**, delegating optimization and machine code generation to LLVM

The compiler pipeline is conceptually:

```
TypeScript‑like Source
        ↓
 Parsing & Type Checking
        ↓
  TS‑Native IR (semantic lowering)
        ↓
      LLVM IR
        ↓
   LLVM Optimizer
        ↓
   Native Machine Code
```

TS‑Native explicitly separates **language semantics** from **code generation**, allowing the language to evolve independently of target architectures.

---

## 1.3 What TS‑Native Is Not

TS‑Native is **not**:

- JavaScript‑compatible at runtime
- A drop‑in replacement for the existing TypeScript compiler
- Required to preserve JavaScript edge‑case behavior (e.g., implicit coercions, truthy/falsy semantics)
- A browser‑first language
- A dynamic scripting language

Although TS‑Native borrows heavily from TypeScript’s syntax and type system, it intentionally diverges from JavaScript where native compilation requires explicit, deterministic behavior.

---

## 1.4 Design Philosophy

The guiding principles of TS‑Native are:

### Explicit Semantics Over Implicit Magic

All runtime behavior must be explicitly defined in this specification. If behavior is not specified, it is considered invalid by default.

### TypeScript Ergonomics, Native Semantics

TS‑Native preserves:
- Type inference as a default
- Structural typing concepts
- Familiar syntax (`const`, `let`, functions, interfaces)

While redefining:
- Numeric representations
- Memory behavior
- Runtime value layouts

### Defaults With Escape Hatches

Reasonable defaults are provided for common cases (e.g., inferred numeric types), while programmers retain the ability to specify exact behavior using explicit annotations.

### Minimal Core, Extensible Surface

Version 0 of the specification defines a **minimal, coherent core** that can be cleanly lowered into LLVM IR. Advanced features (heap allocation, garbage collection, async runtimes, dynamic dispatch) are intentionally deferred to later versions.

---

## 1.5 Target Audience

This specification is intended for:

- Compiler and runtime implementers
- Language designers and researchers
- Advanced TypeScript users exploring native compilation
- Tooling authors building on TS‑Native

It is written to be both **precise enough to implement** and **approachable enough to reason about**, without requiring prior expertise in LLVM or compiler construction.

---

## 1.6 Versioning and Stability

This document defines **TS‑Native v0**.

- The v0 specification prioritizes clarity and implementability over completeness.
- Breaking changes are expected between versions until a stable v1 is declared.
- Each section of the specification is considered normative unless explicitly marked as non‑normative.

Later sections will build upon this foundation, expanding the language while preserving the guarantees established here.


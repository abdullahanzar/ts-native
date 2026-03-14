# TS-Native Specification v0 — Section 3: Variables, Bindings & Mutability

## 3.1 Overview

This section defines how variables are declared, bound, and mutated in TS-Native v0. It specifies the semantics of `const` and `let`, the relationship between bindings and values, and the guarantees provided by the language regarding mutability.

Unlike JavaScript and TypeScript, where mutability is largely a runtime convention, TS-Native treats mutability as a **compile-time enforced semantic property** that directly affects code generation and optimization.

All behavior described in this section is **normative**.

---

## 3.2 Bindings vs Values

In TS-Native, a **binding** associates an identifier with a value.

A binding has the following properties:

- A name (identifier)
- A static type
- A mutability attribute (`const` or `let`)
- A scope

The mutability of a binding is **distinct from** the mutability of the value it refers to. In TS-Native v0, however, all values are primitive and therefore immutable by nature. Future versions may introduce mutable composite values.

---

## 3.3 Variable Declarations

Variables are declared using either `const` or `let`.

### 3.3.1 `const` Declarations

A `const` declaration creates an **immutable binding**.

Example:

```ts
const x = 10;
```

Normative behavior:

- The binding `x` cannot be reassigned after initialization
- The value bound to `x` is fixed for the lifetime of the binding
- The type of `x` is inferred or explicitly declared at initialization

Attempting to reassign a `const` binding is a compile-time error.

```ts
const x = 10;
x = 20;   // compile-time error
```

---

### 3.3.2 `let` Declarations

A `let` declaration creates a **mutable binding**.

Example:

```ts
let y = 5;
y = 6;   // valid
```

Normative behavior:

- The binding may be reassigned to values of the same static type
- Reassignment to a value of a different type is a compile-time error

Example:

```ts
let y = 5;     // inferred as int
y = 6;         // valid
y = 3.5;       // compile-time error
```

---

## 3.4 Type Annotations in Declarations

Variables may include explicit type annotations.

Example:

```ts
const a: double = 5;
let b: int = 10;
```

Normative behavior:

- Explicit type annotations override inferred types
- The initializer expression must be assignable to the annotated type
- Implicit widening conversions (as defined in Section 2) are permitted

---

## 3.5 Initialization Rules

All variable declarations **must be initialized at the point of declaration** in TS-Native v0.

Example:

```ts
let x;      // invalid
let y = 10; // valid
```

This rule eliminates uninitialized memory and enables straightforward lowering to LLVM IR.

---

## 3.6 Scope Rules

Bindings follow **lexical (block) scope**.

Normative behavior:

- A binding is visible from its declaration point to the end of the enclosing block
- Inner scopes may shadow outer bindings
- Shadowing creates a new binding; it does not mutate the outer binding

Example:

```ts
const x = 5;
{
  const x = 10;  // shadows outer x
}
```

---

## 3.7 Reassignment Semantics

Reassignment is permitted only for `let` bindings.

Normative behavior:

- Reassignment updates the value associated with the binding
- The static type of the binding does not change
- Reassignment does not allocate a new binding

Reassignment is lowered to a store operation in LLVM IR.

---

## 3.8 Lifetime and Storage Duration

In TS-Native v0:

- All variables have **automatic storage duration**
- Variables are stack-allocated within their scope
- Variable lifetimes are statically determined

There is no heap allocation or garbage collection in v0.

---

## 3.9 Compile-Time Errors

The following conditions result in compile-time errors:

- Reassignment of a `const` binding
- Use of an uninitialized variable
- Assignment of a value that is not assignable to the binding’s type
- Redeclaration of a binding within the same scope

---

## 3.10 Summary

TS-Native v0 variable semantics provide:

- Clear distinction between immutable and mutable bindings
- Mandatory initialization
- Static type stability
- Predictable stack-based storage

These guarantees enable safe, efficient lowering into LLVM IR and form the basis for future extensions involving composite types, references, and heap allocation.


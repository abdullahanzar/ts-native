# TS-Native Specification v0 — Section 2: Primitive Types & Numeric Semantics

## 2.1 Overview

This section defines the **primitive value types** of TS-Native v0 and specifies their **runtime representations, default inference rules, and numeric behavior**.

Unlike TypeScript, where primitive types are erased and mapped onto JavaScript’s dynamic runtime, TS-Native primitives correspond to **concrete, fixed runtime representations**. These representations are designed to map directly and predictably to LLVM IR types.

All behavior described in this section is **normative**.

---

## 2.2 Design Goals for Primitives

The primitive type system is designed to:

- Be **explicit at runtime** and statically verifiable
- Avoid JavaScript-style implicit coercions
- Provide **sensible defaults** for common usage
- Allow **explicit programmer control** when needed
- Lower cleanly into LLVM IR without hidden boxing

Primitive types form the semantic foundation upon which all higher-level language features are built.

---

## 2.3 Primitive Type Set (v0)

TS-Native v0 defines the following primitive types:

- `int`
- `double`
- `bool`
- `void`

No other primitive types are defined in v0. Additional primitives (e.g. `float`, `u32`, `char`) may be introduced in later versions.

---

## 2.4 Integer Type (`int`)

### 2.4.1 Definition

`int` represents a **signed integer value**.

In TS-Native v0:

- `int` is defined as a **64-bit signed integer**
- Its runtime representation corresponds to LLVM `i64`

This choice prioritizes simplicity, portability, and alignment with common 64-bit architectures.

### 2.4.2 Integer Literals

An integer literal is a numeric literal **without a decimal point**.

Examples:

```ts
const a = 5;
const b = -42;
```

Normative behavior:

- Integer literals are inferred as type `int`
- Integer literals are represented at runtime as 64-bit signed integers

---

## 2.5 Floating-Point Type (`double`)

### 2.5.1 Definition

`double` represents a **64-bit IEEE-754 floating-point value**.

In TS-Native v0:

- `double` maps directly to LLVM `double`
- All floating-point operations follow IEEE-754 semantics

### 2.5.2 Floating-Point Literals

A floating-point literal is a numeric literal **containing a decimal point**.

Examples:

```ts
const x = 5.0;
const y = 3.14159;
```

Normative behavior:

- Floating-point literals are inferred as type `double`
- Floating-point literals are represented at runtime as IEEE-754 doubles

---

## 2.6 Boolean Type (`bool`)

### 2.6.1 Definition

`bool` represents a logical truth value.

In TS-Native v0:

- `bool` has exactly two values: `true` and `false`
- `bool` maps to LLVM `i1`

### 2.6.2 Boolean Semantics

Normative behavior:

- `bool` values may only be produced by boolean literals or boolean expressions
- No implicit truthy or falsy coercion exists
- Conditional expressions require a `bool` condition

Example:

```ts
if (x) { }        // invalid unless x is bool
if (x > 0) { }   // valid
```

---

## 2.7 Void Type (`void`)

### 2.7.1 Definition

`void` represents the **absence of a value**.

Normative behavior:

- `void` may only appear as a function return type
- Variables of type `void` are invalid

Example:

```ts
function log(x: int): void {
  // no return value
}
```

---

## 2.8 Type Inference Rules for Primitives

When a variable declaration omits an explicit type annotation, the compiler infers the type using the following rules:

- Integer literal → `int`
- Floating-point literal → `double`
- Boolean literal → `bool`

Example:

```ts
const a = 10;       // int
const b = 2.5;      // double
const c = true;     // bool
```

Explicit type annotations override inferred defaults.

---

## 2.9 Numeric Conversions

### 2.9.1 Implicit Conversions

The following implicit conversions are permitted:

- `int` → `double` (widening conversion)

Example:

```ts
const x: double = 5;   // valid
```

### 2.9.2 Disallowed Implicit Conversions

The following implicit conversions are **not permitted**:

- `double` → `int`
- `bool` → numeric types
- numeric types → `bool`

Such conversions require explicit casts (to be defined in later sections).

---

## 2.10 Arithmetic Semantics

Normative behavior:

- Arithmetic operators (`+`, `-`, `*`, `/`) operate on operands of the same primitive type
- Mixed-type arithmetic requires explicit or implicit conversion rules

Examples:

```ts
const a = 5 + 2;        // int
const b = 5.0 + 2.0;   // double
const c = 5 + 2.0;     // int → double, result is double
```

The result type of an arithmetic expression is the **most precise type involved** after applying allowed widening conversions.

---

## 2.11 Errors and Undefined Behavior

Normative behavior:

- Numeric overflow for `int` is defined to wrap according to two’s complement semantics
- Division by zero results in a compile-time error when statically detectable
- Floating-point exceptional values (`NaN`, `Infinity`) follow IEEE-754 behavior

No other undefined numeric behavior exists in v0.

---

## 2.12 Summary

Primitive types in TS-Native v0:

- Are concrete and runtime-significant
- Have explicit inference defaults
- Avoid JavaScript-style coercion
- Map directly to LLVM IR

These guarantees enable predictable native code generation and form the foundation for higher-level language features defined in subsequent sections.


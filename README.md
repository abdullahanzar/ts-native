# TS-Native

TS-Native is an experimental language and compiler effort that preserves TypeScript-like syntax while defining explicit runtime semantics suitable for native compilation.

The implementation is now bootstrapped as a Rust Cargo workspace aligned to the compiler pipeline:

```text
Source -> Parser -> Type Checker -> TS-Native IR -> LLVM IR -> Native Code
```

## Current Status
- Experimental
- Specification-first
- Compiler workspace scaffolded in Rust
- Breaking changes expected before v1

## Repository Layout

```text
crates/
  ts-native-parser/
  ts-native-types/
  ts-native-ir/
  ts-native-codegen/
  ts-native-cli/

docs/
  architecture.md
  development.md
  specification/
    README.md
    v0/
      00-introduction.md
      01-primitive-types.md
      02-variables-bindings.md

examples/
tests/fixtures/
scripts/
```

## Quick Start

```bash
./scripts/bootstrap.sh
./scripts/build.sh
./scripts/test.sh
./scripts/lint.sh
```

## Documentation
- Specification index: `docs/specification/README.md`
- v0 chapters: `docs/specification/v0/README.md`
- Architecture: `docs/architecture.md`
- Development setup: `docs/development.md`

## Contributing
See `CONTRIBUTING.md` for development workflow and contribution guidelines.

## License
Apache-2.0


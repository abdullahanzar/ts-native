# Contributing

Thanks for contributing to TS-Native.

## Development Flow
1. Install prerequisites listed in `docs/development.md`.
2. Run `./scripts/bootstrap.sh`.
3. Build and test before opening a PR.

## Commands
- Build: `./scripts/build.sh`
- Test: `./scripts/test.sh`
- Lint/format checks: `./scripts/lint.sh`

## Scope Guidance
- Keep specification changes synchronized with implementation intent.
- Prefer small, focused pull requests.
- Document semantic changes in the specification chapters.

## Pull Request Checklist
- [ ] Build passes (`cargo check --workspace`)
- [ ] Tests pass (`cargo test --workspace`)
- [ ] Lint/format checks pass
- [ ] Docs links and paths are valid
- [ ] Changelog updated when user-visible behavior changes

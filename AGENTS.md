# Rust Development Guidelines

This repository is written in Rust and uses Cargo for building and dependency
management. Contributors should follow these best practices when working on the
project:

01. **Run `make fmt`, `make markdownlint`, and `make lint`** before committing
    to ensure consistent code style and catch common mistakes. After formatting
    and linting, execute **`make test`** and **`make test-ddlog`** to validate
    both standard and DDlog-enabled builds.
02. **Write unit tests** for new functionality. Run `make test` in the root
    crate to ensure all tests pass.
03. **Document public APIs** using Rustdoc comments (`///`) so documentation can
    be generated with `cargo doc`.
04. **Prefer immutable data** and avoid unnecessary `mut` bindings.
05. **Handle errors with the `Result` type** instead of panicking where
    feasible.
06. **Use explicit version ranges** in `Cargo.toml` and keep dependencies
    up-to-date.
07. **Avoid unsafe code** unless absolutely necessary and document any usage
    clearly.
08. **Keep functions small and focused**; if a function grows too large,
    consider splitting it into helpers.
09. **Commit messages should be descriptive**, explaining what was changed and
    why.
10. **Check for `TODO` comments** and convert them into issues if more work is
    required.
11. **Validate Markdown Mermaid diagrams** with `make nixie` before submitting
    documentation changes.
12. **Do not construct SQL by concatenating strings.** Always use Diesel's query
    builder or parameter binding to avoid injection vulnerabilities.

These practices will help maintain a high-quality codebase and make
collaboration easier.

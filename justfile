# Lists the recipes.
default:
    @just --list

# Formats the code.
fmt:
    cargo fmt --all

# Runs the formatter check, clippy, and the documentation build, with warnings as errors.
lint:
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Runs all the tests.
test:
    cargo test --all-targets
    cargo test --doc

# Runs everything a change should pass.
check: lint test

# Checks dependency licenses, advisories, and sources. Needs cargo-deny.
deny:
    cargo deny check

# Reports test coverage. Needs cargo-llvm-cov.
coverage:
    cargo llvm-cov --all-targets --summary-only

# Installs the git hooks.
setup:
    prek install --install-hooks -t pre-commit -t pre-push

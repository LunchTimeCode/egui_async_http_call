@_list:
	just --list --unsorted


run:
    cargo run

test:
    cargo test

# Perform all verifications (compile, test, lint etc.)
verify: test lint

# Run the static code analysis
lint:
	cargo fmt --check
	cargo clippy --all-targets
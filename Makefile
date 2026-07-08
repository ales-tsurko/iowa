CARGO_PACKAGES := -p gg-parser -p gg-compiler
MIRI_FLAGS := -Zmiri-disable-isolation -Zmiri-tree-borrows
MIRI_PACKAGES := gg-parser gg-compiler
COVERAGE_LCOV := target/gg-lcov.info
CRAP_REPORT := target/gg-crap.md
CRAP_EXCLUDES := --exclude '**/build.rs' --exclude '**/benches/**' --exclude '**/examples/**' --exclude '**/tests.rs' --exclude '**/*_tests.rs' --exclude '**/tests/**'
CRAP_ALLOW := --allow unescape
SIMILARITY_PATHS := parser/src compiler/src
SIMILARITY_ARGS := $(SIMILARITY_PATHS) --threshold 0.92 --min-lines 12 --min-tokens 80 --fail-on-duplicates

.PHONY: check test-all lint fmt code-health

check:
	cargo check $(CARGO_PACKAGES) --all-features --all-targets

test-all:
	cargo test $(CARGO_PACKAGES) --all-features --all-targets

lint:
	@status=0; \
	cargo clippy $(CARGO_PACKAGES) --all-targets --all-features -- -D warnings || status=$$?; \
	cargo +nightly fmt --check $(CARGO_PACKAGES) || status=$$?; \
	exit $$status

fmt:
	cargo +nightly fmt $(CARGO_PACKAGES)

code-health:
	similarity-rs $(SIMILARITY_ARGS)
	cargo machete --skip-target-dir
	cargo llvm-cov $(CARGO_PACKAGES) --all-features --all-targets --lcov --output-path $(COVERAGE_LCOV)
	cargo crap --workspace --lcov $(COVERAGE_LCOV) $(CRAP_EXCLUDES) $(CRAP_ALLOW) --format markdown --output $(CRAP_REPORT)
	cargo crap --workspace --lcov $(COVERAGE_LCOV) $(CRAP_EXCLUDES) $(CRAP_ALLOW) --summary --fail-above
	@set -e; \
	for package in $(MIRI_PACKAGES); do \
		MIRIFLAGS="$(MIRI_FLAGS)" cargo +nightly miri test -p "$$package" --all-features; \
	done

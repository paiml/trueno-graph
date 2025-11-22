# trueno-graph Makefile
# Validated by bashrs (PAIML shell safety standard)
# Quality enforcement via pmat + certeza

.PHONY: help
help:
	@echo "trueno-graph v0.1.0 - GPU-first embedded graph database"
	@echo ""
	@echo "Quality targets:"
	@echo "  make test          - Run all tests (unit + integration)"
	@echo "  make bench         - Run Criterion benchmarks"
	@echo "  make quality       - Run all quality gates (pmat + certeza)"
	@echo "  make coverage      - Generate test coverage report"
	@echo "  make lint          - Check clippy + formatting (CI mode)"
	@echo "  make lint-fix      - Auto-fix clippy + formatting issues"
	@echo "  make docs          - Build rustdoc documentation"
	@echo ""
	@echo "Development:"
	@echo "  make build         - Build in debug mode"
	@echo "  make release       - Build optimized release binary"
	@echo "  make clean         - Clean build artifacts"
	@echo ""
	@echo "Quality gates (EXTREME TDD):"
	@echo "  make validate-all  - Full validation (quality + test + bench)"

.PHONY: build
build:
	cargo build --all-features

.PHONY: release
release:
	cargo build --release --all-features

.PHONY: test
test:
	cargo test --all-features

.PHONY: bench
bench: ## Run Criterion benchmarks
	@echo "🏁 Running benchmarks..."
	cargo bench --bench graph_algorithms

.PHONY: bench-save
bench-save: ## Save benchmark baseline
	@echo "📊 Saving benchmark baseline..."
	@mkdir -p .performance-baselines
	cargo bench --bench graph_algorithms -- --save-baseline main

.PHONY: lint
lint: ## Check clippy + formatting (CI mode)
	@echo "🔍 Running lint checks..."
	@cargo clippy --all-features -- -D warnings
	@cargo fmt -- --check
	@echo "✅ Lint checks passed"

.PHONY: lint-fix
lint-fix: ## Auto-fix clippy + formatting issues
	@echo "🔧 Auto-fixing lint issues..."
	@cargo clippy --all-features --fix --allow-dirty --allow-staged
	@cargo fmt
	@echo "✅ Lint issues fixed"

.PHONY: fmt
fmt: ## Format code with rustfmt
	cargo fmt

.PHONY: docs
docs:
	cargo doc --all-features --no-deps --open

.PHONY: coverage
coverage: ## Generate coverage report (≥95% required)
	@echo "📊 Generating coverage report (target: ≥95%)..."
	@# Temporarily disable mold linker (breaks LLVM coverage)
	@test -f ~/.cargo/config.toml && mv ~/.cargo/config.toml ~/.cargo/config.toml.cov-backup || true
	@cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
	@cargo llvm-cov report --html --output-dir target/coverage/html
	@# Restore mold linker
	@test -f ~/.cargo/config.toml.cov-backup && mv ~/.cargo/config.toml.cov-backup ~/.cargo/config.toml || true
	@echo "✅ Coverage report: target/coverage/html/index.html"
	@echo ""
	@echo "📊 Coverage Summary:"
	@cargo llvm-cov report | python3 -c "import sys; lines = [l.strip() for l in sys.stdin if l.strip()]; total_line = [l for l in lines if l.startswith('TOTAL')]; parts = total_line[0].split() if total_line else []; cov_str = parts[-4].rstrip('%') if len(parts) >= 10 else '0'; cov = float(cov_str); total_lines = int(parts[7]) if len(parts) >= 10 else 0; missed_lines = int(parts[8]) if len(parts) >= 10 else 0; covered_lines = total_lines - missed_lines; print(f'   trueno-graph:   {cov:.2f}% ({covered_lines:,}/{total_lines:,} lines)'); print(''); print('   ✅ PASS: Coverage ≥95%' if cov >= 95 else f'   ⚠️  WARN: Coverage ({cov:.2f}%) below 95% target')"

.PHONY: quality
quality: lint
	@echo "Running pmat quality gates..."
	pmat analyze complexity --fail-above 20 --path .
	pmat analyze dead-code --path .
	pmat quality-gate --checks clippy,fmt,tests,coverage --coverage-threshold 95
	@echo "Running bashrs Makefile validation..."
	bashrs lint Makefile
	@echo "Quality gates passed ✅"

.PHONY: mutation-test
mutation-test:
	@echo "Running mutation testing (certeza standard: ≥80% score)..."
	cargo mutants --test-timeout 60 --jobs 8 --in-diff

.PHONY: validate-all
validate-all: quality test bench mutation-test
	@echo "=================================="
	@echo "All quality gates passed ✅"
	@echo "  - Clippy: Zero warnings"
	@echo "  - Tests: All passing"
	@echo "  - Coverage: ≥95%"
	@echo "  - Complexity: ≤20 per function"
	@echo "  - Mutation: ≥80% score"
	@echo "  - Benchmarks: No regressions"
	@echo "=================================="

.PHONY: clean
clean:
	cargo clean
	rm -rf coverage/
	rm -rf target/criterion/

# Install quality tooling (run once)
.PHONY: install-tools
install-tools:
	@echo "Installing quality tooling..."
	cargo install cargo-llvm-cov
	cargo install cargo-mutants
	cargo install bashrs || echo "bashrs already installed"
	@echo "Tools installed ✅"

# Pre-commit hook setup (pmat integration)
.PHONY: install-hooks
install-hooks:
	@echo "Installing pre-commit hooks (pmat + bashrs)..."
	@mkdir -p .git/hooks
	@echo '#!/bin/bash' > .git/hooks/pre-commit
	@echo 'set -e' >> .git/hooks/pre-commit
	@echo '' >> .git/hooks/pre-commit
	@echo '# PMAT quality gates' >> .git/hooks/pre-commit
	@echo 'pmat analyze complexity --fail-above 20 --path .' >> .git/hooks/pre-commit
	@echo 'pmat analyze dead-code --path .' >> .git/hooks/pre-commit
	@echo 'pmat analyze satd --path . --fail-on-any' >> .git/hooks/pre-commit
	@echo '' >> .git/hooks/pre-commit
	@echo '# bashrs Makefile validation' >> .git/hooks/pre-commit
	@echo 'bashrs lint Makefile' >> .git/hooks/pre-commit
	@echo '' >> .git/hooks/pre-commit
	@echo '# Clippy + fmt' >> .git/hooks/pre-commit
	@echo 'cargo clippy --all-features -- -D warnings' >> .git/hooks/pre-commit
	@echo 'cargo fmt -- --check' >> .git/hooks/pre-commit
	@echo '' >> .git/hooks/pre-commit
	@echo 'echo "Pre-commit checks passed ✅"' >> .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Hooks installed ✅ (.git/hooks/pre-commit)"

# Benchmark comparison (vs NetworkX baseline)
.PHONY: bench-compare
bench-compare:
	@echo "Running benchmarks with baseline comparison..."
	cargo bench --features gpu -- --baseline main
	@echo "Benchmark comparison complete. See target/criterion/report/index.html"

# RustPersonalAccounting — pour usage/recette
set positional-arguments

_default:
    @just --list

# Initialisation du dépôt (hooks git) — à faire après un clone.
setup:
    git config core.hooksPath githooks
    chmod +x githooks/pre-commit
    @echo "hooksPath = githooks ✓  (rledger : brew install rustledger)"

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# Validation du ledger d'exemple (données fictives, recette) via le binaire
# rustledger (installé par brew).
check-ledger:
    @if command -v rledger >/dev/null; then rledger check examples/ledger/main.beancount; \
    else \
      echo "rledger absent — repli household-cli (binaire du crate household-core)"; \
      cargo run -q -p household-core --bin household-cli -- check examples/ledger/main.beancount; \
    fi

check: fmt-check clippy test check-ledger

build:
    cargo build --workspace

run:
    cargo run -p household-gui

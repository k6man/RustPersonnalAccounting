# RustPersonalAccounting — pour usage/recette
set positional-arguments

_default:
    @just --list

# Initialisation du dépôt (hooks git) — à faire après un clone.
setup:
    git config core.hooksPath githooks
    chmod +x githooks/pre-commit
    @echo "hooksPath = githooks ✓  (rledger : brew install rustledger)"

# Attache le hook pre-commit de CE dépôt à un dépôt de ledger externe
# (le vrai ledger du foyer, cf. docs/ADR-004) — une seule fois après
# création de ce dépôt. Usage : just setup-ledger /chemin/vers/ledger
setup-ledger dossier:
    #!/usr/bin/env bash
    set -euo pipefail
    git -C "{{dossier}}" rev-parse --show-toplevel >/dev/null 2>&1 \
        || { echo "« {{dossier}} » n'est pas un dépôt git ($(cd '{{dossier}}' 2>/dev/null || echo '?'))" >&2; exit 1; }
    [ -f "{{dossier}}/main.beancount" ] \
        || { echo "ERREUR : pas de main.beancount à la racine de « {{dossier}} » (convention ADR-004)" >&2; exit 1; }
    git -C "{{dossier}}" config core.hooksPath "$PWD/githooks"
    echo "hooks du ledger externe → $PWD/githooks ✓"

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

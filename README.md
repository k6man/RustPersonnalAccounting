# Comptabilité du foyer

Comptabilité personnelle de foyer (couple + 2 enfants, EUR) fondée sur **Beancount**,
moteur **rustledger** (implémentation Rust), suivi **git** natif, et une GUI **egui/eframe**.

> **Suivi du projet** : voir [`TODO.md`](TODO.md) — état courant, jalons, décisions.
> C'est le point d'entrée pour reprendre le travail à froid.

## État

| Jalon | Contenu | État |
|---|---|---|
| 1 | Workspace + ledger d'exemple valide | ✅ |
| 2 | `household-core` (lecture/écriture ledger) | |
| 3 | `household-budget` (grille, rollover) | |
| 4 | GUI lecture seule (dashboard, journal) | |
| 5 | Saisie/écriture de transactions | |
| 6 | Éditeur de budget postes × 12 mois | |
| 7 | Rapports, git dans la GUI, polish | |

## Installation

```sh
# Toolchain Rust (rust-toolchain.toml pinstable)
brew install rust just ripgrep
# Moteur beancount rust (CLI, hook pre-commit & CI)
brew install rustledger
# Hooks
just setup
```

## Lancer

```sh
just check        # fmt + clippy + tests + validation du ledger
just run          # GUI (e quanto present : squelette)
rledger check ledger/main.beancount   # validation ledger seule
just test         # cargo test --workspace
```

## Structure

```
/ledger                     # dépôt de comptabilité, texte, versionné (source de vérité)
  main.beancount            # options + includes (globs par année/budget)
  accounts.beancount        # open/close + plan de comptes (noms français, sans accents)
  commodities.beancount
  /budgets/2026.beancount   # budgets : custom "budget" (convention Fava)
  /transactions/2025…       # un fichier par mois ; croisement par date
/crates
  /household-core           # wrapper rustledger : load, snapshot, writer transactions (atomique)
  /household-budget         # budget pur : grille postes×12 mois, rollover, duplication d'année
  /household-git            # git2 : statut, commit (avec revalidation), historique
  /household-gui            # eframe/egui : état applicatif / commandes / vues
/docs                       # ADR (décisions d'architecture — arrivent au jalon 7)
```

## Conventions

- **Comptes** : noms en français, sans accents dans les identifiants (limite Beancount) ;
  libellés, narrations et UI en français.
- **Chiffres** : `rust_decimal::Decimal` de bout en bout ; format ledger `"1234.56 EUR"`.
- **Dates** : `rustledger_core::NaiveDate` (jiff) — pas de chrono.
- **Plafonds épargne** : métadonnée sur la directive `open` (`ceiling: 22950.00 EUR`).
- **Budgets** : `custom "budget"` Fava-compatible, un fichier par année, une directive = une
  ligne ; supersession datée (remise à zéro via `0.00 EUR`).
- **Écriture** : toujours atomique (fichier temporaire + rename), format canonique stable,
  aligned, diffs git propres.
- **Licence** : GPL‑3.0 (situation de dépendance vis‑à‑vis de rustledger, GPL‑3.0‑only).
```

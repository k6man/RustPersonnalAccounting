# Contexte projet — Comptabilité du foyer (RustPersonnalAccounting)

## Objectif
Comptabilité personnelle de foyer (couple + 2 enfants, EUR) fondée sur **Beancount**
(moteur Rust `rustledger`), versionnée dans git, pilotée par une **TUI ratatui**
(binaire `compta-foyer`). Le ledger est la seule source de vérité des données.

## Stack
- Rust, édition 2024, workspace de 4 crates : `household-core`, `household-budget`,
  `household-git`, `household-gui`.
- Moteur : `rustledger-* = "=0.24.0"` (épinglé exactement, API instable).
- UI : `ratatui 0.26` + `crossterm 0.27`. Git : `git2` (libgit2, pas de binaire git).
- Licence : GPL-3.0-only (ADR-001).

## Conventions non négociables
- Montants : `rust_decimal::Decimal` de bout en bout, jamais `f64` pour une somme.
- Dates : `rustledger_core::NaiveDate` (jiff), pas de chrono.
- Langue : code, commentaires, UI et comptes en français ; identifiants sans accents
  (`Expenses:Alimentation:Courses`).
- `unsafe_code = "forbid"` ; clippy `-D warnings`.
- Toute écriture du ledger passe par les writers de `household-core` / `household-budget`.
- Les tests ne touchent jamais `examples/ledger/` : ils travaillent sur une copie scratch.

## Validation avant commit
`just check` = fmt-check + clippy + `cargo test --workspace` + `rledger check`.

## Décisions d'architecture (docs/)
- ADR-001 : rustledger épinglé, GPL-3.0.
- ADR-002 : git2 en processus ; validation du ledger côté app avant commit (libgit2 n'exécute pas les hooks).
- ADR-003 : budgets en `custom "budget"` (convention Fava), supersession datée, rollover « cumul annuel ».
- ADR-004 : le vrai ledger vit dans un dépôt git séparé ; `examples/ledger/` = données fictives de recette.

## Cartographie des capabilities (par parcours utilisateur)
Chaque capability décrit ce que le foyer peut faire, pas un crate. La dernière colonne indique où vit
le code, à titre informatif.

| Parcours | Capability | Code principal |
|---|---|---|
| Ouvrir son ledger | ouvrir-ledger | household-core (resolution, snapshot), household-gui (worker, app) |
| Se déplacer dans l'application | naviguer-application | household-gui (app) |
| Voir sa situation | consulter-tableau-de-bord | household-gui (viewmodel, views/dashboard) |
| Retrouver / corriger une écriture | parcourir-journal | household-gui (views/journal, worker) |
| Enregistrer une écriture | saisir-transaction | household-gui (saisie, views/saisie) |
| Être sûr de ne rien casser | garantir-integrite-ecritures | household-core (writer) |
| Piloter son budget | piloter-budget | household-budget, household-gui (views/budget) |
| Analyser l'année | analyser-annee | household-gui (viewmodel, views/rapports, graphiques) |
| Valider et sauvegarder | valider-et-versionner | household-git, household-core (bin), githooks |
| Régler l'application | regler-application | household-gui (config) |

Capability transversale : `garantir-integrite-ecritures` s'applique à toute voie d'écriture, y compris
les changes futurs (ex. `importer-releve`).

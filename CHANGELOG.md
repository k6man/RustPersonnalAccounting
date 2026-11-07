# Changelog

Ce fichier est rédigé selon le **Keep a Changelog** en français. Chaque version
est claire et concrète : grands changements d'architecture, pas de
mises à jour des dépendances.

> **Versionnement** : version **0.1.0** au `Cargo.toml` (`[workspace.package]`),
> **non publiée** sur crates.io (pas de metadata de publication dans ce dépôt).
> La licence est GPL‑3.0‑only (ADR‑001).

## [0.1.0] — 2026-11-07

### Added

- **Versionnement + git** : `household-core` (wrapper rustledger, load/snapshot/
  resolution/writers atomiques), `household-budget` (grille, rollover,
  duplication d'année), `household-git` (git2 statut/commit/historique),
  `household-gui` (eframe/egui vue applicatif + worker + vues).
- **Ledger** : `examples/ledger/main.beancount` + budgets (Fava `custom
  budget`), validation par `rledger check` en CI.
- **GUI** : tableau de bord (soldes, plafonds à 3 états, patrimoine, budget),
  journal (saisie, double-clic édition, suppression par empreinte), éditeur de
  budget postes × 12 mois (hierarchique repliable, confirmation du diff,
  duplication d'année N+1, rollover par famille), rapports (dépenses par
  catégorie × mois, N vs N−1, épargne), graphiques (barres/cascade/bars),
  onglets Git (status/diff/commit suggéré/historique).
- **Architecture** : ADR‑001 (rustledger GPL‑3.0), ADR‑002 (validation app
  via libgit2), ADR‑003 (budget Fava), ADR‑004 (ledger vrai hors dépôt).
- **Mise en production** : CI (fmt/clippy/test/ledger/cargo-audit/cargo-deny),
  hook `pre-commit` de validation ledger, `justfile` de validation,
  `deny.toml`, conventions (Decimal bout en bout, NaiveDate, langues françaises).
- **Documentation publique** : README, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY,
  THIRD_PARTY_LICENSES.

[0.1.0]: https://github.com/…/releases/tag/v0.1.0
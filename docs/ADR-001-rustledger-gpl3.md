# ADR-001 — Moteur beancount : rustledger (Rust), relié GPL-3.0

**Statut** : acceptée (2026-10-29, jalon 1).

## Contexte

La comptabilité a besoin d'un moteur beancount : parse, booking, inventaires,
validation. Deux familles : l'original Python (beancount v3 / fava) et
**rustledger**, une implémentation Rust complète et publiée sur crates.io.

Le projet est une application Rust (GUI egui/eframe, writers atomiques) : les
deux mondes doivent cohabiter — app native ET validation outillage (hook
pre-commit, CI).

## Décision

- **Moteur** : rustledger, épinglé exact (`rustledger-* = "0.24.0"`).
- **Interface** : les crates `household-*` dépendent des crates publics
  `rustledger-core`, `rustledger-loader`, `rustledger-budget` ; la CLI
  `rledger` (binaire installé) est réservée à la validation scriptée.
- **Licence** : GPL-3.0 **only** pour ce dépôt — rustledger est GPL-3.0, et
  notre code est un travail dérivé (linking direct), donc la GPL s'applique.
  Toute future dépendance doit être vérifiée GPL-compatible.

## Conséquences

- Pas d'usage de `f64` sur les montants : les types du moteur (Decimal) sont
  propagés de bout en bout.
- Les écarts d'API entre le dépôt `main` de rustledger et la version publiée
  sont documentés au fil des jalons (TODO.md — « Reconnaissance ») : la
  source de vérité est la documentation des sources publiées.
- le hook pre-commit appelle `rledger check` ; la CI l'exécute aussi
  (`cargo binstall rustledger`).

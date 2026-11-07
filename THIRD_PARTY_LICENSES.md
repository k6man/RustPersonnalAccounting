# Dépendances de développement — licences

Lisez ce fichier si vous souhaitez contribuer : on vous a résolu les licences
des dépendances directes de `Cargo.toml` à l'aide de `cargo deny check
licenses` (jobs `cargo-deny` dans le CI). Le **résultat** — pour chaque
créateur / SPDX — est la vérité référencée, pas cette table.

## Dépendances directes

Référence `Cargo.toml` (workspace dependencies). La liste exacte, avec l'état
(lock version résultat de `cargo tree`), est émise par `cargo tree`.

| Créateur / crate | Licence (SPDX) | Note |
|---|---|---|
| `rustledger-core`, `rustledger-parser`, `rustledger-loader`, `rustledger-booking`, `rustledger-validate`, `rustledger-budget` (épinglé `=0.24.0`) | **GPL-3.0** | Moteur beancount. Correspond à la licence du dépôt (`LICENSE`, GPL‑3.0‑only). |
| `eframe`, `egui`, `egui_plot`, `egui_extras` | MIT | GUI egui/eframe. |
| `rust_decimal`, `rust_decimal_macros` | MIT | Types de montants `Decimal`. |
| `jiff` | MIT | Dates (`NaiveDate`). |
| `notify` | MIT | Watch du ledger. |
| `git2` | MIT | Statut/commit dans la GUI. |
| `thiserror`, `anyhow`, `serde`, `toml` | MIT | Librairies de typage/binaire. |

## Rationnel de la licence GPL‑3.0‑only

Le projet est un **travail dérivé** (linking direct) du moteur
`rustledger` (GPL‑3.0). La dépendance directe implique donc l'application de
la licence GPL au code du dépôt : la licence résultante est
**GPL‑3.0‑only**. C'est le choix documenté en [`ADR-001`](docs/ADR-001-rustledger-gpl3.md).

Toute future dépendance doit être vérifiée (licences, compatibilité GPL) avant
d'être ajoutée. Les lentes licences refusées par `cargo deny` génèrent des
jobs de recherche sur les branches en CI.

## Cherchez-vous votre licence ?

Les versions mobiles des crate publiées sur crates.io sont lisibles via le
registre ; vérifiez les licences actives selon la version exacte dans
`Cargo.lock` si un crat y avait changé de licence. Le CI `cargo-deny` fournit
le contrôle en production.
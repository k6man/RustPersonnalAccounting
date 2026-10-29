//! Couche d'accès au ledger beancount du foyer.
//!
//! Squelette du jalon 1 ; implémentation au jalon 2 :
//!
//! - **Chargement** via [`rustledger_loader::load`](https://docs.rs/rustledger-loader)
//!   avec `LoadOptions` par défaut (booking STRICT + validation) ;
//! - ** [`crate ]** `LedgerSnapshot` immuable (`Arc`) : balances par compte (via
//!   `Ledger::balance_view` + `Inventory`), directives sourcées (fichier + span)
//!   pour l'édition chirurgicale, plafonds lus des métadonnées `open` ;
//! - **Écriture** des transactions : append canonique (`format_directives`) dans
//!   `ledger/transactions/AAAA/MM.beancount`, remplacement/removal par span exact,
//!   écriture atomique (temporaire + rename), jamais de re-sérialisation du fichier entier ;
//! - **Binaire `household-cli`** (`check`, `balances`) pour le hook pre-commit et la CI.
//!
//! Conventions de types : `rustledger_core::NaiveDate` (jiff) pour les dates,
//! `rust_decimal::Decimal` pour les montants — ré-exports, zéro conversion.

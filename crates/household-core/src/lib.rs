//! Couche d'accès au ledger beancount (existant réel ou exemple de recette).
//!
//! - **Résolution** : [`resolution`] — chaîne unique (argument >
//!   `RPA_LEDGER` > config GUI > exemple du dépôt).
//! - **Chargement** : [`Snapshot::load`] exécute le pipeline rustledger complet
//!   (`rustledger_loader::load`, plugins, booking STRICT, validation) et
//!   expose l'état immuable : directives **sourcées** (fichier + span, pour
//!   l'édition chirurgicale), balances par devise via
//!   `rustledger_booking::BookingEngine` (les mêmes réflexes que `rledger
//!   report balances`), comptes ouverts et plafonds (`ceiling:`) sur les
//!   métadonnées `open`, payees par fréquence pour l'autocomplétion.
//! - **Écriture** : [`writer`] — append/remplacement/suppression canoniques,
//!   temporaire + rename, pré-vérification sur copie scratch puis
//!   revalidation du ledger réel, verrou `.write.lock`.
//! - **CLI** : `household-cli check` / `balances` (binaire du crate), appelé
//!   par `githooks/pre-commit` en repli de `rledger` et par la CI.
//!
//! Convention de type : `rustledger_core::NaiveDate` (jiff) pour les dates,
//! `rust_decimal::Decimal` pour les montants — ré-exports, zéro conversion.
//!
//! Sémantique beancount v3 (adoptée) : une assertion `balance` à la date D est
//! vérifiée à 00:00 de D ; les soldes prennent [`Through`] (`StartOfDay` /
//! `EndOfDay`).

pub mod error;
pub mod resolution;
pub mod snapshot;
pub mod writer;

pub use error::CoreError;
pub use resolution::{
    EXEMPLE_RELATIF, ModeLedger, ResolutionLedger, SourceResolution, VAR_ENV, resoudre_ledger,
};
pub use snapshot::{AccountBalance, AccountInfo, Ceiling, Snapshot, SourceRef, Through};
pub use writer::{append_transaction, monthly_file, remove_transaction, replace_transaction};

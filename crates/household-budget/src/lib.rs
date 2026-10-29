//! Logique budgétaire **pure** (indépendante de toute UI et de tout I/O pour
//! le calcul), testée unitairement.
//!
//! - **Grille** postes × 12 mois ([`grid`]) : budget mensuel effectif, réel,
//!   écart, % consommé — bâtie sur [`rustledger_budget`] (`Budgets::compare`
//!   par fenêtre mensuelle, sémantique Fava : accrual par jour, ancrage
//!   calendaire, supersession datée, agrégation hiérarchique `children`).
//! - **Rollover** ([`rollover`]) : report cumulé du non-dépensé/dépassement
//!   mois → mois (option, calcul maison non fourni par rustledger).
//! - **Duplication** d'année N→N+1 ([`duplicate`]) : copie des directives avec
//!   ajustement en pour cent et arrondi au multiple supérieur paramétrable.
//! - **Writer** ([`writer`]) : édition chirurgicale de `budgets/AAAA.beancount`
//!   (une directive = une ligne, span exact, commentaires préservés) au-dessus
//!   des garanties de `household-core::writer` (verrou, pré-vérification sur
//!   copie scratch, atomicité, revalidation).
//!
//! Le ledger (`budgets/AAAA.beancount`) reste la seule source de vérité.

pub mod duplicate;
pub mod grid;
pub mod rollover;
pub mod writer;

pub use duplicate::{DuplicationError, duplicate_year, interval_keyword};
pub use grid::{BudgetGrid, ColumnTotal, GridCell, GridRow};
pub use rollover::RolloverSeries;
pub use writer::{budget_directive, budget_file, set_cell, write_year};

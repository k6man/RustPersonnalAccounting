//! Grille budgétaire **postes × 12 mois** (module pur, aucun I/O).
//!
//! Construite sur [`rustledger_budget`] : chaque cellule vient d'un appel
//! mensuel à [`Budgets::compare`] (fenêtre `[1er, 1er du mois suivant)`),
//! avec `children = true` — un poste parent agrège ses sous-postes
//! (addition, pas absorption : chaque poste garde sa propre ligne).
//!
//! Sémantique héritée du modèle Fava/rustledger :
//! - budget mensualisé = accrual de la fenêtre (pro-rata par jour pour les
//!   directives datées en milieu de mois, dénominateur = durée calendaire) ;
//! - réel = écritures de la fenêtre, sign-normalisées, comptées à partir du
//!   début d'effet du budget (rien avant) ;
//! - un poste sans directive en force sur un mois → cellule à zéro.

use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use rustledger_budget::{Bucket, Budgets};
use rustledger_core::{Account, AccountTypes, Directive, NaiveDate};

/// Une cellule (mois) d'un poste : budget mensuel effectif et réel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GridCell {
    pub budgeted: Decimal,
    pub actual: Decimal,
}

impl GridCell {
    /// Écart budget − réel (positif : sous le budget).
    #[must_use]
    pub fn remaining(&self) -> Decimal {
        self.budgeted - self.actual
    }

    /// Fraction consommée (0..1) ; `None` si budget nul (division évitée).
    #[must_use]
    pub fn used_fraction(&self) -> Option<f64> {
        if self.budgeted.is_zero() {
            None
        } else {
            Some(f64::try_from(self.actual / self.budgeted).unwrap_or(f64::INFINITY))
        }
    }
}

/// Une ligne de la grille : un poste (compte budgété, devise) × 12 mois.
#[derive(Debug, Clone)]
pub struct GridRow {
    pub account: String,
    pub currency: String,
    /// Cellules janvier → décembre (index = mois − 1).
    pub cells: [GridCell; 12],
}

impl GridRow {
    /// Budget cumulé sur l'année.
    #[must_use]
    pub fn budgeted_year(&self) -> Decimal {
        self.cells.iter().map(|c| c.budgeted).sum()
    }

    /// Réel cumulé sur l'année.
    #[must_use]
    pub fn actual_year(&self) -> Decimal {
        self.cells.iter().map(|c| c.actual).sum()
    }

    /// Écart annuel (budget − réel).
    #[must_use]
    pub fn remaining_year(&self) -> Decimal {
        self.budgeted_year() - self.actual_year()
    }
}

/// Totaux d'un mois, par catégorie de comptes (chaque budget ET chaque
/// écriture comptés exactement une fois — jamais la somme des lignes,
/// qui double-compte les enfants sous leurs parents).
#[derive(Debug, Clone)]
pub struct ColumnTotal {
    /// Libellé de la catégorie (nom de type de compte).
    pub bucket: String,
    pub currency: String,
    pub budgeted: Decimal,
    pub actual: Decimal,
}

/// La grille complète d'une année.
#[derive(Debug, Clone)]
pub struct BudgetGrid {
    pub year: i16,
    /// Lignes triées par (compte, devise).
    pub rows: Vec<GridRow>,
    /// Totaux par mois (12 vecteurs, un par mois, triés comme `compare`).
    pub totals: Vec<Vec<ColumnTotal>>,
    /// Diagnostics du modèle (directives illisibles, comptes non ouverts…),
    /// dédupliqués sur l'année et triés.
    pub warnings: Vec<String>,
}

impl BudgetGrid {
    /// Construit la grille d'une année à partir des directives du ledger
    /// (toutes — le modèle filtre lui-même les `custom "budget"`).
    #[must_use]
    pub fn build(directives: &[Directive], types: &AccountTypes, year: i16) -> Self {
        let budgets = Budgets::from_directives(directives);

        let mut cells: BTreeMap<(String, String), [GridCell; 12]> = BTreeMap::new();
        let mut totals: Vec<Vec<ColumnTotal>> = vec![Vec::new(); 12];
        let mut warnings: BTreeSet<String> = BTreeSet::new();

        for month in 1..=12u8 {
            let from = month_start(year, month);
            let to = month_start_exclusive(year, month);
            let comparison = budgets.compare(directives, types, from, to, true, None);

            for row in &comparison.rows {
                let key = (
                    row.account.as_str().to_string(),
                    row.currency.as_str().to_string(),
                );
                let row_cells = cells
                    .entry(key)
                    .or_insert_with(|| [GridCell::default(); 12]);
                row_cells[usize::from(month) - 1] = GridCell {
                    budgeted: row.budgeted.unwrap_or_default(),
                    actual: row.actual.unwrap_or_default(),
                };
            }
            for total in &comparison.totals {
                totals[usize::from(month) - 1].push(ColumnTotal {
                    bucket: bucket_label(&total.bucket, types),
                    currency: total.currency.as_str().to_string(),
                    budgeted: total.budgeted.unwrap_or_default(),
                    actual: total.actual.unwrap_or_default(),
                });
            }
            for error in &comparison.errors {
                warnings.insert(format!(
                    "{} {} : {}",
                    error.date,
                    error.account.as_ref().map_or("?", Account::as_str),
                    error.reason
                ));
            }
        }

        Self {
            year,
            rows: cells
                .into_iter()
                .map(|((account, currency), row_cells)| GridRow {
                    account,
                    currency,
                    cells: row_cells,
                })
                .collect(),
            totals,
            warnings: warnings.into_iter().collect(),
        }
    }

    /// La ligne d'un poste (par nom exact), s'il est budgété.
    #[must_use]
    pub fn row(&self, account: &str) -> Option<&GridRow> {
        self.rows.iter().find(|r| r.account == account)
    }
}

/// Nom canonique de la catégorie (vocabulaire filaire de rustledger :
/// `assets`, `expenses`… ou racine personnalisée).
fn bucket_label(bucket: &Bucket, _types: &AccountTypes) -> String {
    match bucket {
        Bucket::Typed(kind) => kind.as_str().to_string(),
        Bucket::Other(root) => root.as_str().to_string(),
    }
}

/// 1er jour du mois (`year`, `month`).
#[must_use]
pub fn month_start(year: i16, month: u8) -> NaiveDate {
    rustledger_core::naive_date(i32::from(year), u32::from(month), 1).expect("mois valide 1..=12")
}

/// 1er jour du mois **suivant** (borne exclusive de la fenêtre).
#[must_use]
pub fn month_start_exclusive(year: i16, month: u8) -> NaiveDate {
    if month == 12 {
        month_start(year.checked_add(1).unwrap_or(year), 1)
    } else {
        month_start(year, month + 1)
    }
}

//! Rollover budgétaire : report cumulé du non-dépensé (ou du dépassement)
//! mois → mois suivant. **Non fourni par rustledger-budget** — calcul maison,
//! sémantique verrouillée par des tests (jalon 3).
//!
//! Sémantique adoptée (« cumul annuel + report vs réel cumulé ») :
//!
//! ```text
//! disponible(m) = report_initial + Σ_{1..=m} budget − Σ_{1..=m} réel
//! ```
//!
//! - Le report traverse l'année : un mois sous le budget augmente le
//!   disponible des mois suivants ; un dépassement le diminue (report
//!   **négatif** autorisé — c'est le choix « cumul », opposé au choix
//!   « plancher à zéro » qui masquerait les dérapages).
//! - `cumulative_budget` inclut le report initial (d'où son nom complet :
//!   budget cumulé **+ report**) pour que
//!   `disponible = cumulative_budget − cumulative_actual` reste l'unique
//!   formule.

use rust_decimal::Decimal;

use crate::grid::GridCell;

/// Séries cumulées d'un poste sur 12 mois.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloverSeries {
    /// Report entrant (fin d'année précédente), zéro par défaut.
    pub initial: Decimal,
    /// Budget cumulé **+ report initial**, mois par mois.
    pub cumulative_budget: [Decimal; 12],
    /// Réel cumulé, mois par mois.
    pub cumulative_actual: [Decimal; 12],
    /// Disponible à fin de mois = cumulative_budget − cumulative_actual.
    pub available: [Decimal; 12],
}

impl RolloverSeries {
    /// Calcule les séries cumulées d'une ligne de grille.
    #[must_use]
    pub fn from_cells(cells: &[GridCell; 12], initial: Decimal) -> Self {
        let mut cumulative_budget = [Decimal::ZERO; 12];
        let mut cumulative_actual = [Decimal::ZERO; 12];
        let mut available = [Decimal::ZERO; 12];
        let mut budget_run = initial;
        let mut actual_run = Decimal::ZERO;
        for (index, cell) in cells.iter().enumerate() {
            budget_run += cell.budgeted;
            actual_run += cell.actual;
            cumulative_budget[index] = budget_run;
            cumulative_actual[index] = actual_run;
            available[index] = budget_run - actual_run;
        }
        Self {
            initial,
            cumulative_budget,
            cumulative_actual,
            available,
        }
    }

    /// Le report sortant (disponible fin décembre) : l'`initial` de l'année
    /// suivante.
    #[must_use]
    pub fn carry_out(&self) -> Decimal {
        self.available[11]
    }
}

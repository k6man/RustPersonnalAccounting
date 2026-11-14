//! Écran Budget (TUI).

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use household_budget::BudgetGrid;
use rust_decimal::Decimal;
use rustledger_core::NaiveDate;

/// Action utilisateur retournée par l'écran budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionBudget {
    /// Aucune action.
    Aucun,
    /// Modifier une cellule budgétaire.
    Cellule {
        annee: i16,
        date: NaiveDate,
        compte: String,
        montant: Decimal,
    },
    /// Générer l'année cible par duplication.
    GenererAnnee {
        source: i16,
        cible: i16,
        pourcent: Decimal,
        pas: Decimal,
    },
    /// Basculer le rollover d'une famille.
    BasculerRollover(String),
}

/// État de l'écran budget (année sélectionnée).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub annee: i16,
}

impl UiState {
    pub fn nouveau(annee: i16) -> Self {
        Self { annee }
    }
}

/// Dessine l'écran budget.
pub fn afficher(
    f: &mut Frame,
    _grille: Option<&BudgetGrid>,
    _etat: &mut UiState,
    _rollover_actif: &dyn Fn(&str) -> bool,
) -> Option<ActionBudget> {
    let block = Block::default().title("Budget").borders(Borders::ALL);
    let texte = Paragraph::new("Budget — à implémenter en ratatui")
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(texte, f.size());
    None
}

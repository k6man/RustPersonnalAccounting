//! Écran Journal (stub TUI — à implémenter).

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::viewmodel::JournalData;

/// Action utilisateur retournée par l'écran journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionJournal {
    /// Aucune action.
    Aucun,
    /// Modifier la transaction à l'index donné.
    Modifier(usize),
    /// Supprimer la transaction à l'index donné.
    Supprimer(usize),
}

/// État de l'écran journal (filtres, sélection, pagination…).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub selection: Option<usize>,
}

impl UiState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Dessine le journal.
pub fn afficher(
    _f: &mut Frame,
    _donnees: &JournalData,
    _etat: &mut UiState,
) -> Option<ActionJournal> {
    let block = Block::default().title("Journal").borders(Borders::ALL);
    let texte = Paragraph::new("Journal — à implémenter en ratatui")
        .block(block)
        .style(Style::default().fg(Color::White));
    _f.render_widget(texte, _f.size());
    None
}

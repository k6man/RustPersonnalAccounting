//! Écran Git (stub TUI — à implémenter).

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::worker::GitDonnees;

/// Action utilisateur retournée par l'écran git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionGit {
    /// Aucune action.
    Aucun,
    /// Actualiser le statut.
    Actualiser,
    /// Commiter avec le message donné.
    Commit(String),
}

/// État de l'écran git (message de commit en cours d'édition).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub message: String,
}

impl UiState {
    pub fn nouveau() -> Self {
        Self::default()
    }
}

/// Dessine l'écran git.
pub fn afficher(_f: &mut Frame, _git: &GitDonnees, _etat: &mut UiState) -> Option<ActionGit> {
    let block = Block::default().title("Git").borders(Borders::ALL);
    let texte = Paragraph::new("Git — à implémenter en ratatui")
        .block(block)
        .style(Style::default().fg(Color::White));
    _f.render_widget(texte, _f.size());
    None
}

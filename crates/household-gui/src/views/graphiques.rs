//! Écran Graphiques (stub TUI — à implémenter).

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::viewmodel::UiData;

/// État de l'écran graphiques (mini-onglet + période).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub onglet: usize,
    pub annee: i16,
    pub mois: Option<u8>,
}

impl UiState {
    pub fn nouveau() -> Self {
        Self::default()
    }
}

/// Dessine l'écran graphiques.
pub fn afficher(_f: &mut Frame, _donnees: &UiData, _etat: &mut UiState) {
    let block = Block::default().title("Graphiques").borders(Borders::ALL);
    let texte = Paragraph::new("Graphiques — à implémenter en ratatui")
        .block(block)
        .style(Style::default().fg(Color::White));
    _f.render_widget(texte, _f.size());
}

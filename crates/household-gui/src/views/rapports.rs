//! Écran Rapports (stub TUI — à implémenter).

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::viewmodel::RapportsData;

/// Dessine l'écran rapports.
pub fn afficher(_f: &mut Frame, _donnees: &RapportsData) {
    let block = Block::default().title("Rapports").borders(Borders::ALL);
    let texte = Paragraph::new("Rapports — à implémenter en ratatui")
        .block(block)
        .style(Style::default().fg(Color::White));
    _f.render_widget(texte, _f.size());
}

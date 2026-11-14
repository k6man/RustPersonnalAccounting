//! Écran Tableau de bord (stub TUI — à implémenter).

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::viewmodel::DashboardData;

/// Action utilisateur retournée par l'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionDashboard {
    /// Aucune action.
    Aucun,
}

/// Dessine le tableau de bord.
pub fn afficher(f: &mut Frame, donnees: &DashboardData) -> Option<ActionDashboard> {
    let _ = donnees;
    let block = Block::default()
        .title("Tableau de bord")
        .borders(Borders::ALL);
    let texte = Paragraph::new("Tableau de bord — à implémenter en ratatui")
        .block(block)
        .style(Style::default().fg(Color::White));
    f.render_widget(texte, f.size());
    None
}

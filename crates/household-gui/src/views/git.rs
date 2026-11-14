//! Écran Git (TUI).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
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
pub fn afficher(f: &mut Frame, git: &GitDonnees, etat: &mut UiState) -> Option<ActionGit> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // résumé
            Constraint::Min(0),    // fichiers
        ])
        .split(f.size());

    // Résumé du diff.
    let resume = &git.resume;
    let resume_texte = Paragraph::new(format!(
        " Fichiers : {} | Insertions : {} | Suppressions : {}",
        resume.fichiers, resume.insertions, resume.suppressions
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Diff HEAD → Index "),
    )
    .style(Style::default().fg(Color::White));
    f.render_widget(resume_texte, chunks[0]);

    // Tableau des fichiers modifiés.
    let rows: Vec<Row> = git
        .fichiers
        .iter()
        .map(|fichier| {
            let etats: Vec<String> = fichier
                .etats
                .iter()
                .map(|e| match e {
                    household_git::EtatFichier::Nouveau => "Nouveau".to_string(),
                    household_git::EtatFichier::Modifie => "Modifié".to_string(),
                    household_git::EtatFichier::Supprime => "Supprimé".to_string(),
                    household_git::EtatFichier::Renomme => "Renommé".to_string(),
                })
                .collect();
            Row::new(vec![fichier.chemin.clone(), etats.join(", ")])
        })
        .collect();
    let tableau = Table::new(rows, [Constraint::Length(40), Constraint::Length(20)])
        .header(
            Row::new(vec!["Fichier", "État(s)"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Fichiers modifiés "),
        )
        .column_spacing(1);
    f.render_widget(tableau, chunks[1]);

    None
}

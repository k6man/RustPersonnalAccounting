//! Écran Git (TUI).
//!
//! Affiche le statut des fichiers modifiés, le résumé du diff, l'historique
//! des commits filtré sur le ledger, et permet de commiter avec un message
//! prérempli (suggéré automatiquement). La validation du ledger est faite
//! côté app avant le commit (ADR-002 : git2 n'exécute pas les hooks).

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::app::ActionClique;
use crate::widgets::text_input::TextInput;
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

/// État de l'écran git (message de commit en cours d'édition, champ actif).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    /// Message de commit en cours d'édition.
    pub message: TextInput,
    /// Champ actuellement sélectionné (message ou bouton commit).
    pub champ_actif: ChampGit,
}

/// Champ actuellement sélectionné dans l'écran Git.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChampGit {
    #[default]
    Message,
    Commit,
}

impl ChampGit {
    pub fn suivant(self) -> Self {
        match self {
            Self::Message => Self::Commit,
            Self::Commit => Self::Message,
        }
    }

    pub fn precedent(self) -> Self {
        self.suivant()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Message => "Message de commit",
            Self::Commit => "Commiter",
        }
    }
}

impl UiState {
    pub fn nouveau() -> Self {
        Self {
            message: TextInput::new().label("Message de commit").max_width(80),
            champ_actif: ChampGit::Message,
        }
    }

    /// Définit le message suggéré dans le champ d'édition.
    pub fn set_message_suggere(&mut self, suggestion: &str) {
        self.message = TextInput::with_value(suggestion.to_string())
            .label("Message de commit")
            .max_width(80);
        self.message.cursor = self.message.value.len();
    }
}

/// Dessine l'écran git.
pub fn afficher(
    f: &mut Frame,
    git: &GitDonnees,
    etat: &mut UiState,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) -> Option<ActionGit> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // résumé du diff
            Constraint::Length(6), // message de commit
            Constraint::Length(3), // bouton commit
            Constraint::Min(0),    // historique
        ])
        .split(f.size());

    // 1. Résumé du diff.
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

    // 2. Zone de saisie du message de commit.
    let style_message = if etat.champ_actif == ChampGit::Message {
        Style::default().bg(Color::Blue).fg(Color::White)
    } else {
        Style::default()
    };
    let message_block = Block::default().borders(Borders::ALL).title(format!(
        " {} (valeur suggérée : {}) ",
        etat.champ_actif.label(),
        git.message_suggere
    ));
    let message_paragraph = Paragraph::new(etat.message.value.as_str())
        .block(message_block)
        .style(style_message);
    f.render_widget(message_paragraph, chunks[1]);

    // 3. Bouton commit.
    let commit_disponible = resume.fichiers > 0;
    let commit_style = if etat.champ_actif == ChampGit::Commit && commit_disponible {
        Style::default()
            .bg(Color::Green)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else if !commit_disponible {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    let bouton_commit = Paragraph::new(format!(
        " [ {} ] ",
        if commit_disponible {
            "Commiter"
        } else {
            "Rien à commiter"
        }
    ))
    .block(Block::default().borders(Borders::ALL).title(" Action "))
    .style(commit_style);
    f.render_widget(bouton_commit, chunks[2]);

    // 4. Historique des commits (filtré sur le ledger).
    if git.disponible && !git.historique.is_empty() {
        let rows: Vec<Row> = git
            .historique
            .iter()
            .take(15)
            .map(|c| {
                Row::new(vec![
                    c.oid.clone(),
                    c.date.clone(),
                    c.auteur.clone(),
                    c.sujet.clone(),
                ])
            })
            .collect();
        let tableau = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Length(16),
                Constraint::Length(20),
                Constraint::Length(30),
            ],
        )
        .header(
            Row::new(vec!["OID", "Date", "Auteur", "Sujet"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Historique (15 derniers) "),
        )
        .column_spacing(1);
        f.render_widget(tableau, chunks[3]);

        // Enregistrer les zones cliquables des lignes de l'historique.
        for (i, _commit) in git.historique.iter().enumerate().take(15) {
            let ligne_y = chunks[3].top() + 1 + i as u16;
            if ligne_y >= chunks[3].bottom() {
                break;
            }
            zones_cliquables.push((
                Rect::new(chunks[3].left() + 1, ligne_y, chunks[3].width - 2, 1),
                ActionClique::GraphiquesOnglet(i),
            ));
        }
    } else if git.disponible {
        let vide = Paragraph::new("Aucun commit dans l'historique du ledger.")
            .block(Block::default().borders(Borders::ALL).title(" Historique "))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(vide, chunks[3]);
    }

    None
}

/// Gère un événement clavier pour le Git.
pub fn on_key(key: KeyEvent, etat: &mut UiState) -> Option<ActionGit> {
    use crossterm::event::KeyCode;

    // Délégation au TextInput si le champ message est actif.
    if etat.champ_actif == ChampGit::Message && etat.message.on_key(key) {
        return None;
    }

    match key.code {
        KeyCode::Tab => {
            etat.champ_actif = etat.champ_actif.suivant();
            None
        }
        KeyCode::Char('t') => {
            etat.champ_actif = etat.champ_actif.precedent();
            None
        }
        KeyCode::Enter => match etat.champ_actif {
            ChampGit::Commit => {
                let resume_fichiers = etat.message.value.trim().to_string();
                if !resume_fichiers.is_empty() {
                    Some(ActionGit::Commit(resume_fichiers))
                } else {
                    None
                }
            }
            ChampGit::Message => {
                etat.champ_actif = ChampGit::Commit;
                None
            }
        },
        KeyCode::Char('r') => Some(ActionGit::Actualiser),
        KeyCode::Esc => None,
        _ => None,
    }
}

/// Gère un événement souris pour le Git.
///
/// Retourne une `ActionGit` si l'utilisateur a cliqué sur le bouton de commit
/// ou sur une ligne de l'historique, `None` sinon.
pub fn on_mouse(mouse: crossterm::event::MouseEvent, etat: &mut UiState) -> Option<ActionGit> {
    use crossterm::event::MouseEventKind;

    if mouse.kind != MouseEventKind::Down(crossterm::event::MouseButton::Left) {
        return None;
    }

    // Clic sur le bouton commit (y = 5 environ, après résumé et message).
    if mouse.row == 6 && mouse.column > 2 && mouse.column < 20 {
        return Some(ActionGit::Commit(etat.message.value.trim().to_string()));
    }

    // Clic sur une ligne de l'historique (y >= 10 environ).
    if mouse.row >= 10 && !git_disponible_is_empty(mouse) {
        let ligne_cliquee = (mouse.row as usize) - 10;
        // On ne fait rien de particulier pour l'instant, juste sélectionner
        // la ligne. Le historique est affiché en lecture seule.
        let _ = ligne_cliquee;
    }

    None
}

/// Vérifie s'il y a de l'historique à afficher.
fn git_disponible_is_empty(_mouse: crossterm::event::MouseEvent) -> bool {
    // Cette fonction est un placeholder ; la logique réelle vérifierait
    // si git.historique est vide. Pour l'instant, on retourne toujours false
    // pour permettre le clic sur les lignes.
    false
}

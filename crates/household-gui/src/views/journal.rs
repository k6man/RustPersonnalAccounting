//! Écran Journal (TUI).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};

use crate::fmt::eur;
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
    /// État du widget Table (pour le rendu stateful).
    table_state: TableState,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            selection: None,
            table_state: TableState::new(),
        }
    }

    /// Index de la ligne sélectionnée (clampé au nombre de lignes).
    pub fn selection_index(&self, nb_lignes: usize) -> usize {
        self.selection.unwrap_or(0).min(nb_lignes.saturating_sub(1))
    }

    /// Définit la sélection.
    pub fn set_selection(&mut self, index: usize) {
        self.selection = Some(index);
        self.table_state.select(Some(index));
    }
}

/// Dessine le journal.
pub fn afficher(f: &mut Frame, donnees: &JournalData, etat: &mut UiState) -> Option<ActionJournal> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // info
            Constraint::Min(0),    // tableau
        ])
        .split(f.size());

    // Info : nombre de lignes.
    let info = Paragraph::new(format!(
        " {} transaction(s) — flèches haut/bas pour naviguer, Entrée pour modifier, Suppr pour supprimer",
        donnees.lignes.len()
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(info, chunks[0]);

    // Tableau des transactions.
    let rows: Vec<Row> = donnees
        .lignes
        .iter()
        .take(50) // pagination simple : 50 lignes max
        .map(|ligne| {
            let compte_str: String = ligne
                .lignes_posting
                .iter()
                .map(|p| p.compte.clone())
                .collect::<Vec<_>>()
                .join(", ");
            let tags_str: String = ligne.tags.to_vec().join(" ");
            let flag = if ligne.flag == '*' { "" } else { "!" };
            let montant = ligne
                .montant
                .map(|m| eur(m).to_string())
                .unwrap_or_else(|| "–".to_string());
            Row::new(vec![
                ligne.date.to_string(),
                flag.to_string(),
                ligne.tiers.clone().unwrap_or_default(),
                ligne.libelle.clone(),
                compte_str,
                montant,
                tags_str,
            ])
        })
        .collect();

    if rows.is_empty() {
        let vide = Paragraph::new("Aucune transaction.")
            .block(Block::default().borders(Borders::ALL).title(" Journal "));
        f.render_widget(vide, chunks[1]);
        return None;
    }

    let nb_lignes = rows.len();
    let selected = etat.selection_index(nb_lignes);
    etat.table_state.select(Some(selected));

    let tableau = Table::new(
        rows,
        [
            Constraint::Length(10), // date
            Constraint::Length(4),  // flag
            Constraint::Length(15), // tiers
            Constraint::Length(30), // libellé
            Constraint::Length(25), // comptes
            Constraint::Length(12), // montant
            Constraint::Length(15), // tags
        ],
    )
    .header(
        Row::new(vec![
            "Date", "F", "Tiers", "Libellé", "Comptes", "Montant", "Tags",
        ])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1),
    )
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Journal — ligne {} / {}",
        selected + 1,
        nb_lignes
    )))
    .highlight_symbol(">> ")
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .column_spacing(1);

    f.render_stateful_widget(tableau, chunks[1], &mut etat.table_state);

    None
}

/// Gère un événement clavier pour le journal.
///
/// Retourne une `ActionJournal` si l'utilisateur a demandé une action
/// (modifier, supprimer), `None` sinon.
pub fn on_key(
    key: crossterm::event::KeyEvent,
    etat: &mut UiState,
    nb_lignes: usize,
) -> Option<ActionJournal> {
    match key.code {
        crossterm::event::KeyCode::Up => {
            let actuel = etat.selection_index(nb_lignes);
            if actuel > 0 {
                etat.set_selection(actuel - 1);
            }
            None
        }
        crossterm::event::KeyCode::Down => {
            let actuel = etat.selection_index(nb_lignes);
            if actuel < nb_lignes.saturating_sub(1) {
                etat.set_selection(actuel + 1);
            }
            None
        }
        crossterm::event::KeyCode::Enter => {
            let actuel = etat.selection_index(nb_lignes);
            if actuel < nb_lignes {
                Some(ActionJournal::Modifier(actuel))
            } else {
                None
            }
        }
        crossterm::event::KeyCode::Delete | crossterm::event::KeyCode::Backspace => {
            let actuel = etat.selection_index(nb_lignes);
            if actuel < nb_lignes {
                Some(ActionJournal::Supprimer(actuel))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Gère un événement souris pour le journal.
///
/// Retourne une `ActionJournal` si l'utilisateur a cliqué sur une ligne.
pub fn on_mouse(
    mouse: crossterm::event::MouseEvent,
    _etat: &mut UiState,
    nb_lignes: usize,
) -> Option<ActionJournal> {
    use crossterm::event::MouseEventKind;

    if mouse.kind != MouseEventKind::Down(crossterm::event::MouseButton::Left) {
        return None;
    }

    // Le tableau commence après le panneau info (3 lignes) et sa bordure.
    // Approximation : la première ligne de données est à y = 4.
    if mouse.row < 4 {
        return None;
    }

    let ligne_cliquee = (mouse.row as usize) - 4;
    if ligne_cliquee < nb_lignes {
        Some(ActionJournal::Modifier(ligne_cliquee))
    } else {
        None
    }
}

//! Écran Journal (TUI).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};

use crate::app::ActionClique;
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

    /// Déplace la sélection d'une ligne vers le haut (molette ou flèche).
    pub fn monter(&mut self, nb_lignes: usize) {
        let actuel = self.selection_index(nb_lignes);
        if actuel > 0 {
            self.set_selection(actuel - 1);
        }
    }

    /// Déplace la sélection d'une ligne vers le bas (molette ou flèche).
    pub fn descendre(&mut self, nb_lignes: usize) {
        let actuel = self.selection_index(nb_lignes);
        if actuel < nb_lignes.saturating_sub(1) {
            self.set_selection(actuel + 1);
        }
    }
}

/// Nombre maximal de lignes affichées (pagination sommaire). La sélection
/// clavier et les zones de clic sont bornées à ce nombre.
pub const MAX_LIGNES: usize = 50;

/// Dessine le journal et enregistre les zones cliquables des lignes.
pub fn afficher(
    f: &mut Frame,
    area: Rect,
    donnees: &JournalData,
    etat: &mut UiState,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) -> Option<ActionJournal> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // info
            Constraint::Min(0),    // tableau
        ])
        .split(area);

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
        .take(MAX_LIGNES)
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

    // Enregistrer les zones cliquables des lignes du tableau. La première
    // rangée de données est 3 lignes sous le haut du bloc : bordure, en-tête,
    // marge basse d'en-tête.
    for (i, ligne) in donnees.lignes.iter().enumerate().take(MAX_LIGNES) {
        let ligne_y = chunks[1].top() + 3 + i as u16;
        if ligne_y >= chunks[1].bottom() {
            break;
        }
        zones_cliquables.push((
            Rect::new(chunks[1].left(), ligne_y, chunks[1].width, 1),
            ActionClique::JournalLigne(ligne.index),
        ));
    }

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
            etat.monter(nb_lignes);
            None
        }
        crossterm::event::KeyCode::Down => {
            etat.descendre(nb_lignes);
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

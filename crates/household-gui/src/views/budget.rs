//! Écran Budget (TUI) : grille postes × 12 mois.
//!
//! Affiche les lignes budgétaires avec budget et réel par mois, les totaux
//! par ligne (année) et les écarts. Les cellules de budget sont éditables
//! (navigation avec Tab, saisie directe, entrée pour valider).

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use household_budget::BudgetGrid;
use rust_decimal::Decimal;
use rustledger_core::NaiveDate;

use crate::fmt::eur;

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

/// État de l'écran budget (année sélectionnée, cellule active).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub annee: i16,
    /// Ligne sélectionnée dans la grille.
    pub ligne_selectionnee: usize,
    /// Mois sélectionné (0..=11).
    pub mois_selectionne: usize,
}

impl UiState {
    pub fn nouveau(annee: i16) -> Self {
        Self {
            annee,
            ligne_selectionnee: 0,
            mois_selectionne: 0,
        }
    }

    /// Nombre de lignes de la grille.
    pub fn nb_lignes(&self, grille: Option<&BudgetGrid>) -> usize {
        grille.map_or(0, |g| g.rows.len())
    }

    /// Index de ligne sélectionné clampé.
    pub fn ligne_index(&self, grille: Option<&BudgetGrid>) -> usize {
        let nb = self.nb_lignes(grille);
        if nb == 0 {
            return 0;
        }
        self.ligne_selectionnee.min(nb - 1)
    }

    /// Passe à la ligne suivante.
    pub fn ligne_suivante(&mut self, grille: Option<&BudgetGrid>) {
        let nb = self.nb_lignes(grille);
        if nb == 0 {
            return;
        }
        self.ligne_selectionnee = (self.ligne_selectionnee + 1).min(nb - 1);
    }

    /// Passe à la ligne précédente.
    pub fn ligne_precedente(&mut self) {
        if self.ligne_selectionnee > 0 {
            self.ligne_selectionnee -= 1;
        }
    }

    /// Passe au mois suivant.
    pub fn mois_suivant(&mut self) {
        self.mois_selectionne = (self.mois_selectionne + 1).min(11);
    }

    /// Passe au mois précédent.
    pub fn mois_precedent(&mut self) {
        if self.mois_selectionne > 0 {
            self.mois_selectionne -= 1;
        }
    }
}

/// Dessine l'écran budget.
pub fn afficher(
    f: &mut Frame,
    grille: Option<&BudgetGrid>,
    etat: &mut UiState,
    _rollover_actif: &dyn Fn(&str) -> bool,
) -> Option<ActionBudget> {
    let grille = grille?;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // titre avec année
            Constraint::Length(1), // aide
            Constraint::Min(0),    // tableau
        ])
        .split(f.size());

    // 1. Titre.
    let titre = Paragraph::new(format!(
        " Budget {}  (flèches haut/bas pour naviguer, gauche/droite pour le mois, Entrée pour éditer)",
        etat.annee
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Budget {} ", etat.annee)),
    )
    .style(Style::default().fg(Color::White));
    f.render_widget(titre, chunks[0]);

    // 2. Aide rapide.
    let aide = Paragraph::new(
        " + : ajouter une ligne  |  × : supprimer  |  e : éditer la cellule  |  g : générer l'année suivante",
    )
    .block(Block::default().borders(Borders::NONE))
    .style(Style::default().fg(Color::Gray));
    f.render_widget(aide, chunks[1]);

    // 3. Tableau budget vs réel.
    let lignes = &grille.rows;
    if lignes.is_empty() {
        let vide = Paragraph::new("Aucune ligne budgétaire pour cette année.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Budget — lignes "),
            )
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(vide, chunks[2]);
        return None;
    }

    let ligne_idx = etat.ligne_index(Some(grille));
    let mois_idx = etat.mois_selectionne;

    // En-têtes de colonnes : Poste | Jan | Fév | ... | Déc | Total | Écart
    let mois_noms = [
        "Jan", "Fév", "Mar", "Avr", "Mai", "Juin", "Jui", "Aoû", "Sep", "Oct", "Nov", "Déc",
    ];
    let header_cells: Vec<String> = std::iter::once("Poste".to_string())
        .chain(mois_noms.iter().map(|m| m.to_string()))
        .chain(["Total", "Écart"].iter().map(|s| s.to_string()))
        .collect();

    let rows: Vec<Row> = lignes
        .iter()
        .enumerate()
        .map(|(i, ligne)| {
            let selected = i == ligne_idx;
            let style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let mut cells: Vec<String> = vec![ligne.account.clone()];
            let mut total_budget = Decimal::ZERO;
            let mut total_reel = Decimal::ZERO;
            for m in 0..12 {
                let cell = &ligne.cells[m];
                total_budget += cell.budgeted;
                total_reel += cell.actual;
                cells.push(format!("{:>10}", eur(cell.budgeted)));
            }
            let ecart = total_budget - total_reel;
            cells.push(format!("{:>10}", eur(total_budget)));
            cells.push(format!("{:>10}", eur(ecart)));
            Row::new(cells).style(style)
        })
        .collect();

    let widths: Vec<Constraint> = std::iter::once(Constraint::Length(28))
        .chain(std::iter::repeat_n(Constraint::Length(10), 12))
        .chain([Constraint::Length(10), Constraint::Length(10)])
        .collect();

    let tableau = Table::new(rows, widths)
        .header(
            Row::new(header_cells)
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Budget vs réel — mois {:02} (cellule {}) ",
            mois_idx + 1,
            mois_noms[mois_idx]
        )))
        .column_spacing(1);

    f.render_widget(tableau, chunks[2]);

    // Afficher la cellule sélectionnée en bas (pas de widget séparé, juste dans le titre)
    let _ = mois_idx;

    None
}

/// Gère un événement clavier pour le budget.
pub fn on_key(
    key: KeyEvent,
    etat: &mut UiState,
    grille: Option<&BudgetGrid>,
    _config: &dyn Fn(&str) -> bool,
    _cmd_tx: &std::sync::mpsc::Sender<crate::worker::Command>,
) -> Option<ActionBudget> {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Up => {
            etat.ligne_precedente();
            None
        }
        KeyCode::Down => {
            etat.ligne_suivante(grille);
            None
        }
        KeyCode::Left => {
            etat.mois_precedent();
            None
        }
        KeyCode::Right => {
            etat.mois_suivant();
            None
        }
        KeyCode::Char('e') => {
            let grille = grille?;
            let ligne_idx = etat.ligne_index(Some(grille));
            let ligne = &grille.rows[ligne_idx];
            let mois = etat.mois_selectionne;
            let cell = &ligne.cells[mois];
            let date =
                rustledger_core::NaiveDate::new(etat.annee, i8::try_from(mois + 1).unwrap_or(1), 1)
                    .ok()?;
            Some(ActionBudget::Cellule {
                annee: etat.annee,
                date,
                compte: ligne.account.clone(),
                montant: cell.budgeted,
            })
        }
        KeyCode::Char('g') => {
            // Générer l'année suivante (placeholder — nécessite une UI de dialogue).
            None
        }
        _ => None,
    }
}

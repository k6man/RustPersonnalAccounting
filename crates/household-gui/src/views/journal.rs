//! Écran Journal (TUI) : liste des transactions avec filtres et pagination.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};

use crate::app::ActionClique;
use crate::filters::{JournalFilter, Pagination};
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
    /// Retour au tableau de bord (Esc).
    RetourDashboard,
}

/// État de l'écran journal (filtres, sélection, pagination…).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub selection: Option<usize>,
    /// Filtres actifs.
    pub filtre: JournalFilter,
    /// Pagination (100 lignes par page).
    pub pagination: Pagination,
    /// État du widget Table (pour le rendu stateful).
    table_state: TableState,
    /// Champ de filtre en cours d'édition (None = pas en édition).
    pub champ_filtre_actif: Option<ChampFiltre>,
}

/// Champ de filtre en cours d'édition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChampFiltre {
    Texte,
    Compte,
    Tag,
    DateMin,
    DateMax,
    MontantMin,
    MontantMax,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            selection: None,
            filtre: JournalFilter::default(),
            pagination: Pagination {
                page: 0,
                par_page: 100,
            },
            table_state: TableState::new(),
            champ_filtre_actif: None,
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

/// Dessine le journal et enregistre les zones cliquables des lignes.
pub fn afficher(
    f: &mut Frame,
    area: Rect,
    donnees: &JournalData,
    etat: &mut UiState,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) -> Option<ActionJournal> {
    // Appliquer les filtres.
    let lignes_filtrees: Vec<_> = donnees
        .lignes
        .iter()
        .filter(|l| etat.filtre.correspond(l))
        .collect();

    // Pagination.
    etat.pagination.borner(lignes_filtrees.len());
    let (debut, fin) = etat.pagination.tranche(lignes_filtrees.len());
    let lignes_page = &lignes_filtrees[debut..fin];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // filtres
            Constraint::Length(3), // info
            Constraint::Min(0),    // tableau
        ])
        .split(area);

    // 1. Barre de filtres.
    let filtre_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // texte
            Constraint::Percentage(15), // compte
            Constraint::Percentage(10), // tag
            Constraint::Percentage(15), // date min
            Constraint::Percentage(15), // date max
            Constraint::Percentage(10), // montant min
            Constraint::Percentage(10), // montant max
        ])
        .split(chunks[0]);

    let filtres = [
        ("Texte", &etat.filtre.texte, ChampFiltre::Texte),
        ("Compte", &etat.filtre.compte, ChampFiltre::Compte),
        ("Tag", &etat.filtre.tag, ChampFiltre::Tag),
        (
            "Date min",
            &etat
                .filtre
                .date_min
                .map(|d| d.to_string())
                .unwrap_or_default(),
            ChampFiltre::DateMin,
        ),
        (
            "Date max",
            &etat
                .filtre
                .date_max
                .map(|d| d.to_string())
                .unwrap_or_default(),
            ChampFiltre::DateMax,
        ),
        (
            "Montant min",
            &etat
                .filtre
                .montant_min
                .map(|m| m.to_string())
                .unwrap_or_default(),
            ChampFiltre::MontantMin,
        ),
        (
            "Montant max",
            &etat
                .filtre
                .montant_max
                .map(|m| m.to_string())
                .unwrap_or_default(),
            ChampFiltre::MontantMax,
        ),
    ];

    for (i, (label, value, champ)) in filtres.iter().enumerate() {
        let is_active = etat.champ_filtre_actif == Some(*champ);
        let style = if is_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let texte = format!(" {}: {}", label, value);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(if is_active { " ▼ " } else { "" })
            .style(style);
        let para = Paragraph::new(texte).block(block).style(style);
        f.render_widget(para, filtre_chunks[i]);

        // Zone cliquable pour activer l'édition du champ.
        zones_cliquables.push((filtre_chunks[i], ActionClique::JournalFiltre(*champ)));
    }

    // Indicateur de filtre actif.
    let filtre_actif = if etat.filtre.actif() {
        " [FILTRE ACTIF]"
    } else {
        ""
    };
    let info_texte = format!(
        " {} transaction(s) ({} affichées, page {}/{}){} — / pour filtrer, Tab navigue, Entrée modifie, Suppr supprime",
        lignes_filtrees.len(),
        lignes_page.len(),
        etat.pagination.page + 1,
        etat.pagination.pages(lignes_filtrees.len()),
        filtre_actif
    );
    let info = Paragraph::new(info_texte).block(Block::default().borders(Borders::ALL));
    f.render_widget(info, chunks[1]);

    // 2. Tableau des transactions (page courante).
    let rows: Vec<Row> = lignes_page
        .iter()
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
        let vide = Paragraph::new("Aucune transaction (filtre trop restrictif ?).")
            .block(Block::default().borders(Borders::ALL).title(" Journal "));
        f.render_widget(vide, chunks[2]);
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
        " Journal — ligne {} / {} (page {}/{}) ",
        selected + 1,
        nb_lignes,
        etat.pagination.page + 1,
        etat.pagination.pages(lignes_filtrees.len())
    )))
    .highlight_symbol(">> ")
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .column_spacing(1);

    f.render_stateful_widget(tableau, chunks[2], &mut etat.table_state);

    // Enregistrer les zones cliquables des lignes du tableau.
    for (i, ligne) in lignes_page.iter().enumerate() {
        let ligne_y = chunks[2].top() + 3 + i as u16;
        if ligne_y >= chunks[2].bottom() {
            break;
        }
        zones_cliquables.push((
            Rect::new(chunks[2].left(), ligne_y, chunks[2].width, 1),
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
    lignes_filtrees: &[&crate::viewmodel::LigneJournal],
) -> Option<ActionJournal> {
    let nb_lignes = lignes_filtrees.len();
    let (debut, fin) = etat.pagination.tranche(nb_lignes);
    let nb_lignes_page = fin - debut;

    // Si un champ de filtre est en édition, on capture toutes les touches pour ce champ.
    if let Some(champ) = etat.champ_filtre_actif {
        return match key.code {
            crossterm::event::KeyCode::Esc => {
                etat.champ_filtre_actif = None;
                None
            }
            crossterm::event::KeyCode::Tab => {
                // Tab passe au champ suivant.
                let champs = [
                    ChampFiltre::Texte,
                    ChampFiltre::Compte,
                    ChampFiltre::Tag,
                    ChampFiltre::DateMin,
                    ChampFiltre::DateMax,
                    ChampFiltre::MontantMin,
                    ChampFiltre::MontantMax,
                ];
                if let Some(pos) = champs.iter().position(|&c| c == champ) {
                    let next = champs[(pos + 1) % champs.len()];
                    etat.champ_filtre_actif = Some(next);
                }
                None
            }
            crossterm::event::KeyCode::BackTab => {
                // Shift+Tab passe au champ précédent.
                let champs = [
                    ChampFiltre::Texte,
                    ChampFiltre::Compte,
                    ChampFiltre::Tag,
                    ChampFiltre::DateMin,
                    ChampFiltre::DateMax,
                    ChampFiltre::MontantMin,
                    ChampFiltre::MontantMax,
                ];
                if let Some(pos) = champs.iter().position(|&c| c == champ) {
                    let prev = champs[(pos + champs.len() - 1) % champs.len()];
                    etat.champ_filtre_actif = Some(prev);
                }
                None
            }
            crossterm::event::KeyCode::Enter => {
                // Entrée valide et sort de l'édition.
                etat.champ_filtre_actif = None;
                // Reborner la pagination après changement de filtre.
                etat.pagination.borner(nb_lignes);
                None
            }
            crossterm::event::KeyCode::Char(c) => {
                // Ajouter le caractère au champ actif.
                match champ {
                    ChampFiltre::Texte => etat.filtre.texte.push(c),
                    ChampFiltre::Compte => etat.filtre.compte.push(c),
                    ChampFiltre::Tag => etat.filtre.tag.push(c),
                    ChampFiltre::DateMin => {
                        etat.filtre.date_min = crate::filters::date_optionnelle(
                            &(etat
                                .filtre
                                .date_min
                                .map_or(String::new(), |d| d.to_string())
                                + &c.to_string()),
                        )
                    }
                    ChampFiltre::DateMax => {
                        etat.filtre.date_max = crate::filters::date_optionnelle(
                            &(etat
                                .filtre
                                .date_max
                                .map_or(String::new(), |d| d.to_string())
                                + &c.to_string()),
                        )
                    }
                    ChampFiltre::MontantMin => {
                        etat.filtre.montant_min = crate::filters::montant_optionnel(
                            &(etat
                                .filtre
                                .montant_min
                                .map_or(String::new(), |m| m.to_string())
                                + &c.to_string()),
                        )
                    }
                    ChampFiltre::MontantMax => {
                        etat.filtre.montant_max = crate::filters::montant_optionnel(
                            &(etat
                                .filtre
                                .montant_max
                                .map_or(String::new(), |m| m.to_string())
                                + &c.to_string()),
                        )
                    }
                }
                None
            }
            crossterm::event::KeyCode::Backspace => {
                // Supprimer le dernier caractère du champ actif.
                match champ {
                    ChampFiltre::Texte => {
                        etat.filtre.texte.pop();
                    }
                    ChampFiltre::Compte => {
                        etat.filtre.compte.pop();
                    }
                    ChampFiltre::Tag => {
                        etat.filtre.tag.pop();
                    }
                    ChampFiltre::DateMin => {
                        let mut s = etat
                            .filtre
                            .date_min
                            .map_or(String::new(), |d| d.to_string());
                        s.pop();
                        etat.filtre.date_min = crate::filters::date_optionnelle(&s);
                    }
                    ChampFiltre::DateMax => {
                        let mut s = etat
                            .filtre
                            .date_max
                            .map_or(String::new(), |d| d.to_string());
                        s.pop();
                        etat.filtre.date_max = crate::filters::date_optionnelle(&s);
                    }
                    ChampFiltre::MontantMin => {
                        let mut s = etat
                            .filtre
                            .montant_min
                            .map_or(String::new(), |m| m.to_string());
                        s.pop();
                        etat.filtre.montant_min = crate::filters::montant_optionnel(&s);
                    }
                    ChampFiltre::MontantMax => {
                        let mut s = etat
                            .filtre
                            .montant_max
                            .map_or(String::new(), |m| m.to_string());
                        s.pop();
                        etat.filtre.montant_max = crate::filters::montant_optionnel(&s);
                    }
                }
                None
            }
            _ => None,
        };
    }

    // Pas de champ en édition : raccourcis globaux du journal.
    match key.code {
        crossterm::event::KeyCode::Char('/') => {
            // '/' active l'édition du premier champ (texte).
            etat.champ_filtre_actif = Some(ChampFiltre::Texte);
            None
        }
        crossterm::event::KeyCode::Char('c')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            // Ctrl+C est géré plus haut dans App::on_key.
            None
        }
        crossterm::event::KeyCode::Char('r') => {
            // 'r' réinitialise les filtres.
            etat.filtre = JournalFilter::default();
            etat.pagination.page = 0;
            etat.pagination.borner(nb_lignes);
            None
        }
        crossterm::event::KeyCode::Esc => {
            // Esc = retour au tableau de bord (si pas dans un champ de filtre).
            Some(ActionJournal::RetourDashboard)
        }
        crossterm::event::KeyCode::PageUp => {
            if etat.pagination.page > 0 {
                etat.pagination.page -= 1;
                etat.selection = Some(debut.saturating_sub(1).min(nb_lignes.saturating_sub(1)));
            }
            None
        }
        crossterm::event::KeyCode::PageDown => {
            let total_pages = etat.pagination.pages(nb_lignes);
            if etat.pagination.page + 1 < total_pages {
                etat.pagination.page += 1;
                let (new_debut, _) = etat.pagination.tranche(nb_lignes);
                etat.selection = Some(new_debut.min(nb_lignes.saturating_sub(1)));
            }
            None
        }
        crossterm::event::KeyCode::Up => {
            etat.monter(nb_lignes_page);
            None
        }
        crossterm::event::KeyCode::Down => {
            etat.descendre(nb_lignes_page);
            None
        }
        crossterm::event::KeyCode::Enter => {
            let actuel = etat.selection_index(nb_lignes_page);
            if actuel < nb_lignes_page {
                // Convertir l'index de page en index global dans lignes_filtrees.
                let index_global = debut + actuel;
                Some(ActionJournal::Modifier(index_global))
            } else {
                None
            }
        }
        crossterm::event::KeyCode::Delete | crossterm::event::KeyCode::Backspace => {
            let actuel = etat.selection_index(nb_lignes_page);
            if actuel < nb_lignes_page {
                let index_global = debut + actuel;
                Some(ActionJournal::Supprimer(index_global))
            } else {
                None
            }
        }
        _ => None,
    }
}

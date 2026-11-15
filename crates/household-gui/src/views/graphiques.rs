//! Écran Graphiques (TUI).
//!
//! Trois mini-onglets :
//! 1. Répartition des dépenses (top 8 + « Autres ») — bascule année/mois
//! 2. Cascade annuelle (identité comptable : Revenus − Dépenses = Δ patrimoine)
//! 3. Budget vs réel du mois (barres groupées)

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};

use crate::app::ActionClique;
use crate::fmt::eur;
use crate::viewmodel::{GraphesData, UiData};

use rust_decimal::Decimal;

/// Action utilisateur retournée par l'écran graphiques.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionGraphiques {
    /// Aucune action.
    Aucun,
    /// Changer le mini-onglet actif (0 = répartition, 1 = cascade, 2 = budget vs réel).
    ChangerOnglet(usize),
    /// Bascule année/mois pour la répartition.
    BasculeRepartition,
}

/// État de l'écran graphiques (mini-onglet + période).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub onglet: usize,
    pub annee: i16,
    pub mois: Option<u8>,
    /// Pour la répartition : bascule année (false) / mois (true).
    pub repartition_mois: bool,
}

impl UiState {
    pub fn nouveau() -> Self {
        Self::default()
    }

    /// Passe au mini-onglet suivant.
    pub fn onglet_suivant(&mut self) {
        self.onglet = (self.onglet + 1).min(2);
    }

    /// Passe au mini-onglet précédent.
    pub fn onglet_precedent(&mut self) {
        if self.onglet > 0 {
            self.onglet -= 1;
        }
    }

    /// Bascule année/mois pour la répartition.
    pub fn bascule_repartition(&mut self) {
        self.repartition_mois = !self.repartition_mois;
    }
}

/// Dessine l'écran graphiques.
pub fn afficher(
    f: &mut Frame,
    donnees: &UiData,
    etat: &mut UiState,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) {
    let graphes = &donnees.graphiques;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // mini-onglets
            Constraint::Min(0),    // contenu
        ])
        .split(f.size());

    // Mini-onglets : Répartition / Cascade / Budget vs réel
    let onglets = ["Répartition", "Cascade", "Budget vs réel"];
    let items: Vec<ListItem> = onglets
        .iter()
        .enumerate()
        .map(|(i, nom)| {
            let style = if i == etat.onglet {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let suffix = if i == 0 && etat.repartition_mois {
                " (mois)"
            } else if i == 0 {
                " (année)"
            } else {
                ""
            };
            ListItem::new(format!(
                " {ctrl} {nom}{suffix}",
                ctrl = i + 1,
                nom = nom,
                suffix = suffix
            ))
            .style(style)
        })
        .collect();
    let liste = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Mini-onglets (Ctrl+1..3, Tab pour basculer année/mois) "),
        )
        .style(Style::default().fg(Color::Gray));
    f.render_widget(liste, chunks[0]);

    // Enregistrer les zones cliquables des onglets graphiques.
    let onglet_largeur = chunks[0].width as usize / onglets.len();
    for (i, _) in onglets.iter().enumerate() {
        zones_cliquables.push((
            Rect::new(
                chunks[0].left() + (i * onglet_largeur) as u16,
                chunks[0].top(),
                onglet_largeur as u16,
                chunks[0].height,
            ),
            ActionClique::GraphiquesOnglet(i),
        ));
    }

    // Contenu selon l'onglet.
    match etat.onglet {
        0 => afficher_repartition(f, graphes, etat, zones_cliquables, chunks[0]),
        1 => afficher_cascade(f, graphes, zones_cliquables, chunks[0]),
        _ => afficher_budget_vs_reel(f, donnees, zones_cliquables, chunks[0]),
    }
}

/// Répartition des dépenses (top 8 + Autres).
fn afficher_repartition(
    f: &mut Frame,
    graphes: &GraphesData,
    etat: &UiState,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
    area: Rect,
) {
    let lignes = if etat.repartition_mois {
        &graphes.repartition_mois
    } else {
        &graphes.repartition_annee
    };

    let total: Decimal = lignes.iter().map(|(_, m)| *m).sum();

    let rows: Vec<Row> = lignes
        .iter()
        .enumerate()
        .map(|(i, (categorie, montant))| {
            let pct = if !total.is_zero() {
                (f64::try_from(*montant / total).unwrap_or(0.0) * 100.0) as u32
            } else {
                0
            };
            Row::new(vec![
                categorie.clone(),
                eur(*montant).to_string(),
                format!("{pct}%"),
                "█".repeat(pct as usize / 2),
            ])
            .style(if i == etat.onglet {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            })
        })
        .collect();

    let titre = if etat.repartition_mois {
        " Répartition des dépenses — mois courant "
    } else {
        " Répartition des dépenses — année "
    };

    let tableau = Table::new(
        rows,
        [
            Constraint::Length(30),
            Constraint::Length(15),
            Constraint::Length(8),
            Constraint::Length(30),
        ],
    )
    .header(
        Row::new(vec!["Catégorie", "Montant", "%", "Barre"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title(titre))
    .column_spacing(1);

    f.render_widget(tableau, f.size());

    // Enregistrer les zones cliquables des lignes de répartition.
    for (i, (_categorie, _)) in lignes.iter().enumerate() {
        let ligne_y = area.top() + 1 + i as u16;
        if ligne_y >= area.bottom() {
            break;
        }
        zones_cliquables.push((
            Rect::new(area.left() + 1, ligne_y, area.width - 2, 1),
            ActionClique::GraphiquesOnglet(i),
        ));
    }
}

/// Cascade annuelle (identité comptable).
fn afficher_cascade(
    f: &mut Frame,
    graphes: &GraphesData,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
    area: Rect,
) {
    let rows = vec![
        Row::new(vec![
            "Revenus".to_string(),
            eur(graphes.cascade.revenus).to_string(),
        ]),
        Row::new(vec![
            "Dépenses".to_string(),
            format!("-{}", eur(graphes.cascade.depenses).to_string()),
        ]),
        Row::new(vec![
            "Δ patrimoine".to_string(),
            eur(graphes.cascade.variation_patrimoine).to_string(),
        ]),
        Row::new(vec![
            "Épargne (info)".to_string(),
            eur(graphes.cascade.epargne).to_string(),
        ]),
    ];

    let nb_rows = rows.len();
    let tableau = Table::new(rows, [Constraint::Length(20), Constraint::Length(15)])
        .header(
            Row::new(vec!["Poste", "Montant"]).style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Cascade annuelle — identité comptable "),
        )
        .column_spacing(1);

    f.render_widget(tableau, f.size());

    // Enregistrer les zones cliquables des lignes de cascade.
    for i in 0..nb_rows {
        let ligne_y = area.top() + 1 + i as u16;
        if ligne_y >= area.bottom() {
            break;
        }
        zones_cliquables.push((
            Rect::new(area.left() + 1, ligne_y, area.width - 2, 1),
            ActionClique::GraphiquesOnglet(i),
        ));
    }
}

/// Budget vs réel du mois (2 barres groupées).
fn afficher_budget_vs_reel(
    f: &mut Frame,
    donnees: &UiData,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
    area: Rect,
) {
    let lignes = &donnees.tableau_de_bord.budget_mois;
    if lignes.is_empty() {
        let vide = Paragraph::new("Aucune donnée budgétaire pour ce mois.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Budget vs réel — mois courant "),
            )
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(vide, f.size());
        return;
    }

    let rows: Vec<Row> = lignes
        .iter()
        .take(8)
        .map(|ligne| {
            let depasse = ligne.reel > ligne.budgete;
            let style = if depasse {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };
            Row::new(vec![
                ligne.compte.clone(),
                eur(ligne.budgete).to_string(),
                eur(ligne.reel).to_string(),
                eur(ligne.ecart()).to_string(),
            ])
            .style(style)
        })
        .collect();

    let tableau = Table::new(
        rows,
        [
            Constraint::Length(28),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["Poste", "Budgeté", "Réel", "Écart"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title(format!(
        " Budget vs réel — {} ",
        donnees.tableau_de_bord.budget_mois_label
    )))
    .column_spacing(1);

    f.render_widget(tableau, f.size());

    // Enregistrer les zones cliquables des lignes de budget vs réel.
    let ligne_y_debut = area.top() + 1;
    for (i, _) in lignes.iter().enumerate() {
        let ligne_y = ligne_y_debut + i as u16;
        if ligne_y >= area.bottom() {
            break;
        }
        zones_cliquables.push((
            Rect::new(area.left() + 1, ligne_y, area.width - 2, 1),
            ActionClique::GraphiquesOnglet(i),
        ));
    }
}

/// Gère un événement clavier pour le graphiques.
pub fn on_key(key: KeyEvent, etat: &mut UiState) -> Option<ActionGraphiques> {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Left => {
            etat.onglet_precedent();
            None
        }
        KeyCode::Right => {
            etat.onglet_suivant();
            None
        }
        KeyCode::Char('t') => {
            etat.bascule_repartition();
            None
        }
        KeyCode::Char('1') => {
            etat.onglet = 0;
            None
        }
        KeyCode::Char('2') => {
            etat.onglet = 1;
            None
        }
        KeyCode::Char('3') => {
            etat.onglet = 2;
            None
        }
        _ => None,
    }
}

/// Gère un événement souris pour le graphiques.
///
/// Retourne une `ActionGraphiques` si l'utilisateur a cliqué sur un onglet,
/// `None` sinon.
pub fn on_mouse(
    mouse: crossterm::event::MouseEvent,
    etat: &mut UiState,
) -> Option<ActionGraphiques> {
    use crossterm::event::MouseEventKind;

    if mouse.kind != MouseEventKind::Down(crossterm::event::MouseButton::Left) {
        return None;
    }

    // Les mini-onglets sont sur la première ligne (y = 1 environ, après la bordure).
    // Les onglets sont : Répartition (col 0-20), Cascade (col 20-40), Budget vs réel (col 40-60).
    if mouse.row < 2 {
        let x = mouse.column as usize;
        if x < 20 {
            etat.onglet = 0;
        } else if x < 40 {
            etat.onglet = 1;
        } else {
            etat.onglet = 2;
        }
        return None;
    }

    None
}

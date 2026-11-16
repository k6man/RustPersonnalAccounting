//! Écran Rapports (TUI).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Row, Table},
};

use crate::app::ActionClique;
use crate::fmt::eur;
use crate::viewmodel::RapportsData;

/// Dessine l'écran rapports dans la zone `area`.
pub fn afficher(
    f: &mut Frame,
    area: Rect,
    donnees: &RapportsData,
    _zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // synthèse
            Constraint::Min(0),    // dépenses par mois
        ])
        .split(area);

    // 1. Synthèse de l'année.
    let revenus = donnees.revenus_annee();
    let depenses = donnees.depenses_annee();
    let epargne = donnees.epargne_annee();
    let taux = donnees.taux_epargne().unwrap_or_default();

    let synthese = vec![
        ListItem::new(format!("  Revenus       : {}", eur(revenus))),
        ListItem::new(format!("  Dépenses      : {}", eur(depenses))),
        ListItem::new(format!("  Épargne nette : {}", eur(epargne))),
        ListItem::new(format!(
            "  Taux d'épargne: {:.1} %",
            (taux * rust_decimal::Decimal::from(100))
                .try_into()
                .unwrap_or(0.0)
        )),
    ];
    let liste = List::new(synthese).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Synthèse de l'année "),
    );
    f.render_widget(liste, chunks[0]);

    // 2. Dépenses par mois (tableau simple).
    let mois_noms = [
        "Jan", "Fév", "Mar", "Avr", "Mai", "Juin", "Jui", "Aoû", "Sep", "Oct", "Nov", "Déc",
    ];
    let rows: Vec<Row> = (0..12)
        .map(|m| {
            Row::new(vec![
                mois_noms[m].to_string(),
                eur(donnees.depenses_mois[m]),
                eur(donnees.revenus_mois[m]),
            ])
        })
        .collect();
    let tableau = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(15),
        ],
    )
    .header(
        Row::new(vec!["Mois", "Dépenses", "Revenus"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Dépenses / Revenus par mois "),
    )
    .column_spacing(1);
    f.render_widget(tableau, chunks[1]);
}

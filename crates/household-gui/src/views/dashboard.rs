//! Écran Tableau de bord (TUI).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};

use crate::app::ActionClique;
use crate::fmt::eur;
use crate::viewmodel::{DashboardData, EtatJauge};

/// Action utilisateur retournée par l'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionDashboard {
    /// Aucune action.
    Aucun,
}

/// Couleur selon l'état d'une jauge.
fn couleur_etat(etat: EtatJauge) -> Color {
    match etat {
        EtatJauge::Ok => Color::Green,
        EtatJauge::Alerte => Color::Yellow,
        EtatJauge::Depassement => Color::Red,
    }
}

/// Dessine le tableau de bord.
pub fn afficher(
    f: &mut Frame,
    donnees: &DashboardData,
    _zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) -> Option<ActionDashboard> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // soldes
            Constraint::Length(12), // jauges
            Constraint::Length(3),  // patrimoine net
            Constraint::Min(0),     // budget vs réel + courbe
        ])
        .split(f.size());

    // 1. Soldes des comptes clés.
    let soldes = vec![
        ListItem::new(format!(
            "  Compte courant   : {}",
            donnees.solde_cc.map_or("–".to_string(), eur)
        )),
        ListItem::new(format!(
            "  Livret A         : {}",
            donnees.solde_livret_a.map_or("–".to_string(), eur)
        )),
        ListItem::new(format!(
            "  LDD              : {}",
            donnees.solde_ldd.map_or("–".to_string(), eur)
        )),
    ];
    let liste_soldes =
        List::new(soldes).block(Block::default().borders(Borders::ALL).title(" Soldes "));
    f.render_widget(liste_soldes, chunks[0]);

    // 2. Jauges de plafond.
    let jauges_items: Vec<ListItem> = donnees
        .jauges
        .iter()
        .map(|jauge| {
            let fraction = EtatJauge::fraction(&EtatJauge::Ok, jauge.courant, jauge.plafond);
            let etat_str = match jauge.etat {
                EtatJauge::Ok => "OK",
                EtatJauge::Alerte => "ALERTE",
                EtatJauge::Depassement => "DEPASSEMENT",
            };
            let couleur = couleur_etat(jauge.etat);
            let nom = if jauge.compte.len() > 20 {
                format!("{}&...", &jauge.compte[..17])
            } else {
                jauge.compte.clone()
            };
            ListItem::new(format!(
                "  {:>3}% {:<22} {:>10} / {:<10} [{}]",
                (fraction * 100.0) as u32,
                nom,
                eur(jauge.courant),
                eur(jauge.plafond),
                etat_str
            ))
            .style(Style::default().fg(couleur))
        })
        .collect();
    let liste_jauges = List::new(jauges_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Plafonds d'épargne "),
    );
    f.render_widget(liste_jauges, chunks[1]);

    // 3. Patrimoine net.
    let patrimoine_texte = Paragraph::new(format!(
        " Patrimoine net : {} ",
        eur(donnees.patrimoine_net)
    ))
    .block(Block::default().borders(Borders::ALL))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(patrimoine_texte, chunks[2]);

    // 4. Budget vs réel du mois courant.
    let bvr_rows: Vec<Row> = donnees
        .budget_mois
        .iter()
        .take(8)
        .map(|ligne| {
            let depassement = ligne.reel > ligne.budgete;
            let style = if depassement {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };
            Row::new(vec![
                format!("{:>10}", eur(ligne.budgete)),
                format!("{:>10}", eur(ligne.reel)),
                format!("{:>10}", eur(ligne.reel - ligne.budgete)),
                ligne.compte.clone(),
            ])
            .style(style)
        })
        .collect();
    if !bvr_rows.is_empty() {
        let bvr_table = Table::new(
            bvr_rows,
            [
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(20),
            ],
        )
        .header(
            Row::new(vec!["Budgeté", "Réel", "Écart", "Compte"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Budget vs réel — {}", donnees.budget_mois_label)),
        )
        .column_spacing(1);
        f.render_widget(bvr_table, chunks[3]);
    } else {
        let vide = Paragraph::new("Aucune donnée budgétaire pour ce mois.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Budget vs réel — {}", donnees.budget_mois_label)),
        );
        f.render_widget(vide, chunks[3]);
    }

    None
}

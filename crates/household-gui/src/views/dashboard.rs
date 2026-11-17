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
use rust_decimal::Decimal;

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

/// Dessine le tableau de bord dans la zone `area` (jamais la frame entière :
/// la barre d'onglets et l'en-tête sont au-dessus).
pub fn afficher(
    f: &mut Frame,
    area: Rect,
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
        .split(area);

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
            let nom = if jauge.compte.chars().count() > 20 {
                // Troncature sûre : couper sur frontière de caractère
                // (un slice d'octets paniquerait sur un compte accentué).
                let tronque: String = jauge.compte.chars().take(17).collect();
                format!("{tronque}...")
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

    // 4. Budget vs réel du mois courant. Le bas est scindé : la table,
    // puis une bande « solde prévisionnel » d'une ligne.
    let bas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(chunks[3]);

    let mut bvr_rows: Vec<Row> = donnees
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

    // Ligne de total du mois (tous les postes, pas seulement les 8
    // affichés) : même convention de colonnes que les lignes au-dessus.
    if !donnees.budget_mois.is_empty() {
        bvr_rows.push(
            Row::new(vec![
                format!("{:>10}", eur(donnees.total_budgete)),
                format!("{:>10}", eur(donnees.total_reel)),
                format!("{:>10}", eur(donnees.total_reel - donnees.total_budgete)),
                "TOTAL".to_string(),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        );
    }
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
        f.render_widget(bvr_table, bas[0]);

        // Bande « solde prévisionnel » : combien il restera sur le compte
        // courant à la fin du mois si le budget est respecté. Le revenu
        // déjà encaissé est dans le solde réel ; le reste à dépenser
        // partira encore (le budget ne porte que des postes de dépenses).
        let prevu = donnees
            .solde_cc
            .map(|solde| solde - donnees.reste_a_depenser);
        let texte = match donnees.solde_cc {
            Some(solde) => format!(
                " Solde prévisionnel fin de mois : {} − {} = {} ",
                eur(solde),
                eur(donnees.reste_a_depenser),
                eur(prevu.unwrap_or_default())
            ),
            None => " Solde prévisionnel fin de mois : compte courant inconnu ".to_string(),
        };
        let style = if prevu.is_some_and(|p| p < Decimal::ZERO) {
            // Le solde irait en négatif : à voir tout de suite.
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        };
        f.render_widget(Paragraph::new(texte).style(style), bas[1]);
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

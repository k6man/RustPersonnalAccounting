//! Écran Graphiques (TUI).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};

use crate::fmt::eur;
use crate::viewmodel::{GraphesData, UiData};

/// État de l'écran graphiques (mini-onglet + période).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub onglet: usize,
    pub annee: i16,
    pub mois: Option<u8>,
}

impl UiState {
    pub fn nouveau() -> Self {
        Self::default()
    }
}

/// Dessine l'écran graphiques.
pub fn afficher(f: &mut Frame, donnees: &UiData, etat: &mut UiState) {
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
            ListItem::new(format!(" {ctrl} {nom}", ctrl = i + 1, nom = nom)).style(style)
        })
        .collect();
    let liste = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Mini-onglets (Ctrl+1..3) "),
        )
        .style(Style::default().fg(Color::Gray));
    f.render_widget(liste, chunks[0]);

    match etat.onglet {
        0 => afficher_repartition(f, graphes),
        1 => afficher_cascade(f, graphes),
        2 => afficher_budget_vs_reel(f, donnees),
        _ => afficher_repartition(f, graphes),
    }
}

/// Répartition des dépenses (top 8 + Autres).
fn afficher_repartition(f: &mut Frame, graphes: &GraphesData) {
    let rows: Vec<Row> = graphes
        .repartition_annee
        .iter()
        .map(|(cat, montant)| Row::new(vec![cat.clone(), eur(*montant).to_string()]))
        .collect();
    let tableau = Table::new(rows, [Constraint::Length(30), Constraint::Length(15)])
        .header(
            Row::new(vec!["Catégorie", "Montant"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Répartition des dépenses (année) "),
        )
        .column_spacing(1);
    f.render_widget(tableau, f.size());
}

/// Cascade annuelle (identité comptable).
fn afficher_cascade(f: &mut Frame, graphes: &GraphesData) {
    let _ = graphes;
    let texte = Paragraph::new("Cascade annuelle — à implémenter")
        .block(Block::default().borders(Borders::ALL).title(" Cascade "))
        .style(Style::default().fg(Color::White));
    f.render_widget(texte, f.size());
}

/// Budget vs réel du mois.
fn afficher_budget_vs_reel(f: &mut Frame, donnees: &UiData) {
    let _ = donnees;
    let texte = Paragraph::new("Budget vs réel — à implémenter")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Budget vs réel "),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(texte, f.size());
}

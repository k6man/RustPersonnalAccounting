//! Écran Saisie (TUI).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};

use crate::fmt::eur;
use crate::saisie::{Brouillon, Empreinte};
use crate::viewmodel::{InfoCompte, LigneJournal};

/// Action utilisateur retournée par l'écran saisie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Aucune action.
    Aucun,
    /// Enregistrer le brouillon.
    Enregistrer,
    /// Annuler.
    Annuler,
}

/// Mode de saisie.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    /// Nouvelle transaction.
    #[default]
    Nouveau,
    /// Modification d'une transaction existante.
    Modification { index: usize, empreinte: Empreinte },
}

/// État de l'écran saisie.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub mode: Mode,
    pub brouillon: Brouillon,
}

impl UiState {
    /// Nouveau brouillon vierge daté du jour.
    pub fn nouveau(aujourdhui: rustledger_core::NaiveDate) -> Self {
        let _ = aujourdhui;
        Self {
            mode: Mode::Nouveau,
            brouillon: Brouillon {
                date: aujourdhui.to_string(),
                flag: '*',
                ..Default::default()
            },
        }
    }

    /// Pré-remplit la saisie depuis une ligne du journal (modification).
    pub fn modification(ligne: &LigneJournal) -> Self {
        Self {
            mode: Mode::Modification {
                index: ligne.index,
                empreinte: Empreinte::de_ligne(ligne),
            },
            brouillon: Brouillon {
                date: ligne.date.to_string(),
                flag: ligne.flag,
                tiers: ligne.tiers.clone().unwrap_or_default(),
                libelle: ligne.libelle.clone(),
                ..Default::default()
            },
        }
    }
}

/// Dessine l'écran saisie.
pub fn afficher(
    f: &mut Frame,
    etat: &mut UiState,
    _comptes: &[InfoCompte],
    _payees: &[(String, usize)],
) -> Option<Action> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // en-tête (date, flag, tiers, libellé)
            Constraint::Length(3), // tags, liens
            Constraint::Min(0),    // lignes de postings
            Constraint::Length(3), // équilibrage
        ])
        .split(f.size());

    // 1. En-tête de la transaction.
    let b = &etat.brouillon;
    let entete = vec![
        ListItem::new(format!("  Date       : {}", b.date)),
        ListItem::new(format!(
            "  Flag       : {}",
            if b.flag == '*' { "pointée" } else { "attente" }
        )),
        ListItem::new(format!("  Tiers      : {}", b.tiers)),
        ListItem::new(format!("  Libellé    : {}", b.libelle)),
    ];
    let liste_entete = List::new(entete).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Transaction "),
    );
    f.render_widget(liste_entete, chunks[0]);

    // 2. Tags et liens.
    let meta = vec![
        ListItem::new(format!("  Tags       : {}", b.tags)),
        ListItem::new(format!("  Liens      : {}", b.liens)),
    ];
    let liste_meta = List::new(meta).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Métadonnées "),
    );
    f.render_widget(liste_meta, chunks[1]);

    // 3. Lignes de postings.
    let rows: Vec<Row> = b
        .lignes
        .iter()
        .enumerate()
        .map(|(i, ligne)| {
            let est_equilibrage = ligne.compte.is_empty() && ligne.montant.is_empty();
            Row::new(vec![
                format!("{i}"),
                ligne.compte.clone(),
                ligne.montant.clone(),
                if est_equilibrage {
                    "ÉQUILIBRAGE".to_string()
                } else {
                    "".to_string()
                },
            ])
        })
        .collect();
    let tableau = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(30),
            Constraint::Length(15),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["#", "Compte", "Montant", "Rôle"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Postings (ajoutez des lignes avec +, supprimez avec ×) "),
    )
    .column_spacing(1);
    f.render_widget(tableau, chunks[2]);

    // 4. Équilibrage.
    let total_eur: rust_decimal::Decimal = b
        .lignes
        .iter()
        .filter_map(|l| l.montant.parse::<rust_decimal::Decimal>().ok())
        .sum();
    let est_equilibre = total_eur.is_zero();
    let equilibrage = Paragraph::new(format!(
        "  Total : {}  {}",
        eur(total_eur),
        if est_equilibre {
            "✓ équilibré"
        } else {
            "✗ non équilibré"
        }
    ))
    .block(Block::default().borders(Borders::ALL))
    .style(if est_equilibre {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    });
    f.render_widget(equilibrage, chunks[3]);

    None
}

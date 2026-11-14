//! Écran Saisie (stub TUI — à implémenter).
//!
//! Les types de données (`Brouillon`, `Empreinte`, `LigneBrouillon`) sont
//! définis dans [`crate::saisie`] (logique pure) et réutilisés ici.

use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

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
    _f: &mut Frame,
    _etat: &mut UiState,
    _comptes: &[InfoCompte],
    _payees: &[(String, usize)],
) -> Option<Action> {
    let block = Block::default().title("Saisie").borders(Borders::ALL);
    let texte = Paragraph::new("Saisie — à implémenter en ratatui")
        .block(block)
        .style(Style::default().fg(Color::White));
    _f.render_widget(texte, _f.size());
    None
}

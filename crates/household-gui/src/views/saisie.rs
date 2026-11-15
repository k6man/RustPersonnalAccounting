//! Écran Saisie (TUI interactif).
//!
//! Permet de saisir une transaction avec des champs éditables (date, flag,
//! tiers, libellé, tags, liens) et des lignes de postings dynamiques.
//! La navigation entre les champs se fait avec Tab / Maj+Tab, et l'édition
//! de chaque champ avec les touches classiques (flèches, Backspace, Suppr…).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};

use crate::fmt::eur;
use crate::saisie::{Brouillon, Empreinte, LigneBrouillon};
use crate::viewmodel::{InfoCompte, LigneJournal};
use crate::widgets::text_input::TextInput;

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

/// Champ actuellement sélectionné dans le formulaire de saisie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Champ {
    #[default]
    Date,
    Flag,
    Tiers,
    Libelle,
    Tags,
    Liens,
    Postings,
}

impl Champ {
    /// Nombre de champs (pour le cycle Tab).
    pub const COUNT: usize = 7;

    /// Libellé affiché à gauche du champ.
    pub fn label(self) -> &'static str {
        match self {
            Self::Date => "Date",
            Self::Flag => "Flag",
            Self::Tiers => "Tiers",
            Self::Libelle => "Libellé",
            Self::Tags => "Tags",
            Self::Liens => "Liens",
            Self::Postings => "Postings",
        }
    }

    /// Passe au champ suivant (cycle).
    pub fn suivant(self) -> Self {
        match self {
            Self::Date => Self::Flag,
            Self::Flag => Self::Tiers,
            Self::Tiers => Self::Libelle,
            Self::Libelle => Self::Tags,
            Self::Tags => Self::Liens,
            Self::Liens => Self::Postings,
            Self::Postings => Self::Date,
        }
    }

    /// Passe au champ précédent (cycle).
    pub fn precedent(self) -> Self {
        match self {
            Self::Date => Self::Postings,
            Self::Flag => Self::Date,
            Self::Tiers => Self::Flag,
            Self::Libelle => Self::Tiers,
            Self::Tags => Self::Libelle,
            Self::Liens => Self::Tags,
            Self::Postings => Self::Liens,
        }
    }
}

/// État de l'écran saisie.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub mode: Mode,
    pub brouillon: Brouillon,
    /// Champ actuellement sélectionné.
    pub champ_actif: Champ,
    /// Index du posting sélectionné dans la liste.
    pub posting_selectionne: usize,
    /// TextInputs pour les champs texte.
    input_date: TextInput,
    input_tiers: TextInput,
    input_libelle: TextInput,
    input_tags: TextInput,
    input_liens: TextInput,
}

impl UiState {
    /// Nouveau brouillon vierge daté du jour.
    pub fn nouveau(aujourdhui: rustledger_core::NaiveDate) -> Self {
        let date_str = aujourdhui.to_string();
        let mut si = Self {
            mode: Mode::Nouveau,
            brouillon: Brouillon {
                date: date_str.clone(),
                flag: '*',
                ..Default::default()
            },
            champ_actif: Champ::Date,
            posting_selectionne: 0,
            input_date: TextInput::with_value(date_str).label("Date"),
            input_tiers: TextInput::new().label("Tiers"),
            input_libelle: TextInput::new().label("Libellé"),
            input_tags: TextInput::new().label("Tags"),
            input_liens: TextInput::new().label("Liens"),
        };
        si.sync_inputs_to_brouillon();
        si
    }

    /// Pré-remplit la saisie depuis une ligne du journal (modification).
    pub fn modification(ligne: &LigneJournal) -> Self {
        let mut si = Self {
            mode: Mode::Modification {
                index: ligne.index,
                empreinte: Empreinte::de_ligne(ligne),
            },
            brouillon: Brouillon {
                date: ligne.date.to_string(),
                flag: ligne.flag,
                tiers: ligne.tiers.clone().unwrap_or_default(),
                libelle: ligne.libelle.clone(),
                lignes: ligne
                    .lignes_posting
                    .iter()
                    .map(|p| LigneBrouillon {
                        compte: p.compte.clone(),
                        montant: p.montant.as_ref().map(|(v, _)| eur(*v)).unwrap_or_default(),
                    })
                    .collect(),
                ..Default::default()
            },
            champ_actif: Champ::Date,
            posting_selectionne: 0,
            input_date: TextInput::with_value(ligne.date.to_string()).label("Date"),
            input_tiers: TextInput::with_value(ligne.tiers.clone().unwrap_or_default())
                .label("Tiers"),
            input_libelle: TextInput::with_value(ligne.libelle.clone()).label("Libellé"),
            input_tags: TextInput::with_value(ligne.tags.join(" ")).label("Tags"),
            input_liens: TextInput::with_value(ligne.liens.join(" ")).label("Liens"),
        };
        si.sync_inputs_to_brouillon();
        si
    }

    /// Synchronise les TextInputs vers le brouillon.
    fn sync_inputs_to_brouillon(&mut self) {
        self.brouillon.date = self.input_date.value.clone();
        self.brouillon.tiers = self.input_tiers.value.clone();
        self.brouillon.libelle = self.input_libelle.value.clone();
        self.brouillon.tags = self.input_tags.value.clone();
        self.brouillon.liens = self.input_liens.value.clone();
    }

    /// Synchronise le brouillon vers les TextInputs.
    #[allow(dead_code)]
    fn sync_brouillon_to_inputs(&mut self) {
        self.input_date.value = self.brouillon.date.clone();
        self.input_tiers.value = self.brouillon.tiers.clone();
        self.input_libelle.value = self.brouillon.libelle.clone();
        self.input_tags.value = self.brouillon.tags.clone();
        self.input_liens.value = self.brouillon.liens.clone();
        // Le curseur va à la fin de chaque champ.
        self.input_date.cursor = self.input_date.value.len();
        self.input_tiers.cursor = self.input_tiers.value.len();
        self.input_libelle.cursor = self.input_libelle.value.len();
        self.input_tags.cursor = self.input_tags.value.len();
        self.input_liens.cursor = self.input_liens.value.len();
    }

    /// Retourne le TextInput du champ actif.
    pub fn input_actif(&mut self) -> &mut TextInput {
        match self.champ_actif {
            Champ::Date => &mut self.input_date,
            Champ::Flag => &mut self.input_tiers, // pas d'input dédié pour le flag
            Champ::Tiers => &mut self.input_tiers,
            Champ::Libelle => &mut self.input_libelle,
            Champ::Tags => &mut self.input_tags,
            Champ::Liens => &mut self.input_liens,
            Champ::Postings => &mut self.input_libelle, // fallback
        }
    }

    /// Nombre de postings.
    pub fn nb_postings(&self) -> usize {
        self.brouillon.lignes.len()
    }

    /// Retourne le posting sélectionné (si existe).
    pub fn posting_selectionne(&self) -> Option<&LigneBrouillon> {
        self.brouillon.lignes.get(self.posting_selectionne)
    }

    /// Retourne le posting sélectionné mutably (si existe).
    pub fn posting_selectionne_mut(&mut self) -> Option<&mut LigneBrouillon> {
        self.brouillon.lignes.get_mut(self.posting_selectionne)
    }
}

impl UiState {
    /// Gère un événement clavier. Retourne une `Action` si l'utilisateur a
    /// demandé une action globale (enregistrer, annuler).
    pub fn on_key(
        &mut self,
        key: KeyEvent,
        _comptes: &[InfoCompte],
        _payees: &[(String, usize)],
    ) -> Option<Action> {
        // Si le champ actif est un TextInput, on lui délègue d'abord.
        let input_consomme = if matches!(
            self.champ_actif,
            Champ::Date | Champ::Tiers | Champ::Libelle | Champ::Tags | Champ::Liens
        ) {
            self.input_actif().on_key(key)
        } else {
            false
        };
        if input_consomme {
            self.sync_inputs_to_brouillon();
            return None;
        }

        match key.code {
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.champ_actif = self.champ_actif.precedent();
                } else {
                    self.champ_actif = self.champ_actif.suivant();
                }
                // Réinitialiser le curseur du nouveau champ si c'est un text input.
                match self.champ_actif {
                    Champ::Date => {
                        self.input_date.cursor = self.input_date.value.len();
                    }
                    Champ::Tiers => {
                        self.input_tiers.cursor = self.input_tiers.value.len();
                    }
                    Champ::Libelle => {
                        self.input_libelle.cursor = self.input_libelle.value.len();
                    }
                    Champ::Tags => {
                        self.input_tags.cursor = self.input_tags.value.len();
                    }
                    Champ::Liens => {
                        self.input_liens.cursor = self.input_liens.value.len();
                    }
                    _ => {}
                }
                None
            }
            KeyCode::Enter => {
                // Dans le champ postings, Entrée ajoute une nouvelle ligne.
                if self.champ_actif == Champ::Postings {
                    self.brouillon.lignes.push(LigneBrouillon::default());
                    self.posting_selectionne = self.brouillon.lignes.len() - 1;
                    return None;
                }
                // Sinon, passer au champ suivant.
                self.champ_actif = self.champ_actif.suivant();
                None
            }
            KeyCode::Char('+') => {
                // Ajouter une ligne de posting.
                self.brouillon.lignes.push(LigneBrouillon::default());
                self.posting_selectionne = self.brouillon.lignes.len() - 1;
                self.champ_actif = Champ::Postings;
                None
            }
            KeyCode::Char('×') | KeyCode::Char('x') => {
                // Supprimer la ligne de posting sélectionnée.
                if !self.brouillon.lignes.is_empty() {
                    self.brouillon.lignes.remove(self.posting_selectionne);
                    if self.posting_selectionne >= self.brouillon.lignes.len() {
                        self.posting_selectionne = self.brouillon.lignes.len().saturating_sub(1);
                    }
                }
                None
            }
            KeyCode::Char('e') => {
                // Éditer le compte du posting sélectionné.
                if let Some(_posting) = self.posting_selectionne_mut() {
                    self.champ_actif = Champ::Postings;
                }
                None
            }
            KeyCode::Esc => Some(Action::Annuler),
            KeyCode::Char('\n') => Some(Action::Enregistrer),
            _ => None,
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
            Constraint::Length(7), // en-tête (date, flag, tiers, libellé)
            Constraint::Length(3), // tags, liens
            Constraint::Min(0),    // lignes de postings
            Constraint::Length(3), // équilibrage
        ])
        .split(f.size());

    // 1. En-tête de la transaction.
    let b = &etat.brouillon;

    // Champs texte avec focus visuel.
    let champ_date = Paragraph::new(etat.input_date.value.as_str())
        .block(Block::default().borders(Borders::ALL).title(format!(
            " {} (Tab pour naviguer) ",
            etat.champ_actif.label()
        )))
        .style(if etat.champ_actif == Champ::Date {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        });
    f.render_widget(champ_date, chunks[0]);

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
            let style = if i == etat.posting_selectionne {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
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
            .style(style)
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
            .title(" Postings (+ ajouter, × supprimer, e éditer) "),
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

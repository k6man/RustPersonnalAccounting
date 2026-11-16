//! Écran Saisie (TUI interactif).
//!
//! Permet de saisir une transaction : Date, Flag, Tiers, Libellé, Tags,
//! Liens (une ligne chacun, caret sur le champ actif) et des lignes de
//! postings dynamiques (compte + montant éditables via `e`).
//! Navigation entre les champs avec Tab / Maj+Tab ; les touches classiques
//! éditent le champ actif (flèches, Backspace, Suppr…).
//!
//! La forme est validée par le module pur [`crate::saisie`] (`erreurs`,
//! `residus`, `parse_montant`) : la vue n'importe que l'affichage.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::app::ActionClique;
use crate::fmt::{eur, nombre};
use crate::saisie::{Brouillon, Empreinte, LigneBrouillon, parse_montant};
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

/// Sous-champ édité dans une ligne de posting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChampPosting {
    Compte,
    Montant,
}

/// Édition d'une ligne de posting en cours (toutes les touches y vont).
///
/// Le brouillon n'est modifié qu'à la validation (Entrée sur le montant) :
/// Esc annule sans effet, un montant invalide garde l'édition ouverte
/// (cellule rendue en rouge).
#[derive(Debug, Clone)]
pub struct EditionPosting {
    /// Index de la ligne dans `brouillon.lignes`.
    pub index: usize,
    /// Sous-champ en cours d'édition.
    pub champ: ChampPosting,
    /// Champ de saisie du sous-champ courant (pré-rempli).
    pub input: TextInput,
    /// Compte édité, conservé pendant que le montant est édité.
    pub compte_edite: String,
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
    /// Édition d'une ligne de posting en cours (`None` hors édition).
    pub edition_posting: Option<EditionPosting>,
    /// TextInputs pour les champs texte.
    input_date: TextInput,
    input_tiers: TextInput,
    input_libelle: TextInput,
    input_tags: TextInput,
    input_liens: TextInput,
}

impl UiState {
    /// Nouveau brouillon vierge daté du jour (deux lignes de postings
    /// prêtes, cf. [`Brouillon::vierge`]).
    pub fn nouveau(aujourdhui: rustledger_core::NaiveDate) -> Self {
        let mut si = Self {
            mode: Mode::Nouveau,
            brouillon: Brouillon::vierge(aujourdhui),
            champ_actif: Champ::Date,
            posting_selectionne: 0,
            edition_posting: None,
            ..Default::default()
        };
        si.recharge_inputs();
        si
    }

    /// Pré-remplit la saisie depuis une ligne du journal (modification) —
    /// le pré-remplissage passe par [`crate::saisie::depuis_ligne`] :
    /// montants re-parsables (`"85.00"`, jamais `"85,00 €"`), tags avec
    /// leur préfixe `#`, liens avec `^`.
    pub fn modification(ligne: &LigneJournal) -> Self {
        let mut si = Self {
            mode: Mode::Modification {
                index: ligne.index,
                empreinte: Empreinte::de_ligne(ligne),
            },
            brouillon: crate::saisie::depuis_ligne(ligne),
            champ_actif: Champ::Date,
            posting_selectionne: 0,
            edition_posting: None,
            ..Default::default()
        };
        si.recharge_inputs();
        si
    }

    /// Reconstruit les TextInputs depuis le brouillon (curseur à la fin).
    fn recharge_inputs(&mut self) {
        self.input_date = TextInput::with_value(self.brouillon.date.clone());
        self.input_tiers = TextInput::with_value(self.brouillon.tiers.clone());
        self.input_libelle = TextInput::with_value(self.brouillon.libelle.clone());
        self.input_tags = TextInput::with_value(self.brouillon.tags.clone());
        self.input_liens = TextInput::with_value(self.brouillon.liens.clone());
    }

    /// Synchronise les TextInputs vers le brouillon.
    fn sync_inputs_to_brouillon(&mut self) {
        self.brouillon.date = self.input_date.value.clone();
        self.brouillon.tiers = self.input_tiers.value.clone();
        self.brouillon.libelle = self.input_libelle.value.clone();
        self.brouillon.tags = self.input_tags.value.clone();
        self.brouillon.liens = self.input_liens.value.clone();
    }

    /// Synchronise le focus des TextInputs avec le champ actif (appelé au
    /// rendu : `champ_actif` reste la seule source de vérité).
    fn sync_focus(&mut self) {
        self.input_date.set_focused(self.champ_actif == Champ::Date);
        self.input_tiers
            .set_focused(self.champ_actif == Champ::Tiers);
        self.input_libelle
            .set_focused(self.champ_actif == Champ::Libelle);
        self.input_tags.set_focused(self.champ_actif == Champ::Tags);
        self.input_liens
            .set_focused(self.champ_actif == Champ::Liens);
    }

    /// Retourne le TextInput du champ actif — `None` pour Flag (bascule)
    /// et Postings (table), qui n'ont pas de champ texte.
    fn input_actif(&mut self) -> Option<&mut TextInput> {
        match self.champ_actif {
            Champ::Date => Some(&mut self.input_date),
            Champ::Tiers => Some(&mut self.input_tiers),
            Champ::Libelle => Some(&mut self.input_libelle),
            Champ::Tags => Some(&mut self.input_tags),
            Champ::Liens => Some(&mut self.input_liens),
            Champ::Flag | Champ::Postings => None,
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
        // Édition d'une ligne de posting : toutes les touches y vont (un
        // compte ou un montant peut contenir r, l, x…). Tab/Entrée passe du
        // compte au montant (Entrée sur le montant valide) ; Esc annule sans
        // écrire ; un montant invalide garde l'édition ouverte.
        if let Some(edition) = &mut self.edition_posting {
            if edition.input.on_key(key) {
                return None;
            }
            match key.code {
                KeyCode::Tab | KeyCode::Enter => {
                    if edition.champ == ChampPosting::Compte {
                        edition.compte_edite = edition.input.value.trim().to_string();
                        edition.champ = ChampPosting::Montant;
                        let valeur = self
                            .brouillon
                            .lignes
                            .get(edition.index)
                            .map_or_else(String::new, |ligne| ligne.montant.clone());
                        edition.input = TextInput::with_value(valeur);
                        edition.input.set_focused(true);
                    } else {
                        // Validation : montant vide = ligne d'équilibrage ;
                        // sinon il doit être parsable (virgule et suffixe de
                        // devise acceptés — le texte saisi est conservé tel
                        // quel, `parse_montant` le relira).
                        let texte = edition.input.value.trim().to_string();
                        if !texte.is_empty() && parse_montant(&texte).is_err() {
                            return None;
                        }
                        let Some(ligne_brouillon) = self.brouillon.lignes.get_mut(edition.index)
                        else {
                            self.edition_posting = None;
                            return None;
                        };
                        ligne_brouillon.compte = edition.compte_edite.clone();
                        ligne_brouillon.montant = texte;
                        self.edition_posting = None;
                    }
                }
                KeyCode::Esc => {
                    self.edition_posting = None;
                }
                _ => {}
            }
            return None;
        }

        // '+' ajoute une ligne de posting depuis n'importe quel champ
        // (raccourci global de l'écran) ; dans un champ texte, les autres
        // lettres — y compris e, x, q, r — sont du texte.
        if key.code == KeyCode::Char('+') {
            self.brouillon.lignes.push(LigneBrouillon::default());
            self.posting_selectionne = self.brouillon.lignes.len() - 1;
            self.champ_actif = Champ::Postings;
            return None;
        }

        let input_consomme = if matches!(
            self.champ_actif,
            Champ::Date | Champ::Tiers | Champ::Libelle | Champ::Tags | Champ::Liens
        ) {
            match key.code {
                KeyCode::Tab | KeyCode::Enter | KeyCode::Esc => false,
                _ => self.input_actif().is_some_and(|input| input.on_key(key)),
            }
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
                // Le curseur du nouveau champ va à la fin de la valeur.
                if let Some(input) = self.input_actif() {
                    input.cursor = input.value.len();
                }
                None
            }
            KeyCode::Enter => {
                // Sur le champ postings, Entrée valide le brouillon.
                // Sur les autres champs, passe au suivant.
                if self.champ_actif == Champ::Postings {
                    return Some(Action::Enregistrer);
                }
                self.champ_actif = self.champ_actif.suivant();
                None
            }
            KeyCode::Up if self.champ_actif == Champ::Postings => {
                self.posting_selectionne = self.posting_selectionne.saturating_sub(1);
                None
            }
            KeyCode::Down if self.champ_actif == Champ::Postings => {
                if !self.brouillon.lignes.is_empty() {
                    self.posting_selectionne =
                        (self.posting_selectionne + 1).min(self.brouillon.lignes.len() - 1);
                }
                None
            }
            KeyCode::Char('f') => {
                // Bascule du flag : '*' pointée ⇄ '!' en attente.
                self.brouillon.flag = if self.brouillon.flag == '*' { '!' } else { '*' };
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
                // Éditer compte + montant du posting sélectionné : ouvre
                // l'éditeur sur le compte (Tab/Entrée passe au montant).
                if self.posting_selectionne().is_some() {
                    let index = self.posting_selectionne;
                    let ligne = &self.brouillon.lignes[index];
                    let mut input = TextInput::with_value(ligne.compte.clone());
                    input.set_focused(true);
                    self.edition_posting = Some(EditionPosting {
                        index,
                        champ: ChampPosting::Compte,
                        compte_edite: ligne.compte.clone(),
                        input,
                    });
                    self.champ_actif = Champ::Postings;
                }
                None
            }
            KeyCode::Esc => Some(Action::Annuler),
            _ => None,
        }
    }
}

/// Largeur commune des étiquettes du formulaire (« Libellé : » = 10).
const LARGEUR_ETIQUETTE: usize = 10;

/// Ligne « Étiquette : valeur » du formulaire ; le champ actif est
/// surligné en bleu (le caret est porté par la ligne de valeur).
fn ligne_formulaire(etiquette: &str, valeur: Line<'static>, actif: bool) -> Line<'static> {
    let style_etiquette = if actif {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mut spans = vec![Span::styled(
        format!(
            " {:<width$}",
            format!("{etiquette} :"),
            width = LARGEUR_ETIQUETTE
        ),
        style_etiquette,
    )];
    for mut span in valeur.spans {
        if actif && span.style.bg.is_none() {
            span.style = span.style.bg(Color::Blue);
        }
        spans.push(span);
    }
    Line::from(spans)
}

/// Dessine l'écran saisie dans la zone `area`.
pub fn afficher(
    f: &mut Frame,
    area: Rect,
    etat: &mut UiState,
    _comptes: &[InfoCompte],
    _payees: &[(String, usize)],
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) -> Option<Action> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // formulaire : Date, Flag, Tiers, Libellé, Tags, Liens
            Constraint::Min(0),    // lignes de postings
            Constraint::Length(4), // avis : reste à répartir + première erreur
        ])
        .split(area);

    etat.sync_focus();
    let b = &etat.brouillon;
    let edition = etat.edition_posting.as_ref();

    // 1. Formulaire : une ligne par champ, caret sur le champ actif.
    let formulaire = vec![
        ligne_formulaire(
            "Date",
            etat.input_date.ligne_avec_curseur(),
            etat.champ_actif == Champ::Date,
        ),
        ligne_formulaire(
            "Flag",
            Line::from(format!("{}  (f pour basculer)", b.flag)),
            etat.champ_actif == Champ::Flag,
        ),
        ligne_formulaire(
            "Tiers",
            etat.input_tiers.ligne_avec_curseur(),
            etat.champ_actif == Champ::Tiers,
        ),
        ligne_formulaire(
            "Libellé",
            etat.input_libelle.ligne_avec_curseur(),
            etat.champ_actif == Champ::Libelle,
        ),
        ligne_formulaire(
            "Tags",
            etat.input_tags.ligne_avec_curseur(),
            etat.champ_actif == Champ::Tags,
        ),
        ligne_formulaire(
            "Liens",
            etat.input_liens.ligne_avec_curseur(),
            etat.champ_actif == Champ::Liens,
        ),
    ];
    f.render_widget(Paragraph::new(formulaire), chunks[0]);

    // 2. Lignes de postings.
    let rows: Vec<Row> = b
        .lignes
        .iter()
        .enumerate()
        .map(|(i, ligne)| {
            let edition_ligne = edition.filter(|e| e.index == i);
            let en_edition = edition_ligne.is_some();
            let style = if i == etat.posting_selectionne && !en_edition {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            // Compte affiché : caret si le compte est édité, valeur éditée
            // pendant que le montant l'est, valeur du brouillon sinon.
            let cellule_compte = match edition_ligne {
                Some(e) if e.champ == ChampPosting::Compte => {
                    Cell::from(e.input.ligne_avec_curseur()).style(cellule_edition(true))
                }
                Some(e) => Cell::from(e.compte_edite.clone()),
                None => Cell::from(ligne.compte.clone()),
            };

            // Montant affiché : symétrique ; rouge si la saisie n'est pas
            // un montant valide (l'édition reste ouverte pour corriger).
            let cellule_montant = match edition_ligne {
                Some(e) if e.champ == ChampPosting::Montant => {
                    let valide =
                        e.input.value.trim().is_empty() || parse_montant(&e.input.value).is_ok();
                    Cell::from(e.input.ligne_avec_curseur()).style(cellule_edition(valide))
                }
                Some(_) | None => Cell::from(ligne.montant.clone()),
            };

            // Ligne d'équilibrage = compte saisi, montant laissé vide
            // (la contrepartie est calculée à l'enregistrement).
            let est_equilibrage = !ligne.compte.is_empty() && ligne.montant.is_empty();

            Row::new(vec![
                Cell::from(format!("{i}")),
                cellule_compte,
                cellule_montant,
                Cell::from(if est_equilibrage { "ÉQUILIBRAGE" } else { "" }),
            ])
            .style(style)
        })
        .collect();
    let tableau = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Min(30),
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
    f.render_widget(tableau, chunks[1]);

    // Enregistrer les zones cliquables des lignes de postings. La première
    // rangée de données est 2 lignes sous le haut du bloc : bordure + en-tête.
    for (i, ligne) in etat.brouillon.lignes.iter().enumerate() {
        if !ligne.compte.is_empty() {
            let ligne_y = chunks[1].top() + 2 + i as u16;
            if ligne_y >= chunks[1].bottom() {
                break;
            }
            zones_cliquables.push((
                Rect::new(
                    chunks[1].left() + 4,
                    ligne_y,
                    chunks[1].width.saturating_sub(8),
                    1,
                ),
                ActionClique::SaisieSelection(i),
            ));
        }
    }

    // 3. Avis : reste à répartir par devise + première erreur de forme
    // (les deux indicateurs du module pur `Brouillon`).
    let residus = b.residus();
    let reste = if residus.is_empty() {
        "✓ équilibré".to_string()
    } else {
        let morceaux: Vec<String> = residus
            .iter()
            .map(|(devise, valeur)| {
                if devise == "EUR" {
                    eur(*valeur)
                } else {
                    format!("{} {devise}", nombre(*valeur))
                }
            })
            .collect();
        format!("✗ reste à répartir : {}", morceaux.join(" ; "))
    };
    let erreurs = b.erreurs();
    let forme = if erreurs.is_empty() {
        "✓ prête à enregistrer (Entrée sur Postings)".to_string()
    } else {
        let autres = erreurs.len() - 1;
        if autres > 0 {
            format!("✗ {} (+ {autres} autres)", erreurs[0])
        } else {
            format!("✗ {}", erreurs[0])
        }
    };
    let avis = Paragraph::new(vec![Line::from(reste), Line::from(forme)])
        .block(Block::default().borders(Borders::ALL).title(" Avis "));
    f.render_widget(avis, chunks[2]);

    None
}

/// Style d'une cellule en cours d'édition : bleu, blanc si la saisie est
/// valide, rouge sinon (l'édition reste ouverte pour corriger).
fn cellule_edition(valide: bool) -> Style {
    Style::default()
        .bg(Color::Blue)
        .fg(if valide { Color::White } else { Color::Red })
        .add_modifier(Modifier::BOLD)
}

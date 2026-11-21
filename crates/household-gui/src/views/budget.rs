//! Écran Budget (TUI) : grille postes × 12 mois.
//!
//! Affiche les lignes budgétaires avec budget et réel par mois, les totaux
//! par ligne (année) et les écarts. Les cellules de budget sont éditables
//! (navigation avec Tab, saisie directe, entrée pour valider).

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use std::collections::BTreeMap;

use crate::app::ActionClique;
use crate::fmt::{eur, nombre};
use crate::widgets::text_input::TextInput;
use household_budget::BudgetGrid;
use rust_decimal::Decimal;
use rustledger_core::NaiveDate;

/// Action utilisateur retournée par l'écran budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionBudget {
    /// Aucune action.
    Aucun,
    /// Modifier une cellule budgétaire.
    Cellule {
        annee: i16,
        date: NaiveDate,
        compte: String,
        montant: Decimal,
    },
    /// Commentaire d'une ligne (compte) : texte vide = suppression.
    Commentaire {
        annee: i16,
        compte: String,
        texte: String,
    },
    /// Générer l'année cible par duplication.
    GenererAnnee {
        source: i16,
        cible: i16,
        pourcent: Decimal,
        pas: Decimal,
    },
    /// Basculer le rollover d'une famille.
    BasculerRollover(String),
}

/// État de l'écran budget (année sélectionnée, cellule active, défilement).
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub annee: i16,
    /// Ligne sélectionnée dans la grille.
    pub ligne_selectionne: usize,
    /// Mois sélectionné (0..=11).
    pub mois_selectionne: usize,
    /// Posting sélectionné dans la ligne.
    pub posting_selectionne: usize,
    /// Édition de cellule en cours (toutes les touches y vont) — `None`
    /// hors édition. Validée par Entrée uniquement : tant qu'elle est
    /// ouverte, aucune écriture n'a lieu.
    pub edition: Option<EditionCellule>,
    /// Édition du commentaire de la ligne sélectionnée (toutes les touches
    /// y vont) — `None` hors édition. Validée par Entrée, annulée par Esc ;
    /// texte vide (après nettoyage) = suppression du commentaire.
    pub edition_commentaire: Option<EditionCommentaire>,
    /// Première ligne de données affichée (offset de défilement vertical).
    pub defilement: usize,
    /// Nombre de lignes de données visibles, mesuré au dernier rendu —
    /// utilisé par le routage clavier entre deux dessins.
    pub visibles: usize,
}

/// Édition d'une cellule budgétaire.
#[derive(Debug, Clone)]
pub struct EditionCellule {
    /// Ligne de la grille en cours d'édition.
    pub ligne: usize,
    /// Mois (0..=11) de la cellule en cours d'édition.
    pub mois: usize,
    /// Champ de saisie du montant, pré-rempli avec la valeur courante.
    pub input: TextInput,
}

/// Édition du commentaire d'une ligne budgétaire.
#[derive(Debug, Clone)]
pub struct EditionCommentaire {
    /// Ligne de la grille dont on édite le commentaire.
    pub ligne: usize,
    /// Champ de saisie du texte, pré-rempli avec la note existante.
    pub input: TextInput,
}

impl UiState {
    pub fn nouveau(annee: i16) -> Self {
        Self {
            annee,
            ligne_selectionne: 0,
            mois_selectionne: 0,
            posting_selectionne: 0,
            edition: None,
            edition_commentaire: None,
            defilement: 0,
            visibles: 0,
        }
    }

    /// Nombre de lignes de la grille.
    pub fn nb_lignes(&self, grille: Option<&BudgetGrid>) -> usize {
        grille.map_or(0, |g| g.rows.len())
    }

    /// Index de ligne sélectionné clampé.
    pub fn ligne_index(&self, grille: Option<&BudgetGrid>) -> usize {
        let nb = self.nb_lignes(grille);
        if nb == 0 {
            return 0;
        }
        self.ligne_selectionne.min(nb - 1)
    }

    /// Passe à la ligne suivante.
    pub fn ligne_suivante(&mut self, grille: Option<&BudgetGrid>) {
        let nb = self.nb_lignes(grille);
        if nb == 0 {
            return;
        }
        self.ligne_selectionne = (self.ligne_selectionne + 1).min(nb - 1);
    }

    /// Passe à la ligne précédente.
    pub fn ligne_precedente(&mut self) {
        if self.ligne_selectionne > 0 {
            self.ligne_selectionne -= 1;
        }
    }

    /// Passe au mois suivant.
    pub fn mois_suivant(&mut self) {
        self.mois_selectionne = (self.mois_selectionne + 1).min(11);
    }

    /// Passe au mois précédent.
    pub fn mois_precedent(&mut self) {
        if self.mois_selectionne > 0 {
            self.mois_selectionne -= 1;
        }
    }

    /// Défile de `delta` lignes (positif : vers le bas), borné à
    /// `[0, nb_total − visibles]`. Un `visibles` de 0 (terminal minuscule)
    /// est traité comme 1 : le défilement reste utilisable.
    pub fn defiler(&mut self, nb_total: usize, delta: isize) {
        let visibles = self.visibles.max(1);
        let max = nb_total.saturating_sub(visibles);
        let actuel = self.defilement.min(max) as isize;
        let cible = actuel + delta;
        self.defilement = cible.clamp(0, max as isize) as usize;
    }
}

/// Style de la croix de sélection (cellule active + en-tête du mois
/// sélectionné) : bleu + blanc + gras, cohérent avec l'éditeur de cellule
/// qui s'ouvre sur cette même cellule. La ligne est marquée à part, en
/// gris discret (voir `afficher`).
fn style_selection() -> Style {
    Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Dessine l'écran budget dans la zone `area`. `commentaires` est la map
/// compte → note de l'année affichée (lue depuis le ledger).
pub fn afficher(
    f: &mut Frame,
    area: Rect,
    grille: Option<&BudgetGrid>,
    etat: &mut UiState,
    commentaires: &BTreeMap<String, String>,
    _rollover_actif: &dyn Fn(&str) -> bool,
    zones_cliquables: &mut Vec<(Rect, ActionClique)>,
) -> Option<ActionBudget> {
    let grille = grille?;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // titre avec année
            Constraint::Length(1), // aide
            Constraint::Min(0),    // tableau
            Constraint::Length(1), // bande de commentaire de la ligne active
        ])
        .split(area);

    // 1. Titre (sans bordure : la rangée fait 1 ligne, une bordure masquerait
    // le texte).
    let titre = Paragraph::new(format!(
        " Budget {}  (flèches haut/bas pour naviguer, gauche/droite pour le mois)",
        etat.annee
    ))
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(titre, chunks[0]);

    // 2. Aide rapide.
    let aide = Paragraph::new(
        " flèches : naviguer · e : éditer la cellule · c : commentaire · Entrée : valider · Esc : annuler",
    )
    .style(Style::default().fg(Color::Gray));
    f.render_widget(aide, chunks[1]);

    // 3. Tableau budget vs réel — fenêtre glissante (défilement vertical).
    let lignes = &grille.rows;
    if lignes.is_empty() {
        let vide = Paragraph::new("Aucune ligne budgétaire pour cette année.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Budget — lignes "),
            )
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(vide, chunks[2]);
        return None;
    }

    let nb_total = lignes.len();
    let visibles = chunks[2]
        .height
        .saturating_sub(3) // bordures + en-tête
        .saturating_sub(1) // marge
        as usize;
    etat.visibles = visibles;
    etat.defilement = etat
        .defilement
        .min(nb_total.saturating_sub(visibles.max(1)));

    // Ajuste la sélection si elle sort de la fenêtre visible.
    let ligne_idx = etat.ligne_index(Some(grille));
    if ligne_idx < etat.defilement {
        etat.ligne_selectionne = etat.defilement;
    } else if ligne_idx >= etat.defilement + visibles {
        etat.ligne_selectionne = etat.defilement + visibles - 1;
    }
    let ligne_idx = etat.ligne_index(Some(grille));
    let mois_idx = etat.mois_selectionne;
    let edition = etat.edition.as_ref();

    // En-têtes de colonnes : Poste | Jan | Fév | ... | Déc | Total | Écart.
    // Croix de sélection : l'en-tête du mois sélectionné porte le même
    // marqueur bleu que la cellule — la colonne se repère d'un coup d'œil
    // depuis le haut du tableau, même quand la ligne est loin.
    let mois_noms = [
        "Jan", "Fév", "Mar", "Avr", "Mai", "Juin", "Jui", "Aoû", "Sep", "Oct", "Nov", "Déc",
    ];
    let header_cells: Vec<Cell> = std::iter::once(Cell::from("Poste"))
        .chain(mois_noms.iter().enumerate().map(|(m, nom)| {
            if m == mois_idx {
                Cell::from(*nom).style(style_selection())
            } else {
                Cell::from(*nom)
            }
        }))
        .chain([Cell::from("Total"), Cell::from("Écart")])
        .collect();

    let rows: Vec<Row> = lignes
        .iter()
        .enumerate()
        .skip(etat.defilement)
        .take(visibles)
        .map(|(i, ligne)| {
            let selected = i == ligne_idx;
            let style = if selected {
                // Croix de sélection : ligne en gris discret — le REVERSED
                // intégral écrasait le rouge de l'écart et ne disait rien
                // du mois. La cellule active (ci-dessous) porte le marqueur
                // fort ; `Cell::render` applique son style APRÈS celui de
                // la rangée (ratatui 0.26.3) : les champs `None` héritent
                // du fond gris, le bleu de la cellule l'écrase.
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            let mut cells: Vec<Cell> = vec![Cell::from(ligne.account.clone())];
            let mut total_budget = Decimal::ZERO;
            let mut total_reel = Decimal::ZERO;
            for m in 0..12 {
                let cell = &ligne.cells[m];
                total_budget += cell.budgeted;
                total_reel += cell.actual;
                if let Some(e) = edition.filter(|e| e.ligne == i && e.mois == m) {
                    // Cellule en cours d'édition : la saisie remplace la
                    // valeur, avec caret ; rouge si le montant n'est pas
                    // un Decimal valide (l'édition reste alors ouverte).
                    let valide = e
                        .input
                        .value
                        .trim()
                        .replace(',', ".")
                        .parse::<Decimal>()
                        .is_ok();
                    let style_cellule = Style::default()
                        .bg(Color::Blue)
                        .fg(if valide { Color::White } else { Color::Red })
                        .add_modifier(Modifier::BOLD);
                    cells.push(Cell::from(e.input.ligne_avec_curseur()).style(style_cellule));
                } else if selected && m == mois_idx {
                    // Cellule active de la croix de sélection (hors édition :
                    // la cellule éditée porte déjà son propre style bleu).
                    cells.push(
                        Cell::from(format!("{:>6}", nombre(cell.budgeted)))
                            .style(style_selection()),
                    );
                } else {
                    cells.push(Cell::from(format!("{:>6}", nombre(cell.budgeted))));
                }
            }
            let ecart = total_budget - total_reel;
            cells.push(Cell::from(format!("{:>10}", eur(total_budget))));
            let style_ecart = if ecart < Decimal::ZERO {
                // Dépassement : le réel a mangé le budget.
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };
            cells.push(Cell::from(format!("{:>10}", eur(ecart))).style(style_ecart));
            Row::new(cells).style(style)
        })
        .collect();

    // Largeurs proportionnelles : la grille tiendait ≈183 colonnes en
    // largeurs fixes — Total/Écart étaient hors écran dès 120 colonnes.
    // Les cellules mensuelles sont compactes (nombre sans €) : sur un
    // terminal étroit, le clip droit perd d'abord les décimales — dégradation
    // lisible (« 1 234 » reste affiché).
    let widths: Vec<Constraint> = std::iter::once(Constraint::Percentage(20))
        .chain(std::iter::repeat_n(Constraint::Percentage(5), 12))
        .chain([Constraint::Percentage(10), Constraint::Percentage(10)])
        .collect();

    let titre = if let Some(e) = edition {
        let compte = grille
            .rows
            .get(e.ligne)
            .map_or_else(|| "?".to_string(), |l| l.account.clone());
        format!(
            " Édition — {} {} (Entrée : valider, Esc : annuler) ",
            mois_noms[e.mois], compte
        )
    } else {
        // Position complète dans le titre : lisible même si les couleurs
        // passent mal sur le terminal de l'utilisateur.
        let compte = grille
            .rows
            .get(ligne_idx)
            .map_or_else(|| "?".to_string(), |l| l.account.clone());
        let mut titre = format!(
            " Budget vs réel — ligne {}/{} · {} · {} ",
            ligne_idx + 1,
            grille.rows.len(),
            mois_noms[mois_idx],
            compte
        );
        // Indicateur de défilement dans le titre quand des lignes sont cachées.
        if nb_total > visibles {
            titre = format!(
                " Budget vs réel — ligne {}/{} · {} · {} · lignes {}/{} ",
                ligne_idx + 1,
                grille.rows.len(),
                mois_noms[mois_idx],
                compte,
                etat.defilement + 1,
                nb_total
            );
        }
        titre
    };
    let tableau = Table::new(rows, widths)
        .header(
            Row::new(header_cells)
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(Block::default().borders(Borders::ALL).title(titre))
        .column_spacing(1);

    f.render_widget(tableau, chunks[2]);

    // Enregistrer les zones cliquables des lignes budgétaires : un clic
    // sélectionne la ligne (affichage), sans écriture dans le ledger. La
    // première rangée de données est 3 lignes sous le haut du bloc :
    // bordure, en-tête, marge basse d'en-tête. On ne prend que les lignes
    // visibles dans la fenêtre de défilement.
    for (i, _ligne) in lignes
        .iter()
        .enumerate()
        .skip(etat.defilement)
        .take(visibles)
    {
        let ligne_y = chunks[2].top() + 3 + (i - etat.defilement) as u16;
        if ligne_y >= chunks[2].bottom() {
            break;
        }
        zones_cliquables.push((
            Rect::new(
                chunks[2].left() + 1,
                ligne_y,
                chunks[2].width.saturating_sub(2),
                1,
            ),
            ActionClique::BudgetLigne(i),
        ));
    }

    // Bande de commentaire de la ligne sélectionnée, sous la grille : la
    // note telle qu'elle vit dans le ledger (lignes `;`), ou le prompt de
    // saisie pendant l'édition. Une seule ligne : rien ne bouge dans la
    // grille elle-même (largeurs préservées).
    let ligne_active = grille.rows.get(ligne_idx);
    if let Some(ed) = etat.edition_commentaire.as_ref() {
        // Saisie en cours : le champ prend la bande, avec caret, en bleu.
        let bande = Paragraph::new(format!(
            " 💬 {}  (Entrée : valider, Esc : annuler) ",
            ed.input.ligne_avec_curseur()
        ))
        .style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(bande, chunks[3]);
    } else {
        let note = ligne_active.and_then(|l| commentaires.get(l.account.as_str()));
        let texte = match note {
            Some(n) => format!(" 💬 «{n}»  (c : éditer) "),
            None => " 💬 aucun commentaire  (c : en ajouter) ".to_string(),
        };
        let bande = Paragraph::new(texte).style(Style::default().fg(Color::Gray));
        f.render_widget(bande, chunks[3]);
    }

    None
}

/// Gère un événement clavier pour le budget. `commentaires` est la map
/// compte → note de l'année affichée (pré-remplissage de l'éditeur).
/// `rollover_actif` indique si le rollover est actif pour une famille (affichage futur).
pub fn on_key(
    key: KeyEvent,
    etat: &mut UiState,
    grille: Option<&BudgetGrid>,
    commentaires: &BTreeMap<String, String>,
    _rollover_actif: &dyn Fn(&str) -> bool,
) -> Option<ActionBudget> {
    use crossterm::event::KeyCode;

    // Édition du commentaire ouverte : toutes les touches y vont (le texte
    // peut contenir r, l, e, x…). Entrée valide (texte nettoyé, vide =
    // suppression), Esc annule sans écriture.
    if let Some(ed) = &mut etat.edition_commentaire {
        if ed.input.on_key(key) {
            return None;
        }
        return match key.code {
            KeyCode::Enter => {
                let Some(grille) = grille else {
                    etat.edition_commentaire = None;
                    return None;
                };
                let Some(ligne) = grille.rows.get(ed.ligne) else {
                    etat.edition_commentaire = None;
                    return None;
                };
                let action = ActionBudget::Commentaire {
                    annee: etat.annee,
                    compte: ligne.account.clone(),
                    texte: household_budget::nettoie(&ed.input.value),
                };
                etat.edition_commentaire = None;
                Some(action)
            }
            KeyCode::Esc => {
                etat.edition_commentaire = None;
                None
            }
            _ => None,
        };
    }

    // Édition en cours : toutes les touches y vont (un montant peut
    // contenir r, l, x…). Entrée valide, Esc annule sans écriture ; un
    // montant invalide garde l'édition ouverte (la cellule est rendue en
    // rouge). Tant que l'édition n'est pas validée, AUCUNE écriture n'a
    // lieu — c'est la régression du comportement « 'e' réécrivait la
    // cellule avec sa valeur existante ».
    if let Some(edition) = &mut etat.edition {
        if edition.input.on_key(key) {
            return None;
        }
        return match key.code {
            KeyCode::Enter => {
                let Some(grille) = grille else {
                    etat.edition = None;
                    return None;
                };
                let Some(ligne) = grille.rows.get(edition.ligne) else {
                    etat.edition = None;
                    return None;
                };
                // Virgule décimale acceptée (saisie française) ; le parse
                // est un `Decimal` de bout en bout, jamais de f64.
                let texte = edition.input.value.trim().replace(',', ".");
                let Ok(montant) = texte.parse::<Decimal>() else {
                    return None;
                };
                let Some(date) =
                    NaiveDate::new(etat.annee, i8::try_from(edition.mois + 1).unwrap_or(1), 1).ok()
                else {
                    etat.edition = None;
                    return None;
                };
                let action = ActionBudget::Cellule {
                    annee: etat.annee,
                    date,
                    compte: ligne.account.clone(),
                    montant,
                };
                etat.edition = None;
                Some(action)
            }
            KeyCode::Esc => {
                etat.edition = None;
                None
            }
            _ => None,
        };
    }

    match key.code {
        KeyCode::Up => {
            etat.ligne_precedente();
            None
        }
        KeyCode::Down => {
            etat.ligne_suivante(grille);
            None
        }
        KeyCode::PageUp => {
            let nb_total = grille.map_or(0, |g| g.rows.len());
            let delta = -(etat.visibles.max(1) as isize);
            etat.defiler(nb_total, delta);
            // Ajuste la sélection pour qu'elle reste dans la fenêtre visible.
            if etat.ligne_selectionne < etat.defilement {
                etat.ligne_selectionne = etat.defilement;
            }
            None
        }
        KeyCode::PageDown => {
            let nb_total = grille.map_or(0, |g| g.rows.len());
            let delta = etat.visibles.max(1) as isize;
            etat.defiler(nb_total, delta);
            if etat.ligne_selectionne >= etat.defilement + etat.visibles {
                etat.ligne_selectionne = (etat.defilement + etat.visibles)
                    .min(nb_total)
                    .saturating_sub(1);
            }
            None
        }
        KeyCode::Left => {
            etat.mois_precedent();
            None
        }
        KeyCode::Right => {
            etat.mois_suivant();
            None
        }
        KeyCode::Char('c') => {
            // Ouvre l'éditeur de commentaire sur la ligne sélectionnée,
            // pré-rempli avec la note existante (lecture depuis le ledger).
            // Aucune écriture à ce stade : la commande ne part qu'à la
            // validation (Entrée).
            let grille = grille?;
            if grille.rows.is_empty() {
                return None;
            }
            let ligne_idx = etat.ligne_index(Some(grille));
            let compte = &grille.rows[ligne_idx].account;
            let mut input =
                TextInput::with_value(commentaires.get(compte).cloned().unwrap_or_default());
            input.set_focused(true);
            etat.edition_commentaire = Some(EditionCommentaire {
                ligne: ligne_idx,
                input,
            });
            None
        }
        KeyCode::Char('e') => {
            // Ouvre l'éditeur sur la cellule sélectionnée, pré-rempli avec
            // la valeur courante. Aucune écriture à ce stade : la commande
            // ne part qu'à la validation (Entrée).
            let grille = grille?;
            if grille.rows.is_empty() {
                return None;
            }
            let ligne_idx = etat.ligne_index(Some(grille));
            let mois = etat.mois_selectionne;
            let valeur = grille.rows[ligne_idx].cells[mois].budgeted;
            let mut input = TextInput::with_value(valeur.to_string());
            input.set_focused(true);
            etat.edition = Some(EditionCellule {
                ligne: ligne_idx,
                mois,
                input,
            });
            None
        }
        KeyCode::Char('g') => {
            // Générer l'année suivante (placeholder — nécessite une UI de dialogue).
            None
        }
        _ => None,
    }
}

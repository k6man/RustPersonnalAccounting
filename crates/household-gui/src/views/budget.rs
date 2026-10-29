//! Écran « Budget » : grille postes × 12 mois éditable (writer chirurgical
//! du jalon 3), arbre hiérarchique repliable (groupes synthétisés),
//! rollover par famille (préférences en TOML local — TODO Q6), duplication
//! d'année, totaux.
//!
//! Présentation pure : helpers hiérarchie/rollover dans [`crate::budget`],
//! calculs dans `household_budget`.

use std::collections::BTreeSet;

use egui::{Color32, RichText};
use egui_extras::{Column, TableBuilder};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use household_budget::{BudgetGrid, GridCell};

use crate::budget as logique;
use crate::fmt;

const MOIS: [&str; 12] = [
    "janv", "févr", "mars", "avr", "mai", "juin", "juil", "août", "sept", "oct", "nov", "déc",
];

const ROUGE: Color32 = Color32::from_rgb(0xd8, 0x3b, 0x2f);

/// État d'interface de l'éditeur de budget.
#[derive(Debug, Clone)]
pub struct UiState {
    /// Année affichée.
    pub annee: i16,
    /// Familles repliées (tout est déplié par défaut).
    pub repliees: BTreeSet<String>,
    /// Cellule en cours d'édition (le diff se confirme sous la grille).
    pub edition: Option<EditionCellule>,
    /// Dialogue de duplication d'année (ouvert ?).
    pub duplication: Option<DialogueDuplication>,
}

/// Une cellule en cours d'édition.
#[derive(Debug, Clone)]
pub struct EditionCellule {
    pub compte: String,
    pub mois: u8,
    pub ancien: Decimal,
    pub tampon: String,
}

impl EditionCellule {
    /// Le montant actuellement saisi (tolérant : virgule, vide = inchangé).
    #[must_use]
    pub fn montant_saisi(&self) -> Decimal {
        self.tampon
            .trim()
            .replace(',', ".")
            .parse()
            .unwrap_or(self.ancien)
    }
}

/// Dialogue « générer l'année N+1 ».
#[derive(Debug, Clone)]
pub struct DialogueDuplication {
    pub source: i16,
    pub cible: i16,
    /// Ajustement en pour cent (saisie flottante d'UI).
    pub pourcent: f64,
    /// Pas d'arrondi : 0 = centimes, 1 = euros entiers, 2 = multiples de 5 €.
    pub pas: usize,
}

impl DialogueDuplication {
    /// Pas en Decimal selon le choix de l'UI.
    #[must_use]
    pub fn pas_decimal(&self) -> Decimal {
        match self.pas {
            1 => dec!(1.00),
            2 => dec!(5.00),
            _ => dec!(0.01),
        }
    }
}

impl UiState {
    #[must_use]
    pub fn nouveau(annee: i16) -> Self {
        Self {
            annee,
            repliees: BTreeSet::new(),
            edition: None,
            duplication: None,
        }
    }
}

/// Action demandée par l'écran vers l'application.
#[derive(Debug, Clone)]
pub enum ActionBudget {
    /// Confirmer l'édition de la cellule en cours.
    Cellule {
        annee: i16,
        date: rustledger_core::NaiveDate,
        compte: String,
        montant: Decimal,
    },
    /// Générer l'année cible depuis la source.
    GenererAnnee {
        source: i16,
        cible: i16,
        pourcent: Decimal,
        pas: Decimal,
    },
    /// Basculer le rollover d'une famille (persistance côté app).
    BasculerRollover(String),
}

/// Dessine l'éditeur de budget. `grille` = grille de l'année sélectionnée
/// (absente si l'année n'est pas encore budgétée).
pub fn afficher(
    ui: &mut egui::Ui,
    grille: Option<&BudgetGrid>,
    etat: &mut UiState,
    rollover_actif: impl Fn(&str) -> bool,
) -> Option<ActionBudget> {
    let mut action = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // --- Sélecteur d'année + génération ------------------------------
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Budget {}", etat.annee))
                    .heading()
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("◀").clicked() {
                    etat.annee -= 1;
                    etat.edition = None;
                }
                if ui.button("▶").clicked() {
                    etat.annee += 1;
                    etat.edition = None;
                }
                if ui.button(format!("Générer {}…", etat.annee + 1)).clicked() {
                    etat.duplication = Some(DialogueDuplication {
                        source: etat.annee,
                        cible: etat.annee + 1,
                        pourcent: 0.0,
                        pas: 0,
                    });
                }
            });
        });
        ui.add_space(4.0);

        // --- Dialogue de duplication --------------------------------------
        if etat.duplication.is_some() {
            let mut generer = None;
            let mut annuler = false;
            if let Some(dialogue) = &mut etat.duplication {
                ui.group(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Générer {} à partir de {}",
                            dialogue.cible, dialogue.source
                        ))
                        .strong(),
                    );
                    ui.horizontal(|ui| {
                        ui.label("Ajustement :");
                        ui.add(
                            egui::DragValue::new(&mut dialogue.pourcent)
                                .speed(0.5)
                                .suffix(" %"),
                        );
                        ui.label("Arrondi :");
                        if ui.selectable_label(dialogue.pas == 0, "centimes").clicked() {
                            dialogue.pas = 0;
                        }
                        if ui.selectable_label(dialogue.pas == 1, "1 €").clicked() {
                            dialogue.pas = 1;
                        }
                        if ui.selectable_label(dialogue.pas == 2, "5 €").clicked() {
                            dialogue.pas = 2;
                        }
                    });
                    ui.label(
                        RichText::new(
                            "Crée budgets/<année>.beancount + son include dans main ; \
                             refuse d'écraser une année existante.",
                        )
                        .weak()
                        .small(),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Générer").clicked() {
                            generer = Some((
                                dialogue.source,
                                dialogue.cible,
                                Decimal::try_from(dialogue.pourcent).unwrap_or_default(),
                                dialogue.pas_decimal(),
                            ));
                        }
                        if ui.button("Annuler").clicked() {
                            annuler = true;
                        }
                    });
                });
            }
            if let Some((source, cible, pourcent, pas)) = generer {
                action = Some(ActionBudget::GenererAnnee {
                    source,
                    cible,
                    pourcent,
                    pas,
                });
                etat.duplication = None;
            }
            if annuler {
                etat.duplication = None;
            }
            ui.add_space(6.0);
        }

        // --- La grille -----------------------------------------------------
        match grille {
            Some(grille) => {
                table_grille(ui, grille, etat, &rollover_actif, &mut action);
                totaux(ui, grille);
            }
            None => {
                ui.add_space(8.0);
                ui.label(format!(
                    "Aucun budget pour {}. Utilisez « Générer {}… » \
                     (duplication d'une année existante).",
                    etat.annee,
                    etat.annee + 1
                ));
            }
        }

        // --- Barre de confirmation d'édition (diff attendu) ----------------
        if let Some(edition) = &etat.edition {
            ui.add_space(8.0);
            if barre_confirmation(ui, edition, etat.annee, &mut action) {
                etat.edition = None;
            }
        }
    });

    action
}

/// La grille postes × 12 mois (arbre hiérarchique synthétisé).
fn table_grille(
    ui: &mut egui::Ui,
    grille: &BudgetGrid,
    etat: &mut UiState,
    rollover_actif: &impl Fn(&str) -> bool,
    action: &mut Option<ActionBudget>,
) {
    // Arbre (groupes synthétisés) + visibilité + lignes de tête (rollover).
    let arbre = logique::arbre(&grille.rows);
    let visibles = logique::visibles(&arbre, &etat.repliees);
    let tetes: BTreeSet<usize> = logique::tetes(&arbre).into_iter().collect();

    let mut table = TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::remainder().resizable(true))
        .column(Column::exact(56.0)) // rollover
        .column(Column::exact(72.0)); // année
    for _ in 0..12 {
        table = table.column(Column::exact(76.0));
    }
    table
        .header(22.0, |mut en_tete| {
            en_tete.col(|ui| {
                ui.strong("Poste");
            });
            en_tete.col(|ui| {
                ui.strong("Roul.");
            });
            en_tete.col(|ui| {
                ui.strong("Année");
            });
            for mois in MOIS {
                en_tete.col(|ui| {
                    ui.strong(mois);
                });
            }
        })
        .body(|mut corps| {
            for &index in &visibles {
                let noeud = &arbre[index];
                let est_tete = tetes.contains(&index);
                let rollover_on = est_tete && rollover_actif(&noeud.compte);

                corps.row(32.0, |mut rangee| {
                    // Poste (indenté, triangle si enfants) + note épargne.
                    rangee.col(|ui| {
                        ui.horizontal(|ui| {
                            for _ in 0..noeud.profondeur {
                                ui.label("    ");
                            }
                            if noeud.a_enfants {
                                let ouvert = !etat.repliees.contains(&noeud.compte);
                                if ui
                                    .selectable_label(false, if ouvert { "▾" } else { "▸" })
                                    .clicked()
                                {
                                    if ouvert {
                                        etat.repliees.insert(noeud.compte.clone());
                                    } else {
                                        etat.repliees.remove(&noeud.compte);
                                    }
                                }
                            }
                            let nom = if noeud.budgete {
                                RichText::new(noeud.compte.as_str())
                            } else {
                                RichText::new(noeud.compte.as_str()).weak()
                            };
                            ui.label(nom);
                        });
                        if logique::note_epargne(&noeud.compte).is_some() {
                            ui.label(RichText::new("intérêts inclus dans le réel").weak().small());
                        }
                    });

                    // Toggle rollover (lignes de tête seulement ; l'état réel
                    // vient de la config — rechargée après bascule).
                    rangee.col(|ui| {
                        if est_tete {
                            let mut affiche = rollover_on;
                            if ui.checkbox(&mut affiche, "").clicked() {
                                *action =
                                    Some(ActionBudget::BasculerRollover(noeud.compte.clone()));
                            }
                        }
                    });

                    // Total annuel (budget, ou report sortant en rollover).
                    rangee.col(|ui| {
                        let texte = if rollover_on {
                            fmt::nombre(logique::rollover_de(noeud).carry_out())
                        } else {
                            fmt::nombre(noeud.cellules.iter().map(|c| c.budgeted).sum())
                        };
                        ui.label(RichText::new(texte).strong());
                    });

                    // Les 12 cellules.
                    for (mois_zero, cellule) in noeud.cellules.iter().enumerate() {
                        rangee.col(|ui| {
                            cellule_budget(ui, noeud, cellule, mois_zero, rollover_on, etat);
                        });
                    }
                });
            }
        });
}

/// Une cellule : budget cliquable (édition, postes budgétés seulement),
/// réel dessous coloré si dépassement — ou disponible cumulé en rollover.
/// Les groupes synthétisés sont en lecture seule (l'édition se fait sur les
/// postes qui portent les directives).
fn cellule_budget(
    ui: &mut egui::Ui,
    noeud: &logique::NoeudBudget,
    cellule: &GridCell,
    mois_zero: usize,
    rollover_on: bool,
    etat: &mut UiState,
) {
    ui.vertical(|ui| {
        let en_edition = etat
            .edition
            .as_ref()
            .is_some_and(|e| e.compte == noeud.compte && usize::from(e.mois) == mois_zero + 1);

        if en_edition {
            champ_edition(ui, etat);
        } else if noeud.budgete {
            // Budget du mois — ou disponible cumulé (rollover).
            let (texte, couleur) = if rollover_on {
                let dispo = logique::disponible(noeud, mois_zero);
                (
                    fmt::nombre(dispo),
                    if dispo < Decimal::ZERO {
                        ROUGE
                    } else {
                        ui.visuals().text_color()
                    },
                )
            } else if cellule.budgeted.is_zero() {
                ("·".to_string(), ui.visuals().weak_text_color())
            } else {
                (fmt::nombre(cellule.budgeted), ui.visuals().text_color())
            };
            if ui.button(RichText::new(texte).color(couleur)).clicked() {
                etat.edition = Some(EditionCellule {
                    compte: noeud.compte.clone(),
                    mois: u8::try_from(mois_zero + 1).unwrap_or(1),
                    ancien: cellule.budgeted,
                    tampon: cellule.budgeted.to_string(),
                });
            }
        } else {
            // Groupe synthétisé : agrégat en lecture seule.
            let texte = if rollover_on {
                fmt::nombre(logique::disponible(noeud, mois_zero))
            } else if cellule.budgeted.is_zero() {
                "·".to_string()
            } else {
                fmt::nombre(cellule.budgeted)
            };
            ui.label(RichText::new(texte).weak());
        }

        // Réel du mois dessous (masqué en rollover : le cumul le remplace).
        if !rollover_on {
            let depasse = cellule.actual > cellule.budgeted
                || (cellule.budgeted.is_zero() && !cellule.actual.is_zero());
            let couleur = if depasse {
                ROUGE
            } else {
                ui.visuals().weak_text_color()
            };
            ui.label(
                RichText::new(if cellule.actual.is_zero() {
                    "–".to_string()
                } else {
                    fmt::nombre(cellule.actual)
                })
                .small()
                .color(couleur),
            );
        }
    });
}

/// Champ d'édition de la cellule (la confirmation passe par la barre de diff).
fn champ_edition(ui: &mut egui::Ui, etat: &mut UiState) {
    let Some(edition) = &mut etat.edition else {
        return;
    };
    ui.add(
        egui::TextEdit::singleline(&mut edition.tampon)
            .desired_width(64.0)
            .hint_text("0.00"),
    );
}

/// Barre de confirmation : le diff attendu avant écriture. Retourne
/// `true` quand l'édition est terminée (confirmée ou annulée).
fn barre_confirmation(
    ui: &mut egui::Ui,
    edition: &EditionCellule,
    annee: i16,
    action: &mut Option<ActionBudget>,
) -> bool {
    let mut terminee = false;
    ui.group(|ui| {
        let nouveau = edition.montant_saisi();
        ui.label(
            RichText::new(format!(
                "{} — {} : {} → {}",
                edition.compte,
                MOIS[usize::from(edition.mois) - 1],
                fmt::eur(edition.ancien),
                fmt::eur(nouveau),
            ))
            .strong(),
        );
        ui.label(
            RichText::new(
                "Une directive est modifiée si elle existe au 1er de ce mois, \
                 sinon une supersession est ajoutée (valable à partir de ce mois). \
                 Le ledger est revalidé sur copie avant l'écriture.",
            )
            .weak()
            .small(),
        );
        ui.horizontal(|ui| {
            if ui.button("Confirmer").clicked() {
                *action = Some(ActionBudget::Cellule {
                    annee,
                    date: rustledger_core::naive_date(i32::from(annee), u32::from(edition.mois), 1)
                        .expect("1er du mois"),
                    compte: edition.compte.clone(),
                    montant: nouveau,
                });
                terminee = true;
            }
            if ui.button("Annuler").clicked() {
                terminee = true;
            }
        });
    });
    terminee
}

/// Totaux par mois (chaque budget compté une fois) + total de l'année.
fn totaux(ui: &mut egui::Ui, grille: &BudgetGrid) {
    ui.add_space(6.0);
    ui.label(RichText::new("Totaux").strong());
    ui.horizontal_wrapped(|ui| {
        for (mois_zero, mois) in MOIS.iter().enumerate() {
            let budgete = logique::total_mois(&grille.totals, mois_zero);
            ui.label(format!("{} : {}", mois, fmt::eur(budgete)));
            ui.separator();
        }
        let annee: Decimal = (0..12)
            .map(|m| logique::total_mois(&grille.totals, m))
            .sum();
        ui.label(RichText::new(format!("Année : {}", fmt::eur(annee))).strong());
    });
}

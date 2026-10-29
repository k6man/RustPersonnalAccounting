//! Écran « Journal » : tableau paginé filtrable + détail de l'écriture
//! sélectionnée (avec son origine fichier:ligne).
//!
//! Présentation pure : filtres/pagination = [`crate::filters`] (pur), les
//! données = [`crate::viewmodel::JournalData`].

use egui::RichText;
use egui_extras::{Column, TableBuilder};

use crate::filters::{Pagination, date_optionnelle, montant_optionnel};
use crate::fmt;
use crate::viewmodel::{JournalData, LigneJournal};

/// Action demandée par l'écran journal vers l'application (jalon 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionJournal {
    /// Ouvrir la transaction dans l'écran Saisie (mode modification).
    Modifier(usize),
    /// Supprimer la transaction (confirmée par double clic côté vue).
    Supprimer(usize),
}

/// État d'interface du journal (saisies + sélection) — pas de logique métier.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub texte: String,
    pub compte: String,
    pub tag: String,
    pub date_min: String,
    pub date_max: String,
    pub montant_min: String,
    pub montant_max: String,
    pub page: usize,
    pub par_page: usize,
    pub selection: Option<usize>,
    /// Suppression en attente de confirmation (index de l'entrée).
    pub suppression_confirmee: Option<usize>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            par_page: 100,
            ..Self::default()
        }
    }
}

/// Dessine l'écran journal. Retourne l'action demandée (jalon 5).
pub fn afficher(
    ui: &mut egui::Ui,
    journal: &JournalData,
    etat: &mut UiState,
) -> Option<ActionJournal> {
    let mut action = None;
    // 1. Filtres (saisie → parsing pur).
    panneau_filtres(ui, etat);

    // 2. Filtrage (fonctions pures testées).
    let filtre = crate::filters::JournalFilter {
        texte: etat.texte.clone(),
        compte: etat.compte.clone(),
        tag: etat.tag.clone(),
        date_min: date_optionnelle(&etat.date_min),
        date_max: date_optionnelle(&etat.date_max),
        montant_min: montant_optionnel(&etat.montant_min),
        montant_max: montant_optionnel(&etat.montant_max),
    };
    let lignes: Vec<&LigneJournal> = journal
        .lignes
        .iter()
        .filter(|l| filtre.correspond(l))
        .collect();

    // Bandeau d'état des filtres (+ réinitialisation).
    if filtre.actif() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Filtres actifs").weak());
            if ui.small_button("Réinitialiser").clicked() {
                *etat = UiState::new();
            }
        });
    }

    // 3. Pagination bornée au volume filtré.
    let mut pagination = Pagination {
        page: etat.page,
        par_page: etat.par_page,
    };
    pagination.borner(lignes.len());
    etat.page = pagination.page;
    let (debut, fin) = pagination.tranche(lignes.len());
    let pages = pagination.pages(lignes.len());

    // 4. Tableau.
    let table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto().resizable(true)) // date
        .column(Column::exact(24.0)) // !
        .column(Column::auto().resizable(true)) // tiers
        .column(Column::remainder()) // libellé
        .column(Column::auto()) // comptes
        .column(Column::auto()) // montant
        .column(Column::auto()); // tags
    table
        .header(22.0, |mut en_tete| {
            en_tete.col(|ui| {
                ui.strong("Date");
            });
            en_tete.col(|ui| {
                ui.strong("!");
            });
            en_tete.col(|ui| {
                ui.strong("Tiers");
            });
            en_tete.col(|ui| {
                ui.strong("Libellé");
            });
            en_tete.col(|ui| {
                ui.strong("Comptes");
            });
            en_tete.col(|ui| {
                ui.strong("Montant");
            });
            en_tete.col(|ui| {
                ui.strong("Tags");
            });
        })
        .body(|mut corps| {
            for ligne in lignes.iter().skip(debut).take(fin - debut) {
                corps.row(20.0, |mut rangee| {
                    let selectionnee = etat.selection.is_some_and(|s| s == ligne.index);
                    rangee.col(|ui| {
                        etiquette_selectionnable(
                            ui,
                            selectionnee,
                            RichText::new(ligne.date.to_string()).monospace(),
                        );
                    });
                    rangee.col(|ui| {
                        if ligne.flag == '!' {
                            ui.label(
                                RichText::new("!").color(egui::Color32::from_rgb(0xe2, 0x93, 0x1b)),
                            );
                        }
                    });
                    rangee.col(|ui| {
                        etiquette_selectionnable(
                            ui,
                            selectionnee,
                            ligne.tiers.as_deref().unwrap_or(""),
                        );
                    });
                    rangee.col(|ui| {
                        etiquette_selectionnable(ui, selectionnee, &ligne.libelle);
                    });
                    rangee.col(|ui| {
                        etiquette_selectionnable(ui, selectionnee, &ligne.categories);
                    });
                    rangee.col(|ui| {
                        let texte = ligne
                            .montant
                            .map(fmt::eur)
                            .unwrap_or_else(|| "…".to_string());
                        etiquette_selectionnable(ui, selectionnee, RichText::new(texte).strong());
                    });
                    rangee.col(|ui| {
                        if !ligne.tags.is_empty() {
                            etiquette_selectionnable(
                                ui,
                                selectionnee,
                                format!("#{}", ligne.tags.join(" #")),
                            );
                        }
                    });
                });
            }
            if lignes.is_empty() {
                corps.row(24.0, |mut rangee| {
                    rangee.col(|ui| {
                        ui.label("Aucune écriture pour ces filtres.");
                    });
                    for _ in 0..6 {
                        rangee.col(|_ui| {});
                    }
                });
            }
        });

    // 5. Contrôles de pagination.
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let active = etat.page > 0;
        if ui
            .add_enabled(active, egui::Button::new("◀ Précédent"))
            .clicked()
        {
            etat.page -= 1;
        }
        ui.label(format!(
            "page {} / {} — {} écritures affichées sur {}",
            pages.min(etat.page + 1),
            pages,
            fin - debut,
            lignes.len()
        ));
        if ui
            .add_enabled(etat.page + 1 < pages, egui::Button::new("Suivant ▶"))
            .clicked()
        {
            etat.page += 1;
        }
    });

    // 6. Détail de l'écriture sélectionnée + actions d'édition (jalon 5).
    if let Some(index) = etat.selection
        && let Some(ligne) = journal.lignes.iter().find(|l| l.index == index)
    {
        ui.add_space(8.0);
        panneau_detail(ui, ligne);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("✎ Modifier").clicked() {
                action = Some(ActionJournal::Modifier(index));
            }
            // Suppression en deux clics (confirmation locale).
            if etat.suppression_confirmee == Some(index) {
                if ui
                    .button(
                        RichText::new("⚠ Confirmer la suppression")
                            .color(egui::Color32::from_rgb(0xd8, 0x3b, 0x2f)),
                    )
                    .clicked()
                {
                    etat.suppression_confirmee = None;
                    action = Some(ActionJournal::Supprimer(index));
                }
                if ui.button("Annuler").clicked() {
                    etat.suppression_confirmee = None;
                }
            } else if ui.button("Supprimer").clicked() {
                etat.suppression_confirmee = Some(index);
            }
        });
    }

    action
}

/// Libellé cliquable qui marque la sélection (plus fort quand sélectionnée).
fn etiquette_selectionnable(
    ui: &mut egui::Ui,
    selectionnee: bool,
    texte: impl Into<egui::WidgetText>,
) {
    let mut texte_widget: egui::WidgetText = texte.into();
    if selectionnee {
        texte_widget = texte_widget.strong();
    }
    ui.add(egui::Label::new(texte_widget));
}

/// Les comptes de catégorisation viennent du view-model (`ligne.categories`).
fn panneau_filtres(ui: &mut egui::Ui, etat: &mut UiState) {
    egui::Grid::new("filtres-journal")
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Recherche :");
            ui.add(
                egui::TextEdit::singleline(&mut etat.texte)
                    .desired_width(180.0)
                    .hint_text("tiers, libellé…"),
            );
            ui.label("Compte :");
            ui.add(
                egui::TextEdit::singleline(&mut etat.compte)
                    .desired_width(200.0)
                    .hint_text("Expenses:…"),
            );
            ui.label("Tag :");
            ui.add(
                egui::TextEdit::singleline(&mut etat.tag)
                    .desired_width(110.0)
                    .hint_text("#courses"),
            );
            ui.end_row();

            ui.label("Du :");
            champ_date(ui, &mut etat.date_min);
            ui.label("au :");
            champ_date(ui, &mut etat.date_max);
            ui.label("Montant ≥ :");
            ui.add(egui::TextEdit::singleline(&mut etat.montant_min).desired_width(90.0));
            ui.label("≤ :");
            ui.add(egui::TextEdit::singleline(&mut etat.montant_max).desired_width(90.0));
            ui.end_row();
        });
    ui.add_space(2.0);
    // Indices de saisie (dates invalides en rouge).
    let dates_invalides = (!etat.date_min.trim().is_empty()
        && date_optionnelle(&etat.date_min).is_none())
        || (!etat.date_max.trim().is_empty() && date_optionnelle(&etat.date_max).is_none());
    if dates_invalides {
        ui.colored_label(
            egui::Color32::from_rgb(0xd8, 0x3b, 0x2f),
            "Date invalide (format attendu AAAA-MM-JJ) — filtre ignoré",
        );
    }
}

fn champ_date(ui: &mut egui::Ui, valeur: &mut String) {
    let valide = valeur.trim().is_empty() || date_optionnelle(valeur).is_some();
    let champ = egui::TextEdit::singleline(valeur)
        .desired_width(105.0)
        .hint_text("AAAA-MM-JJ");
    if valide {
        ui.add(champ);
    } else {
        ui.add(champ.text_color(egui::Color32::from_rgb(0xd8, 0x3b, 0x2f)));
    }
}

fn panneau_detail(ui: &mut egui::Ui, ligne: &LigneJournal) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(ligne.date.to_string()).monospace().strong());
                if let Some(tiers) = &ligne.tiers {
                    ui.label(RichText::new(tiers).strong());
                }
                ui.label(&ligne.libelle);
                if ligne.flag == '!' {
                    ui.colored_label(egui::Color32::from_rgb(0xe2, 0x93, 0x1b), "en attente (!)");
                }
            });
            egui::Grid::new("detail-postings")
                .spacing([16.0, 2.0])
                .show(ui, |ui| {
                    for posting in &ligne.lignes_posting {
                        ui.label(RichText::new(posting.compte.as_str()).monospace());
                        match posting.montant {
                            Some((montant, ref devise)) => {
                                let texte = format!("{} {}", fmt::nombre(montant), devise);
                                ui.label(RichText::new(texte).monospace().strong());
                            }
                            None => {
                                ui.label(RichText::new("— (équilibrage)").weak().monospace());
                            }
                        }
                        ui.end_row();
                    }
                });
            ui.horizontal(|ui| {
                if !ligne.tags.is_empty() {
                    ui.label(format!("#{}", ligne.tags.join(" #")));
                }
                if !ligne.liens.is_empty() {
                    ui.label(format!("^{}", ligne.liens.join(" ^")));
                }
            });
            ui.label(
                RichText::new(format!(
                    "source : {}:{} (entrée #{})",
                    ligne.fichier, ligne.ligne, ligne.index
                ))
                .weak()
                .small(),
            );
        });
    });
}

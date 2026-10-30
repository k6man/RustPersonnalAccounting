//! Écran « Rapports » : dépenses par catégorie × mois (heatmap), comparaison
//! N vs N−1 (barres egui_plot), revenus, épargne, taux d'épargne.
//!
//! Présentation pure : tous les agrégats viennent de
//! [`crate::viewmodel::RapportsData`] (Decimal) ; la conversion `f64`
//! n'existe qu'aux frontières du tracé.

use std::collections::BTreeMap;

use egui::RichText;
use egui_plot::{Bar, BarChart, Plot};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::fmt;
use crate::viewmodel::RapportsData;

const MOIS: [&str; 12] = [
    "janv", "févr", "mars", "avr", "mai", "juin", "juil", "août", "sept", "oct", "nov", "déc",
];

/// Dessine l'écran Rapports.
pub fn afficher(ui: &mut egui::Ui, rapports: &RapportsData) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(
            RichText::new(format!("Rapports {}", rapports.annee))
                .heading()
                .strong(),
        );
        ui.add_space(4.0);

        // --- Synthèse de l'année ------------------------------------------
        ui.horizontal_wrapped(|ui| {
            synthese(ui, "Dépenses", fmt::eur(rapports.depenses_annee()));
            ui.separator();
            synthese(ui, "Revenus", fmt::eur(rapports.revenus_annee()));
            ui.separator();
            synthese(ui, "Épargne", fmt::eur(rapports.epargne_annee()));
            ui.separator();
            let taux = rapports.taux_epargne().map_or_else(
                || "n/a".to_string(),
                |t| fmt::pourcent(t.to_f64().unwrap_or(0.0)),
            );
            synthese(ui, "Taux d'épargne", taux);
        });
        ui.add_space(10.0);

        // --- Comparaison N vs N−1 (barres) --------------------------------
        if let Some(precedente) = &rapports.depenses_annee_precedente {
            barres_comparaison(ui, rapports, precedente);
        } else {
            ui.label(
                RichText::new(format!(
                    "Pas de transactions en {} : pas de comparaison N−1.",
                    rapports.annee - 1
                ))
                .weak(),
            );
        }
        ui.add_space(10.0);

        // --- Heatmap catégories × mois ------------------------------------
        heatmap(ui, &rapports.depenses_categories, &rapports.depenses_mois);
    });
}

/// Une case de synthèse (étiquette en petit, valeur en gros).
fn synthese(ui: &mut egui::Ui, etiquette: &str, valeur: String) {
    ui.vertical(|ui| {
        ui.label(RichText::new(etiquette).weak().small());
        ui.label(RichText::new(valeur).strong());
    });
}

/// Comparaison des dépenses mensuelles N vs N−1.
fn barres_comparaison(ui: &mut egui::Ui, rapports: &RapportsData, precedente: &[Decimal; 12]) {
    let en_eur = |montant: Decimal| montant.to_f64().unwrap_or(0.0);
    let courante = BarChart::new(
        rapports.annee.to_string(),
        (0..12)
            .map(|m| {
                Bar::new(m as f64, en_eur(rapports.depenses_mois[m])).name(MOIS[m].to_string())
            })
            .collect(),
    )
    .color(egui::Color32::from_rgb(0xc3, 0x5a, 0x3f));
    let ancienne = BarChart::new(
        (rapports.annee - 1).to_string(),
        (0..12)
            .map(|m| Bar::new(m as f64, en_eur(precedente[m])).name(MOIS[m].to_string()))
            .collect(),
    )
    .color(egui::Color32::from_rgb(0xa0, 0xa0, 0xa0));

    Plot::new("comparaison-depenses")
        .legend(egui_plot::Legend::default())
        .x_axis_formatter(|mark, _plage| {
            MOIS.get(mark.value.round() as usize)
                .map_or_else(String::new, |m| (*m).to_string())
        })
        .show(ui, |trace| {
            trace.bar_chart(ancienne);
            trace.bar_chart(courante);
        });
}

/// Tableau catégories × mois, chaque cellule colorée selon son poids
/// relatif (le maximum de la ligne sert d'intensité).
fn heatmap(
    ui: &mut egui::Ui,
    categories: &BTreeMap<String, [Decimal; 12]>,
    totaux: &[Decimal; 12],
) {
    ui.label(RichText::new("Dépenses par catégorie et par mois").strong());
    egui_extras::TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(egui_extras::Column::remainder())
        .columns(egui_extras::Column::auto().at_least(56.0), 12)
        .column(egui_extras::Column::auto().at_least(72.0))
        .header(22.0, |mut en_tete| {
            en_tete.col(|ui| {
                ui.strong("Catégorie");
            });
            for mois in MOIS {
                en_tete.col(|ui| {
                    ui.strong(mois);
                });
            }
            en_tete.col(|ui| {
                ui.strong("Année");
            });
        })
        .body(|mut corps| {
            for (categorie, cellules) in categories {
                let total_ligne: Decimal = cellules.iter().copied().sum();
                let maximum = cellules.iter().copied().max().unwrap_or(Decimal::ZERO);
                corps.row(22.0, |mut rangee| {
                    rangee.col(|ui| {
                        ui.label(categorie.as_str());
                    });
                    for cellule in cellules {
                        rangee.col(|ui| {
                            cellule_chaleur(ui, *cellule, maximum);
                        });
                    }
                    rangee.col(|ui| {
                        ui.label(RichText::new(fmt::nombre(total_ligne)).strong());
                    });
                });
            }
            // Ligne des totaux mensuels.
            let total_annee: Decimal = totaux.iter().copied().sum();
            corps.row(22.0, |mut rangee| {
                rangee.col(|ui| {
                    ui.strong("TOTAL");
                });
                for total in totaux {
                    rangee.col(|ui| {
                        ui.label(RichText::new(fmt::nombre(*total)).strong());
                    });
                }
                rangee.col(|ui| {
                    ui.label(RichText::new(fmt::nombre(total_annee)).strong());
                });
            });
        });
}

/// Une cellule de heatmap : montant coloré selon l'intensité relative.
fn cellule_chaleur(ui: &mut egui::Ui, montant: Decimal, maximum: Decimal) {
    if montant.is_zero() {
        ui.label(RichText::new("·").weak());
        return;
    }
    let intensite = if maximum.is_zero() {
        0.0_f32
    } else {
        (montant / maximum).to_f64().unwrap_or(0.0).clamp(0.0, 1.0) as f32
    };
    // Fondu du texte neutre vers un rouge-orangé marqué.
    let base = ui.visuals().text_color();
    let presse = egui::Color32::from_rgb(0xc3, 0x5a, 0x3f);
    let canaux = |b: u8, p: u8| -> u8 {
        let delta = (i16::from(p) - i16::from(b)) as f32 * intensite;
        (f32::from(b) + delta).round().clamp(0.0, 255.0) as u8
    };
    let couleur = egui::Color32::from_rgb(
        canaux(base.r(), presse.r()),
        canaux(base.g(), presse.g()),
        canaux(base.b(), presse.b()),
    );
    ui.label(RichText::new(fmt::nombre(montant)).color(couleur));
}

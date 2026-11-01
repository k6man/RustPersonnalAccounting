//! Écran « Tableau de bord » : soldes clés, jauges de plafond, patrimoine
//! net, budget vs réel du mois, courbe d'épargne.
//!
//! Présentation pure : tout vient de [`crate::viewmodel::DashboardData`].

use egui::{Color32, RichText};
use egui_plot::{Line, Plot, PlotPoints};

use crate::fmt;
use crate::viewmodel::{DashboardData, EtatJauge};

/// Dessine le tableau de bord.
pub fn afficher(ui: &mut egui::Ui, donnees: &DashboardData) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        cartes_soldes(ui, donnees);
        ui.add_space(8.0);
        patrimoine_et_budget(ui, donnees);
        ui.add_space(8.0);
        courbe_epargne(ui, donnees);
        avertissements_budget(ui, donnees);
    });
}

/// Les trois cartes de soldes + les jauges de plafond.
fn cartes_soldes(ui: &mut egui::Ui, donnees: &DashboardData) {
    ui.heading("Soldes des comptes");
    ui.add_space(4.0);

    // Cartes en cadres à largeur garantie : le contenu (titre + montant en
    // taille heading) ne subit jamais de contrainte de cellule.
    egui::Grid::new("soldes")
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            carte_solde(ui, "Compte courant", donnees.solde_cc);
            carte_solde(ui, "Livret A", donnees.solde_livret_a);
            carte_solde(ui, "LDD", donnees.solde_ldd);
            ui.end_row();
        });
    if !donnees.jauges.is_empty() {
        ui.add_space(10.0);
        ui.label(RichText::new("Plafonds d'épargne").strong());
        for jauge in &donnees.jauges {
            ui.horizontal(|ui| {
                let couleur = crate::views::couleur_etat(jauge.etat);
                let fraction = jauge.etat.fraction(jauge.courant, jauge.plafond);
                let texte = format!(
                    "{} : {} / {} ({})",
                    jauge.compte,
                    fmt::eur(jauge.courant),
                    fmt::eur(jauge.plafond),
                    match jauge.etat {
                        EtatJauge::Ok => "ok",
                        EtatJauge::Alerte => "attention ≥ 90 %",
                        EtatJauge::Depassement => "DÉPASSÉ",
                    }
                );
                // Le texte vit HORS de la barre : jamais tronqué par sa
                // largeur, quelle que soit la fenêtre.
                ui.label(texte);
                // Largeur FIÉE : `desired_width(f32::INFINITY)` ici dilate la
                // largeur du contenu du `ScrollArea::vertical` à l'infini
                // (egui 0.37), ce qui peut faire basculer d'autres widgets
                // dont les tailles dépendent de la largeur disponible (le
                // `Plot` de la courbe allouait un rect infini → tracé vide).
                let barre = egui::ProgressBar::new(fraction as f32)
                    .fill(couleur)
                    .desired_width(240.0);
                if jauge.etat == EtatJauge::Depassement {
                    // Dépassement : visuellement bloquant (cadre rouge).
                    egui::Frame::group(ui.style())
                        .stroke(egui::Stroke::new(2.0, couleur))
                        .show(ui, |ui| {
                            ui.add(barre);
                        });
                } else {
                    ui.add(barre);
                }
            });
        }
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "NB : le « courant » inclut les intérêts annuels crédités sur le compte (cf. conventions).",
            )
            .weak()
            .small(),
        );
    }
}

fn carte_solde(ui: &mut egui::Ui, titre: &str, solde: Option<rust_decimal::Decimal>) {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(10.0)
        .show(ui, |ui| {
            // Largeur minimum explicite : le montant en taille heading
            // (« 18 450,12 € ») a de la marge, la carte garde une taille
            // stable quel que soit son contenu.
            ui.set_min_size(egui::vec2(210.0, 0.0));
            ui.vertical(|ui| {
                ui.label(RichText::new(titre).weak().small());
                match solde {
                    Some(solde) => {
                        ui.label(RichText::new(fmt::eur(solde)).heading().strong());
                    }
                    None => {
                        ui.label(RichText::new("—").heading().weak());
                    }
                }
            });
        });
}

/// Patrimoine net + carte budget vs réel du mois courant.
///
/// Pas de `egui::Grid` ici : la première cellule était écrasée par la carte
/// budget (largeur mesurée au plus juste, le montant heading se repliait sur
/// trois lignes). Pile verticale + carte à largeur garantie.
fn patrimoine_et_budget(ui: &mut egui::Ui, donnees: &DashboardData) {
    carte_patrimoine(ui, donnees.patrimoine_net);
    ui.add_space(8.0);
    ui.label(RichText::new(format!("Budget vs réel — {}", donnees.budget_mois_label)).weak());
    carte_budget(ui, donnees);
}

fn carte_patrimoine(ui: &mut egui::Ui, patrimoine: rust_decimal::Decimal) {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(10.0)
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(210.0, 0.0));
            ui.vertical(|ui| {
                ui.label(RichText::new("Patrimoine net").weak().small());
                ui.label(RichText::new(fmt::eur(patrimoine)).heading().strong());
            });
        });
}

/// Les postes les plus tendus du mois (8 premiers).
fn carte_budget(ui: &mut egui::Ui, donnees: &DashboardData) {
    if donnees.budget_mois.is_empty() {
        ui.label("Aucun budget pour ce mois.");
        return;
    }
    egui::Grid::new("budget-mois")
        .spacing([12.0, 3.0])
        .show(ui, |ui| {
            for poste in donnees.budget_mois.iter().take(8) {
                let couleur = if poste.reel > poste.budgete {
                    Color32::from_rgb(0xd8, 0x3b, 0x2f)
                } else {
                    ui.visuals().text_color()
                };
                ui.label(poste.compte.as_str());
                let ratio = poste.consomme().unwrap_or(0.0);
                ui.add(
                    egui::ProgressBar::new(ratio.clamp(0.0, 1.0) as f32)
                        .desired_width(240.0)
                        .show_percentage()
                        .fill(if ratio > 1.0 {
                            Color32::from_rgb(0xd8, 0x3b, 0x2f)
                        } else {
                            Color32::from_rgb(0x3f, 0xa7, 0x60)
                        }),
                );
                ui.label(
                    RichText::new(format!(
                        "{} / {} ({})",
                        fmt::eur(poste.reel),
                        fmt::eur(poste.budgete),
                        fmt::pourcent(ratio)
                    ))
                    .color(couleur),
                );
                ui.end_row();
            }
        });
}

/// Courbe du patrimoine net (egui_plot), X = index du mois.
///
/// NB : le `Plot` regarde la largeur disponible de son `Ui` pour se
/// dimensionner ; une largeur infinie en amont (widget à
/// `desired_width(f32::INFINITY)` dans le même `ScrollArea`, cf. les jauges
/// ci-dessus) le fait allouer un rect infini → sommets peints à l'infini →
/// tracé silencieusement vide. On garde donc toute largeur qui provient du
/// contenu de ce ScrollArea finie.
fn courbe_epargne(ui: &mut egui::Ui, donnees: &DashboardData) {
    ui.add_space(10.0);
    ui.label(RichText::new("Patrimoine net, mois par mois").strong());
    if donnees.courbe.is_empty() {
        ui.label("Pas encore de données.");
        return;
    }
    let etiquettes: Vec<String> = donnees.courbe.iter().map(|(m, _)| m.clone()).collect();
    let points: Vec<[f64; 2]> = donnees
        .courbe
        .iter()
        .enumerate()
        .map(|(i, (_, valeur))| [f64::from(i as u32), f64::try_from(*valeur).unwrap_or(0.0)])
        .collect();

    Plot::new("courbe-epargne")
        .height(220.0)
        .x_axis_formatter(move |mark, _plage| {
            let index = mark.value.clamp(0.0, f64::from(u32::MAX)) as usize;
            etiquettes.get(index).cloned().unwrap_or_default()
        })
        .y_axis_formatter(|mark, _plage| {
            rust_decimal::Decimal::try_from(mark.value)
                .map(fmt::nombre)
                .unwrap_or_default()
        })
        .show(ui, |plot_ui| {
            plot_ui.line(
                Line::new("Patrimoine", PlotPoints::new(points))
                    .color(Color32::from_rgb(0x4f, 0x9f, 0xff))
                    .width(2.0),
            );
        });
}

fn avertissements_budget(ui: &mut egui::Ui, donnees: &DashboardData) {
    if donnees.budget_avertissements.is_empty() {
        return;
    }
    ui.add_space(8.0);
    let titre = format!(
        "Avertissements budgétaires ({})",
        donnees.budget_avertissements.len()
    );
    egui::CollapsingHeader::new(RichText::new(titre).weak())
        .id_salt("avertissements-budget")
        .show(ui, |ui| {
            for avertissement in &donnees.budget_avertissements {
                ui.label(avertissement.as_str());
            }
        });
}

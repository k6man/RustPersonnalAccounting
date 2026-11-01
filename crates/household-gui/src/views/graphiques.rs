//! Écran « Graphiques » : répartition des dépenses (année / mois),
//! cascade annuelle (identité comptable), budget vs réel du mois.
//!
//! Présentation pure : tout vient de [`crate::viewmodel::GraphesData`] et
//! [`crate::viewmodel::UiData`] (Decimal) — la conversion `f64` n'existe
//! qu'aux frontières du tracé. Un seul graphe actif à la fois (sélecteur
//! interne) : jamais plus d'un `Plot` par fenêtre, donc zéro collision
//! d'IDs et des largeurs finies (règles egui 0.37, cf. TODO.md).

use egui::{Color32, RichText};
use egui_plot::{Bar, BarChart, Legend, Plot};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::fmt;
use crate::viewmodel::{BudgetVsReel, GraphesData, UiData};

/// Mini-onglet sélectionné dans la fenêtre (un seul graphe visible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OngletGraphe {
    /// Répartition des dépenses (barres horizontales colorées).
    Repartition,
    /// Cascade de l'année : Revenus − Dépenses = Δ patrimoine (+ épargne
    /// en information).
    #[default]
    Cascade,
    /// Budget vs réel du mois, barres groupées par poste.
    BudgetReel,
}

/// Période de la répartition (défaut = année — vision la plus englobante).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Periode {
    #[default]
    Annee,
    Mois,
}

/// État de l'écran (gardé entre ouverture/fermeture de la fenêtre).
#[derive(Debug, Clone)]
pub struct UiState {
    pub onglet: OngletGraphe,
    pub periode: Periode,
}

impl UiState {
    #[must_use]
    pub fn nouveau() -> Self {
        Self {
            onglet: OngletGraphe::default(),
            periode: Periode::default(),
        }
    }
}

/// Les trois mini-onglets (ordre = affichage).
const MINI_ONGLETS: [(OngletGraphe, &str); 3] = [
    (OngletGraphe::Repartition, "Répartition des dépenses"),
    (OngletGraphe::Cascade, "Cascade de l'année"),
    (OngletGraphe::BudgetReel, "Budget vs réel du mois"),
];

/// Dessine l'écran Graphiques.
pub fn afficher(ui: &mut egui::Ui, donnees: &UiData, etat: &mut UiState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal(|ui| {
            for (cible, libelle) in MINI_ONGLETS {
                ui.selectable_value(&mut etat.onglet, cible, libelle);
            }
        });
        ui.separator();
        ui.add_space(4.0);
        match etat.onglet {
            OngletGraphe::Repartition => repartition(ui, donnees, etat),
            OngletGraphe::Cascade => cascade(ui, &donnees.graphiques),
            OngletGraphe::BudgetReel => budget_reel(ui, donnees),
        }
    });
}

// ------------------------------------------------------------ répartition ---

/// Barres horizontales colorées : les catégories triées décroissant, part
/// de total en % (les « Autres » couvrent le reste — la somme des parts
/// est exactement 100 %).
fn repartition(ui: &mut egui::Ui, donnees: &UiData, etat: &mut UiState) {
    let graphes = &donnees.graphiques;
    ui.heading("Répartition des dépenses");
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Période :").weak());
        ui.selectable_value(&mut etat.periode, Periode::Annee, "Année");
        ui.selectable_value(&mut etat.periode, Periode::Mois, "Mois courant");
    });
    let sous_titre = match etat.periode {
        Periode::Annee => format!("Exercice {}", graphes.annee),
        Periode::Mois => format!(
            "Mois de {} {}",
            mois_nom(usize::from(graphes.mois) - 1),
            graphes.annee
        ),
    };
    ui.label(RichText::new(sous_titre).weak());
    ui.add_space(6.0);

    let lignes = match etat.periode {
        Periode::Annee => &graphes.repartition_annee,
        Periode::Mois => &graphes.repartition_mois,
    };
    if lignes.is_empty() {
        ui.label("Aucune dépense EUR sur la période.");
        return;
    }
    let total: Decimal = lignes.iter().map(|(_, montant)| *montant).sum();

    for (index, (categorie, montant)) in lignes.iter().enumerate() {
        ui.horizontal(|ui| {
            // Étiquette à largeur fixe : la colonne des barres s'aligne.
            ui.add_sized(
                [220.0, 18.0],
                egui::Label::new(RichText::new(categorie).small()).truncate(),
            );
            barre_horizontale(ui, index, *montant, total);
            ui.label(fmt::eur(*montant));
            let part = f64::try_from(*montant / total).unwrap_or(0.0);
            ui.label(RichText::new(fmt::pourcent(part)).weak().small());
        });
    }
    ui.add_space(4.0);
    ui.label(RichText::new(format!("Total : {}", fmt::eur(total))).strong());
}

/// Une barre horizontale peinte à la main : largeur fixe (égui 0.37 :
/// jamais de largeur infinie dans un `ScrollArea::vertical` — cf. la
/// courbe vide du tableau de bord), arrondi en u8 (`CornerRadius`).
fn barre_horizontale(ui: &mut egui::Ui, index: usize, montant: Decimal, total: Decimal) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(320.0, 14.0), egui::Sense::hover());
    let fraction = if total.is_zero() {
        0.0_f32
    } else {
        f64::try_from(montant / total)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0) as f32
    };
    let largeur = rect.width() * fraction;
    if largeur > 0.0 {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(largeur, rect.height())),
            3u8,
            couleur_barre(index),
        );
    }
}

/// Palette fixe par index (stabilité visuelle entre rechargements).
fn couleur_barre(index: usize) -> Color32 {
    const PALETTE: [Color32; 8] = [
        Color32::from_rgb(0x4f, 0x9f, 0xff),
        Color32::from_rgb(0x3f, 0xa7, 0x60),
        Color32::from_rgb(0xe2, 0x93, 0x1b),
        Color32::from_rgb(0xc3, 0x5a, 0x3f),
        Color32::from_rgb(0x9c, 0x7a, 0xc0),
        Color32::from_rgb(0x51, 0xaf, 0xb4),
        Color32::from_rgb(0xd8, 0xc1, 0x54),
        Color32::from_rgb(0x99, 0x99, 0x99),
    ];
    PALETTE[index % PALETTE.len()]
}

// ------------------------------------------------------------- cascade ---

/// Cascade de l'année : trois barres d'identité (Revenus − Dépenses = Δ
/// patrimoine) colorées séparément + la barre d'information épargne.
fn cascade(ui: &mut egui::Ui, graphes: &GraphesData) {
    let w = &graphes.cascade;
    ui.heading(format!("Cascade de l'exercice {}", graphes.annee));
    ui.add_space(2.0);
    if w.revenus.is_zero() && w.depenses.is_zero() && w.variation_patrimoine.is_zero() {
        ui.label("Aucun flux enregistré pour cette année.");
        return;
    }

    let en_eur = |montant: Decimal| montant.to_f64().unwrap_or(0.0);
    // Une BarChart par barre : la couleur est portée par le chart (egui
    //_plot 0.38), pas par la barre individuelle.
    let barres = [
        (
            Bar::new(0.0, en_eur(w.revenus)),
            Color32::from_rgb(0x3f, 0xa7, 0x60),
        ),
        (
            Bar::new(1.0, -en_eur(w.depenses)),
            Color32::from_rgb(0xc3, 0x5a, 0x3f),
        ),
        (
            Bar::new(2.0, en_eur(w.variation_patrimoine)),
            Color32::from_rgb(0x4f, 0x9f, 0xff),
        ),
        (
            Bar::new(3.0, en_eur(w.epargne)),
            Color32::from_rgb(0xe2, 0x93, 0x1b),
        ),
    ];
    let libelles = ["Revenus", "Dépenses", "Δ patrimoine", "Épargne (info)"];

    Plot::new("cascade-annuelle")
        .height(230.0)
        .legend(Legend::default())
        .x_axis_formatter(move |mark, _plage| {
            libelles
                .get(mark.value.round() as usize)
                .map_or_else(String::new, |l| (*l).to_string())
        })
        .y_axis_formatter(|mark, _plage| {
            rust_decimal::Decimal::try_from(mark.value)
                .map(fmt::nombre)
                .unwrap_or_default()
        })
        .show(ui, |trace| {
            for (barre, couleur) in barres {
                trace.bar_chart(BarChart::new(String::new(), vec![barre]).color(couleur));
            }
        });
    ui.add_space(4.0);
    // Valeurs exactes sous le tracé (le survol ne suffit pas à la lecture).
    ui.label(RichText::new(format!(
        "Revenus {} − Dépenses {} = {} ; Δ patrimoine observé : {}",
        fmt::eur(w.revenus),
        fmt::eur(w.depenses),
        fmt::eur(w.revenus - w.depenses),
        fmt::eur(w.variation_patrimoine)
    )));
    ui.label(
        RichText::new(format!(
            "Épargne nette : {} (information : intérêts déjà comptés dans les \
         revenus, transferts entre comptes d'épargne et compte courant \
         s'annulent).",
            fmt::eur(w.epargne)
        ))
        .weak(),
    );
}

// --------------------------------------------------------- budget vs réel ---

/// Barres groupées budget vs réel du mois courant (même top 8 trié
/// « plus tendus en tête » que la carte du tableau de bord).
fn budget_reel(ui: &mut egui::Ui, donnees: &UiData) {
    let bord = &donnees.tableau_de_bord;
    let top: Vec<&BudgetVsReel> = bord.budget_mois.iter().take(8).collect();
    ui.heading("Budget vs réel — mois courant");
    ui.label(RichText::new(&bord.budget_mois_label).weak());
    ui.add_space(2.0);
    if top.is_empty() {
        ui.label("Aucun budget pour ce mois.");
        return;
    }

    let en_eur = |montant: Decimal| montant.to_f64().unwrap_or(0.0);
    let budgetes = BarChart::new(
        "Budgeté",
        top.iter()
            .enumerate()
            .map(|(i, poste)| Bar::new(f64::from(i as u32), en_eur(poste.budgete)))
            .collect(),
    )
    .color(egui::Color32::from_rgb(0xb0, 0xb0, 0xb0));
    let reels = BarChart::new(
        "Réel",
        top.iter()
            .enumerate()
            .map(|(i, poste)| Bar::new(f64::from(i as u32), en_eur(poste.reel)))
            .collect(),
    )
    .color(egui::Color32::from_rgb(0x4f, 0x9f, 0xff));

    // Noms courts de comptes pour l'axe X (2 derniers segments, tronqués).
    let noms: Vec<String> = top.iter().map(|poste| nom_court(&poste.compte)).collect();
    Plot::new("budget-vs-reel")
        .height(240.0)
        .legend(Legend::default())
        .x_axis_formatter(move |mark, _plage| {
            noms.get(mark.value.round() as usize)
                .cloned()
                .unwrap_or_default()
        })
        .y_axis_formatter(|mark, _plage| {
            rust_decimal::Decimal::try_from(mark.value)
                .map(fmt::nombre)
                .unwrap_or_default()
        })
        .show(ui, |trace| {
            trace.bar_chart(budgetes);
            trace.bar_chart(reels);
        });
    ui.add_space(2.0);
    for poste in &top {
        let ratio = poste.consomme().unwrap_or(0.0);
        let couleur = if poste.reel > poste.budgete {
            Color32::from_rgb(0xd8, 0x3b, 0x2f)
        } else {
            ui.visuals().text_color()
        };
        ui.label(
            RichText::new(format!(
                "{} : {} / {} ({})",
                poste.compte,
                fmt::eur(poste.reel),
                fmt::eur(poste.budgete),
                fmt::pourcent(ratio)
            ))
            .color(couleur),
        );
    }
}

/// Nom court d'un compte pour l'axe X : les deux derniers segments
/// (`Expenses:Alimentation:Courses` → `Alimentation:Courses`).
fn nom_court(compte: &str) -> String {
    let segments: Vec<&str> = compte.split(':').collect();
    let debut = segments.len().saturating_sub(2);
    segments[debut..].join(":")
}

/// Nom de mois complet pour les sous-titres (et non l'abréviation).
fn mois_nom(index: usize) -> &'static str {
    const MOIS: [&str; 12] = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    MOIS.get(index).copied().unwrap_or("?")
}

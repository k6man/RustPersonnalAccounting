//! Écran « Saisie » : brouillon de transaction → writer (jalon 5).
//!
//! Présentation pure : la validation de forme vit dans [`crate::saisie`]
//! (pur, testé) ; la validation *autorisée* est la pré-vérification scratch
//! du writer — l'écran l'explique à l'utilisateur.

use egui::RichText;

use crate::saisie::{
    Brouillon, Empreinte, LigneBrouillon, completer_comptes, completer_payees, date_optionnelle,
};
use crate::viewmodel::InfoCompte;

/// Mode de l'écran : nouvelle transaction ou modification d'une existante.
#[derive(Debug, Clone)]
pub enum Mode {
    Nouveau,
    Modification { index: usize, empreinte: Empreinte },
}

/// État d'interface de l'écran Saisie.
#[derive(Debug, Clone)]
pub struct UiState {
    pub brouillon: Brouillon,
    pub mode: Mode,
}

impl UiState {
    /// Nouvelle saisie, brouillon vierge daté du jour.
    #[must_use]
    pub fn nouveau(aujourdhui: rustledger_core::NaiveDate) -> Self {
        Self {
            brouillon: Brouillon::vierge(aujourdhui),
            mode: Mode::Nouveau,
        }
    }

    /// Passage en mode modification, pré-rempli depuis le journal.
    #[must_use]
    pub fn modification(ligne: &crate::viewmodel::LigneJournal) -> Self {
        Self {
            brouillon: crate::saisie::depuis_ligne(ligne),
            mode: Mode::Modification {
                index: ligne.index,
                empreinte: Empreinte::de_ligne(ligne),
            },
        }
    }
}

/// Action demandée par l'écran vers l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Enregistrer (le mode — création ou modification — vit dans l'état).
    Enregistrer,
    /// Abandonner la modification (retour au journal, brouillon réinitialisé).
    Annuler,
}

/// Dessine l'écran de saisie. Retourne l'action demandée.
pub fn afficher(
    ui: &mut egui::Ui,
    etat: &mut UiState,
    comptes: &[InfoCompte],
    payees: &[(String, usize)],
) -> Option<Action> {
    let mut action = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Entête du mode.
        match &etat.mode {
            Mode::Nouveau => ui.heading("Nouvelle transaction"),
            Mode::Modification { .. } => ui.heading("Modifier la transaction"),
        };
        ui.add_space(6.0);

        // --- En-tête de transaction -------------------------------------
        // NB : pas de `egui::Grid` ici — egui 0.37 borne `desired_width`
        // d'un `TextEdit` par la largeur allouée à sa cellule, qui reste
        // collée à la valeur du frame précédent (~40 px). Rangées
        // horizontales explicites à la place.
        ui.horizontal(|ui| {
            ui.label("Date :");
            let date_valide = date_optionnelle(&etat.brouillon.date).is_some();
            champ_date(ui, &mut etat.brouillon.date, date_valide);
            ui.add_space(14.0);
            ui.label("État :");
            if ui
                .selectable_label(etat.brouillon.flag == '*', "* pointée")
                .clicked()
            {
                etat.brouillon.flag = '*';
            }
            if ui
                .selectable_label(etat.brouillon.flag == '!', "! en attente")
                .clicked()
            {
                etat.brouillon.flag = '!';
            }
        });
        ui.add_space(4.0);

        // Tiers avec suggestions (fréquence).
        ui.horizontal(|ui| {
            ui.label("Tiers :");
            let propositions = completer_payees(payees, &etat.brouillon.tiers, 6);
            champ_suggestions(
                ui,
                &mut etat.brouillon.tiers,
                &propositions,
                "ex. Carrefour",
                320.0,
            );
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Libellé :");
            ui.add(
                egui::TextEdit::singleline(&mut etat.brouillon.libelle)
                    .desired_width(420.0)
                    .hint_text("ex. Courses de la semaine"),
            );
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Tags :");
            ui.add(
                egui::TextEdit::singleline(&mut etat.brouillon.tags)
                    .desired_width(420.0)
                    .hint_text("#courses #weekend"),
            );
            ui.add_space(14.0);
            ui.label("Liens :");
            ui.add(
                egui::TextEdit::singleline(&mut etat.brouillon.liens)
                    .desired_width(420.0)
                    .hint_text("^facture-042"),
            );
        });

        ui.add_space(8.0);
        ui.label(RichText::new("Écritures").strong());
        lignes_postings(ui, etat, comptes);

        // --- Validation et enregistrement --------------------------------
        ui.add_space(10.0);
        let erreurs = etat.brouillon.erreurs();
        if erreurs.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(0x3f, 0xa7, 0x60),
                "✓ prête à enregistrer",
            );
        } else {
            for erreur in &erreurs {
                ui.colored_label(
                    egui::Color32::from_rgb(0xd8, 0x3b, 0x2f),
                    format!("✗ {erreur}"),
                );
            }
        }
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Validation finale : le ledger est reconstitué sur une copie et \
                 revalidé avant toute écriture (refus = fichier intact).",
            )
            .weak()
            .small(),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let enregistrable = erreurs.is_empty();
            if ui
                .add_enabled(
                    enregistrable,
                    egui::Button::new(match etat.mode {
                        Mode::Nouveau => "Enregistrer",
                        Mode::Modification { .. } => "Enregistrer la modification",
                    }),
                )
                .clicked()
            {
                action = Some(Action::Enregistrer);
            }
            if matches!(etat.mode, Mode::Modification { .. }) && ui.button("Annuler").clicked() {
                action = Some(Action::Annuler);
            }
        });
    });

    action
}

/// Éditeur de lignes de postings : comptes avec autocomplétion, montants
/// libres, équilibrage explicite de la contrepartie.
///
/// Pas de `egui::Grid` : egui 0.37 borne la largeur d'un `TextEdit` par la
/// largeur allouée à la cellule, qui ne grandit pas (le champ resterait
/// coincé à ~interact_size ≈ 40 px). Une rangée horizontale par posting.
fn lignes_postings(ui: &mut egui::Ui, etat: &mut UiState, comptes: &[InfoCompte]) {
    let date = date_optionnelle(&etat.brouillon.date);
    let mut suppression = Option::<usize>::None;

    for (numero, ligne) in etat.brouillon.lignes.iter_mut().enumerate() {
        let montant_vide = ligne.montant.is_empty();
        // Compte + suggestions.
        let propositions = date
            .map(|date| completer_comptes(comptes, &ligne.compte, date, 6))
            .unwrap_or_default();
        ui.horizontal(|ui| {
            champ_suggestions(
                ui,
                &mut ligne.compte,
                &propositions,
                "Assets:… / Expenses:…",
                460.0,
            );
            // Montant (vide = équilibrage, marqué).
            ui.add(
                egui::TextEdit::singleline(&mut ligne.montant)
                    .desired_width(150.0)
                    .hint_text(if montant_vide {
                        "= équilibrage"
                    } else {
                        "0.00 [DEVISE]"
                    }),
            );
            if ui.small_button("✕").clicked() {
                // On note la demande pour l'appliquer après la boucle
                // (emprunt mutable en cours sur le brouillon).
                suppression = Some(numero);
            }
        });
        ui.add_space(3.0);
    }
    if let Some(numero) = suppression {
        etat.brouillon.lignes.remove(numero);
    }

    ui.horizontal(|ui| {
        if ui.small_button("＋ ligne").clicked() {
            etat.brouillon.lignes.push(LigneBrouillon::default());
        }
        // Reporter le résidu sur une nouvelle ligne (contrepartie explicite).
        let residus = etat.brouillon.residus();
        if let Some((devise, reste)) = residus.first()
            && ui
                .small_button(format!("Reporter {reste} {devise} sur une nouvelle ligne"))
                .clicked()
        {
            etat.brouillon.lignes.push(LigneBrouillon {
                compte: String::new(),
                montant: format!("{reste} {devise}"),
            });
        }
        // Indicateur d'équilibre.
        match etat.brouillon.residus().first() {
            Some((devise, reste)) => {
                ui.colored_label(
                    egui::Color32::from_rgb(0xd8, 0x3b, 0x2f),
                    format!("reste à répartir : {reste} {devise}"),
                );
            }
            None => {
                ui.colored_label(egui::Color32::from_rgb(0x3f, 0xa7, 0x60), "équilibrée ✓");
            }
        }
    });
}

/// Champ texte avec liste de suggestions cliquables sous le champ quand il
/// a le focus. La largeur dépend du contenu attendu (un compte complet
/// n'a pas la même longueur qu'un tiers).
fn champ_suggestions(
    ui: &mut egui::Ui,
    texte: &mut String,
    propositions: &[String],
    indice: &str,
    largeur: f32,
) {
    ui.vertical(|ui| {
        let reponse = ui.add(
            egui::TextEdit::singleline(texte)
                .desired_width(largeur)
                .hint_text(indice),
        );
        if reponse.has_focus() {
            for proposition in propositions {
                if ui.small_button(proposition.as_str()).clicked() {
                    *texte = proposition.clone();
                }
            }
        }
    });
}

fn champ_date(ui: &mut egui::Ui, valeur: &mut String, valide: bool) {
    let champ = egui::TextEdit::singleline(valeur)
        .desired_width(130.0)
        .hint_text("AAAA-MM-JJ");
    if valide {
        ui.add(champ);
    } else {
        ui.add(champ.text_color(egui::Color32::from_rgb(0xd8, 0x3b, 0x2f)));
    }
}

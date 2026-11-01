//! Écran « Git » : statut par fichier, résumé du diff, commit avec message
//! (suggestion préremplie, validée), historique du ledger.
//!
//! Présentation pure : les données viennent de [`crate::worker::GitDonnees`]
//! (chargées dans le worker) ; l'écriture passe par `Command::GitCommit`,
//! qui valide le ledger avant d'indexer (ADR-002).

use egui::RichText;

use crate::worker::GitDonnees;

/// Action demandée par l'écran vers l'application.
#[derive(Debug, Clone)]
pub enum ActionGit {
    /// Recharger le statut / historique.
    Actualiser,
    /// Commiter avec ce message (le worker valide le ledger avant).
    Commit(String),
}

/// État local de l'écran (tampon du message de commit).
#[derive(Debug, Clone)]
pub struct UiState {
    /// Tampon du message de commit (éditable).
    pub message: String,
    /// Suggestion déjà vue : quand elle change (nouveaux fichiers modifiés),
    /// le tampon est re-prérempli.
    pub suggestion_vue: String,
}

impl UiState {
    #[must_use]
    pub fn nouveau() -> Self {
        Self {
            message: String::new(),
            suggestion_vue: "…".to_string(),
        }
    }
}

/// Dessine l'écran Git.
pub fn afficher(ui: &mut egui::Ui, git: &GitDonnees, etat: &mut UiState) -> Option<ActionGit> {
    let mut action = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new("Versionnement du ledger").heading().strong());
        ui.add_space(4.0);

        // Barre d'actions.
        ui.add_space(0.0);
        if ui
            .add_enabled(git.disponible, egui::Button::new("Actualiser"))
            .clicked()
        {
            action = Some(ActionGit::Actualiser);
        }

        if !git.disponible {
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Aucun dépôt git n'englobe ce ledger : lancez \
                     `git init` dans le dossier du projet pour activer \
                     l'historique et les commits.",
                )
                .weak(),
            );
            return;
        }

        // --- Statut des fichiers ------------------------------------------
        ui.add_space(6.0);
        if git.fichiers.is_empty() {
            ui.label(RichText::new("✔ Tout est commité — rien à faire.").strong());
        } else {
            ui.label(
                RichText::new(format!(
                    "{} fichier(s) modifié(s) ({} ajoutions, {} suppressions)",
                    git.resume.fichiers, git.resume.insertions, git.resume.suppressions
                ))
                .strong(),
            );
            egui_extras::TableBuilder::new(ui)
                // Deux tables dans le même Ui : leur id par défaut dérive de
                // `ui.scope_id()` + "__table_state" → collision (egui peint
                // des warnings rouges). Sel unique par table.
                .id_salt("fichiers-git")
                .striped(true)
                .column(egui_extras::Column::remainder())
                .column(egui_extras::Column::auto().at_least(160.0))
                .header(20.0, |mut en_tete| {
                    en_tete.col(|ui| {
                        ui.strong("Fichier");
                    });
                    en_tete.col(|ui| {
                        ui.strong("État");
                    });
                })
                .body(|mut corps| {
                    for fichier in &git.fichiers {
                        corps.row(20.0, |mut rangee| {
                            rangee.col(|ui| {
                                ui.label(fichier.chemin.as_str());
                            });
                            rangee.col(|ui| {
                                let libelles: Vec<&str> =
                                    fichier.etats.iter().map(|e| e.libelle()).collect();
                                ui.label(libelles.join(", "));
                            });
                        });
                    }
                });
        }

        // --- Zone de commit --------------------------------------------------
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.label(
                RichText::new(
                    "Un commit valide le ledger (comme le hook pre-commit, \
                     mais côté app — libgit2 n'exécute pas les hooks).",
                )
                .weak()
                .small(),
            );
            // Re-préremplissage quand la suggestion change (nouveaux
            // fichiers) ; sinon le tampon de l'utilisateur est respecté.
            if etat.suggestion_vue != git.message_suggere {
                etat.message = git.message_suggere.clone();
                etat.suggestion_vue = git.message_suggere.clone();
            }
            ui.add(
                egui::TextEdit::singleline(&mut etat.message)
                    .hint_text("ledger: …")
                    // Largeur fixe : `f32::INFINITY` dilate un ScrollArea
                    // vertical à l'infini (cf. dashboard.rs, jauges).
                    .desired_width(480.0),
            );
            ui.add_space(4.0);
            if ui
                .add_enabled(
                    !git.fichiers.is_empty(),
                    egui::Button::new("Stage tout + commiter"),
                )
                .clicked()
            {
                action = Some(ActionGit::Commit(etat.message.trim().to_string()));
            }
        });

        // --- Historique ------------------------------------------------------
        ui.add_space(10.0);
        ui.label(RichText::new("Historique (commits touchant le ledger)").strong());
        egui_extras::TableBuilder::new(ui)
            .id_salt("historique-git")
            .striped(true)
            .column(egui_extras::Column::auto().at_least(80.0))
            .column(egui_extras::Column::auto().at_least(120.0))
            .column(egui_extras::Column::auto().at_least(140.0))
            .column(egui_extras::Column::remainder())
            .header(20.0, |mut en_tete| {
                en_tete.col(|ui| {
                    ui.strong("OID");
                });
                en_tete.col(|ui| {
                    ui.strong("Date");
                });
                en_tete.col(|ui| {
                    ui.strong("Auteur");
                });
                en_tete.col(|ui| {
                    ui.strong("Message");
                });
            })
            .body(|mut corps| {
                for commit in &git.historique {
                    corps.row(20.0, |mut rangee| {
                        rangee.col(|ui| {
                            ui.monospace(commit.oid.as_str());
                        });
                        rangee.col(|ui| {
                            ui.label(commit.date.as_str());
                        });
                        rangee.col(|ui| {
                            ui.label(commit.auteur.as_str());
                        });
                        rangee.col(|ui| {
                            ui.label(commit.sujet.as_str());
                        });
                    });
                }
            });
    });

    action
}

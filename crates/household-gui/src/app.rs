//! Application eframe : état, canaux, onglets, bannière d'erreurs.
//!
//! Architecture (cf. TODO jalon 4) :
//! - l'état GUI ne fait que *présenter* : tout le calcul vient du view-model
//!   (thread worker) ;
//! - `logic` (hors rendu) draine les résultats du worker ;
//! - `ui` dessine : barre d'en-tête, bannière d'erreurs, onglets, vue.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use egui::RichText;

use crate::viewmodel::UiData;
use crate::worker::{Command, LoadOutcome};

/// Les écrans de l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Onglet {
    TableauDeBord,
    Journal,
    Saisie,
    Budget,
    Rapports,
    Git,
}

impl Onglet {
    const TOUS: [Onglet; 6] = [
        Onglet::TableauDeBord,
        Onglet::Journal,
        Onglet::Saisie,
        Onglet::Budget,
        Onglet::Rapports,
        Onglet::Git,
    ];

    fn libelle(self) -> &'static str {
        match self {
            Self::TableauDeBord => "Tableau de bord",
            Self::Journal => "Journal",
            Self::Saisie => "Saisie",
            Self::Budget => "Budget",
            Self::Rapports => "Rapports",
            Self::Git => "Git",
        }
    }

    /// Onglet choisi par la variable d'environnement `COMPTA_ECRAN`
    /// (`dashboard|journal|saisie|budget|rapports|git`) — affordance de
    /// test, l'app démarre sinon sur le tableau de bord.
    #[must_use]
    pub fn depuis_env(var: Option<&str>) -> Self {
        match var.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("journal") => Self::Journal,
            Some("saisie") => Self::Saisie,
            Some("budget") => Self::Budget,
            Some("rapports") => Self::Rapports,
            Some("git") => Self::Git,
            Some("dashboard") | Some("tableau") => Self::TableauDeBord,
            _ => Self::TableauDeBord,
        }
    }
}

/// L'application.
pub struct FoyerApp {
    main: PathBuf,
    /// Émetteur de commandes vers le worker (cloné pour le watcher).
    cmd_tx: Sender<Command>,
    /// Récepteur des résultats du worker.
    outcome_rx: Receiver<LoadOutcome>,
    /// Garde le watcher vivant (le détruire arrête la surveillance).
    _watcher: notify::RecommendedWatcher,

    /// Données prêtes à afficher (`None` tant que le premier chargement
    /// n'est pas fini).
    donnees: Option<Arc<UiData>>,
    /// Échec du dernier chargement (le ledger est cassé).
    echec: Option<String>,
    /// Rechargement / écriture en cours ?
    en_chargement: bool,
    /// Message transitoire (écriture réussie / refusée).
    flash: Option<(String, bool)>, // (texte, est_erreur)

    onglet: Onglet,
    journal: crate::views::journal::UiState,
    saisie: crate::views::saisie::UiState,
    budget: crate::views::budget::UiState,
    /// État de l'écran Git (message de commit en cours d'édition).
    git: crate::views::git::UiState,
    /// Données git du dernier chargement (`None` tant que non demandées).
    git_donnees: Option<crate::worker::GitDonnees>,
    /// Préférences GUI (rollover par famille…) — TOML local, jamais le ledger.
    config: crate::config::Config,
}

impl FoyerApp {
    /// Crée l'app : worker + watcher + premier chargement asynchrone.
    pub fn new(cc: &eframe::CreationContext<'_>, main: PathBuf) -> Self {
        let (cmd_tx, outcome_rx) = crate::worker::demarrer(cc.egui_ctx.clone(), main.clone());
        let watcher = crate::worker::demarrer_watcher(&main, cmd_tx.clone());
        let onglet = Onglet::depuis_env(std::env::var("COMPTA_ECRAN").ok().as_deref());
        let app = Self {
            main,
            cmd_tx,
            outcome_rx,
            _watcher: watcher,
            donnees: None,
            echec: None,
            en_chargement: true,
            flash: None,
            onglet,
            journal: crate::views::journal::UiState::new(),
            saisie: crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui()),
            budget: crate::views::budget::UiState::nouveau(crate::worker::aujourd_hui().year()),
            git: crate::views::git::UiState::nouveau(),
            git_donnees: None,
            config: crate::config::Config::charger(),
        };
        // Écran Git : même arme que l'entrée par l'onglet (statut à la volée).
        if app.onglet == Onglet::Git {
            app.cmd_tx.send(Command::GitStatut).ok();
        }
        app
    }

    fn demander_rechargement(&mut self) {
        self.en_chargement = true;
        self.cmd_tx.send(Command::Reload).ok();
    }
}

impl eframe::App for FoyerApp {
    /// Hors rendu : réception des résultats du worker.
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(outcome) = self.outcome_rx.try_recv() {
            match outcome {
                LoadOutcome::Pret(donnees) => {
                    self.donnees = Some(donnees);
                    self.echec = None;
                }
                LoadOutcome::Echec(echec) => {
                    // L'état précédent reste affiché sous l'erreur.
                    self.echec = Some(echec);
                }
                LoadOutcome::Ecrit { donnees, message } => {
                    self.donnees = Some(donnees);
                    self.echec = None;
                    self.flash = Some((message, false));
                    // Après une écriture, on retourne au journal pour la voir.
                    self.onglet = Onglet::Journal;
                    self.journal.selection = None;
                }
                LoadOutcome::Refuse(refus) => {
                    self.flash = Some((refus, true));
                }
                LoadOutcome::Git { donnees, message } => {
                    self.git_donnees = Some(donnees);
                    self.en_chargement = false;
                    if let Some((texte, est_erreur)) = message {
                        self.flash = Some((texte, est_erreur));
                    }
                    continue;
                }
            }
            self.en_chargement = false;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("entete").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Comptabilité du foyer").heading().strong());
                if let Some(donnees) = &self.donnees {
                    ui.label(
                        RichText::new(format!(
                            "{} — chargé à {}",
                            donnees.main.display(),
                            donnees.charge_le
                        ))
                        .weak()
                        .small(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.en_chargement {
                        ui.add(egui::widgets::Spinner::new().size(18.0));
                    }
                    if ui.button("Recharger").clicked() {
                        self.demander_rechargement();
                    }
                });
            });
            ui.add_space(2.0);
        });

        // Bannière d'erreurs de validation (et échec de chargement).
        if let Some(echec) = &self.echec {
            egui::Panel::top("banniere-echec")
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(0x6a, 0x1b, 0x1a)))
                .show(ui, |ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(0xff, 0xd9, 0xd9),
                        format!("Chargement impossible : {echec}"),
                    );
                });
        } else if let Some(donnees) = &self.donnees
            && !donnees.erreurs.is_empty()
        {
            egui::Panel::top("banniere-erreurs")
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(0x5c, 0x4a, 0x0e)))
                .show(ui, |ui| {
                    let total = donnees.erreurs.len();
                    ui.colored_label(
                        egui::Color32::from_rgb(0xff, 0xf3, 0xc4),
                        format!("⚠ Le ledger a {total} erreur(s) de validation :"),
                    );
                    for ligne_erreur in donnees.erreurs.iter().take(5) {
                        let ou = ligne_erreur
                            .fichier
                            .as_ref()
                            .map(|f| format!(" ({f}:{})", ligne_erreur.ligne.unwrap_or(0)))
                            .unwrap_or_default();
                        ui.colored_label(
                            egui::Color32::from_rgb(0xff, 0xf3, 0xc4),
                            format!(
                                "[{} {}] {}{}",
                                ligne_erreur.code, ligne_erreur.phase, ligne_erreur.message, ou
                            ),
                        );
                    }
                    if total > 5 {
                        ui.colored_label(
                            egui::Color32::from_rgb(0xff, 0xf3, 0xc4),
                            format!("… et {} autres", total - 5),
                        );
                    }
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            // Barre d'onglets + flash.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for onglet in Onglet::TOUS {
                    if ui
                        .selectable_label(self.onglet == onglet, onglet.libelle())
                        .clicked()
                    {
                        self.onglet = onglet;
                        // L'écran Git relit son statut à chaque entrée.
                        if onglet == Onglet::Git {
                            self.en_chargement = true;
                            self.flash = None;
                            self.cmd_tx.send(Command::GitStatut).ok();
                        }
                    }
                }
                if let Some((message, est_erreur)) = &self.flash {
                    let couleur = if *est_erreur {
                        egui::Color32::from_rgb(0xd8, 0x3b, 0x2f)
                    } else {
                        egui::Color32::from_rgb(0x3f, 0xa7, 0x60)
                    };
                    if ui
                        .label(RichText::new(format!("— {message}")).color(couleur))
                        .clicked()
                    {
                        self.flash = None; // clic = acquittement
                    }
                }
            });
            ui.add_space(6.0);
            ui.separator();

            match (&self.donnees, self.onglet) {
                (Some(donnees), Onglet::TableauDeBord) => {
                    crate::views::dashboard::afficher(ui, &donnees.tableau_de_bord);
                }
                (Some(donnees), Onglet::Journal) => {
                    let action =
                        crate::views::journal::afficher(ui, &donnees.journal, &mut self.journal);
                    match action {
                        Some(crate::views::journal::ActionJournal::Modifier(index)) => {
                            // Pré-remplit la saisie depuis la ligne du journal.
                            if let Some(ligne) =
                                donnees.journal.lignes.iter().find(|l| l.index == index)
                            {
                                self.saisie = crate::views::saisie::UiState::modification(ligne);
                                self.onglet = Onglet::Saisie;
                            }
                        }
                        Some(crate::views::journal::ActionJournal::Supprimer(index)) => {
                            self.demander_suppression(index);
                        }
                        None => {}
                    }
                }
                (Some(donnees), Onglet::Saisie) => {
                    let action = crate::views::saisie::afficher(
                        ui,
                        &mut self.saisie,
                        &donnees.comptes,
                        &donnees.payees,
                    );
                    match action {
                        Some(crate::views::saisie::Action::Enregistrer) => {
                            self.envoyer_brouillon();
                        }
                        Some(crate::views::saisie::Action::Annuler) => {
                            self.saisie = crate::views::saisie::UiState::nouveau(
                                crate::worker::aujourd_hui(),
                            );
                            self.onglet = Onglet::Journal;
                        }
                        None => {}
                    }
                }
                (Some(donnees), Onglet::Budget) => {
                    let action = crate::views::budget::afficher(
                        ui,
                        donnees.grilles.get(&self.budget.annee),
                        &mut self.budget,
                        |famille| self.config.rollover_actif(famille),
                    );
                    match action {
                        Some(crate::views::budget::ActionBudget::Cellule {
                            annee,
                            date,
                            compte,
                            montant,
                        }) => {
                            self.en_chargement = true;
                            self.flash = None;
                            self.cmd_tx
                                .send(Command::CelluleBudget {
                                    annee,
                                    date,
                                    compte,
                                    montant,
                                })
                                .ok();
                        }
                        Some(crate::views::budget::ActionBudget::GenererAnnee {
                            source,
                            cible,
                            pourcent,
                            pas,
                        }) => {
                            self.en_chargement = true;
                            self.flash = None;
                            self.cmd_tx
                                .send(Command::GenererAnnee {
                                    source,
                                    cible,
                                    pourcent,
                                    pas,
                                })
                                .ok();
                        }
                        Some(crate::views::budget::ActionBudget::BasculerRollover(famille)) => {
                            match self.config.basculer_rollover(&famille) {
                                Ok(()) => {
                                    self.flash = Some((
                                        format!(
                                            "rollover {} — préférence enregistrée",
                                            if self.config.rollover_actif(&famille) {
                                                "activé"
                                            } else {
                                                "désactivé"
                                            }
                                        ),
                                        false,
                                    ));
                                }
                                Err(e) => self.flash = Some((e, true)),
                            }
                        }
                        None => {}
                    }
                }
                (Some(donnees), Onglet::Rapports) => {
                    crate::views::rapports::afficher(ui, &donnees.rapports);
                }
                (Some(_), Onglet::Git) => {
                    match &self.git_donnees {
                        Some(git) => {
                            let action = crate::views::git::afficher(ui, git, &mut self.git);
                            match action {
                                Some(crate::views::git::ActionGit::Actualiser) => {
                                    self.en_chargement = true;
                                    self.flash = None;
                                    self.cmd_tx.send(Command::GitStatut).ok();
                                }
                                Some(crate::views::git::ActionGit::Commit(message))
                                    if !message.is_empty() =>
                                {
                                    self.en_chargement = true;
                                    self.flash = None;
                                    self.cmd_tx.send(Command::GitCommit { message }).ok();
                                }
                                _ => {}
                            }
                        }
                        None => {
                            // Écran git pas encore chargé : spinner + premier
                            // déclenchement (l'arm de l'onglet a déjà
                            // envoyé GitStatut).
                            ui.centered_and_justified(|ui| {
                                ui.add(egui::widgets::Spinner::new().size(30.0));
                            });
                        }
                    }
                }
                (None, _) => {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.0);
                            ui.add(egui::widgets::Spinner::new().size(42.0));
                            ui.add_space(12.0);
                            ui.label("Chargement du ledger…");
                            if self.echec.is_none() {
                                ui.label(
                                    RichText::new(self.main.display().to_string())
                                        .weak()
                                        .small(),
                                );
                            }
                        });
                    });
                }
            }
        });
    }
}

impl FoyerApp {
    /// Envoie le brouillon au worker (création ou modification selon le mode).
    fn envoyer_brouillon(&mut self) {
        self.en_chargement = true;
        self.flash = None;
        match &self.saisie.mode {
            crate::views::saisie::Mode::Nouveau => {
                self.cmd_tx
                    .send(Command::Ecrire(self.saisie.brouillon.clone()))
                    .ok();
            }
            crate::views::saisie::Mode::Modification { index, empreinte } => {
                self.cmd_tx
                    .send(Command::Modifier {
                        index: *index,
                        empreinte: empreinte.clone(),
                        brouillon: self.saisie.brouillon.clone(),
                    })
                    .ok();
            }
        }
        // Brouillon réinitialisé : prêt pour la saisie suivante.
        self.saisie = crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
    }

    /// Demande la suppression (index + empreinte pour la relocalisation).
    fn demander_suppression(&mut self, index: usize) {
        let Some(donnees) = &self.donnees else {
            return;
        };
        let Some(ligne) = donnees.journal.lignes.iter().find(|l| l.index == index) else {
            return;
        };
        self.en_chargement = true;
        self.flash = None;
        self.cmd_tx
            .send(Command::Supprimer {
                index,
                empreinte: crate::saisie::Empreinte::de_ligne(ligne),
            })
            .ok();
    }
}

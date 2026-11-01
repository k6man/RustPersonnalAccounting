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

    /// Position dans [`Onglet::TOUS`] (indexé par l'état des fenêtres et les
    /// raccourcis Ctrl+1..6).
    #[must_use]
    pub fn index(self) -> usize {
        Self::TOUS
            .iter()
            .position(|o| *o == self)
            .expect("Onglet::TOUS contient tous les onglets")
    }

    /// Écran demandé par la variable d'environnement `COMPTA_ECRAN`
    /// (`dashboard|journal|saisie|budget|rapports|git`) — affordance de
    /// test : seule cette fenêtre est ouverte au démarrage (sinon : le
    /// tableau de bord).
    #[must_use]
    pub fn depuis_env(var: Option<&str>) -> Option<Self> {
        match var.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("journal") => Some(Self::Journal),
            Some("saisie") => Some(Self::Saisie),
            Some("budget") => Some(Self::Budget),
            Some("rapports") => Some(Self::Rapports),
            Some("git") => Some(Self::Git),
            Some("dashboard") | Some("tableau") => Some(Self::TableauDeBord),
            _ => None,
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

    /// Fenêtres d'écrans ouvertes, indexées comme `Onglet::TOUS`.
    fenetres_ouvertes: [bool; Onglet::TOUS.len()],
    /// Une écriture de saisie est en vol : l'Enregistrer est ignoré jusqu'au
    /// résultat (anti doublon — le brouillon n'est plus réinitialisé à
    /// l'envoi, il survit à un refus pour être corrigé).
    saisie_en_vol: bool,
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
        let mut fenetres_ouvertes = [false; Onglet::TOUS.len()];
        match Onglet::depuis_env(std::env::var("COMPTA_ECRAN").ok().as_deref()) {
            // Affordance de test : SEULE l'écran demandé est ouvert.
            Some(onglet) => {
                fenetres_ouvertes[onglet.index()] = true;
            }
            // Usage normal : le tableau de bord démarre quasi plein écran.
            None => {
                fenetres_ouvertes[Onglet::TableauDeBord.index()] = true;
            }
        }
        let app = Self {
            main,
            cmd_tx,
            outcome_rx,
            _watcher: watcher,
            donnees: None,
            echec: None,
            en_chargement: true,
            saisie_en_vol: false,
            flash: None,
            fenetres_ouvertes,
            journal: crate::views::journal::UiState::new(),
            saisie: crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui()),
            budget: crate::views::budget::UiState::nouveau(crate::worker::aujourd_hui().year()),
            git: crate::views::git::UiState::nouveau(),
            git_donnees: None,
            config: crate::config::Config::charger(),
        };
        // Écran Git : même arme que l'ouverture par la barre (statut à la volée).
        if app.fenetres_ouvertes[Onglet::Git.index()] {
            app.cmd_tx.send(Command::GitStatut).ok();
        }
        app
    }

    fn demander_rechargement(&mut self) {
        self.en_chargement = true;
        self.cmd_tx.send(Command::Reload).ok();
    }

    /// Ouvre une fenêtre d'écran. L'écran Git relit son statut à chaque
    /// ouverture (arme du même genre que l'ancien clic d'onglet).
    fn ouvrir(&mut self, onglet: Onglet) {
        let index = onglet.index();
        if self.fenetres_ouvertes[index] {
            return; // déjà ouverte : egui la met au premier plan au clic
        }
        self.fenetres_ouvertes[index] = true;
        if onglet == Onglet::Git {
            self.en_chargement = true;
            self.flash = None;
            self.cmd_tx.send(Command::GitStatut).ok();
        }
    }

    /// Bascule une fenêtre (barre / Ctrl+N).
    fn basculer(&mut self, onglet: Onglet) {
        if self.fenetres_ouvertes[onglet.index()] {
            self.fenetres_ouvertes[onglet.index()] = false;
        } else {
            self.ouvrir(onglet);
        }
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
                    if self.saisie_en_vol {
                        // Un échec peut venir d'une écriture en vol (ledger
                        // illisible, échec post-écriture) : débloque la
                        // saisie, le brouillon reste corrigeable.
                        self.saisie_en_vol = false;
                    }
                }
                LoadOutcome::Ecrit { donnees, message } => {
                    self.donnees = Some(donnees);
                    self.echec = None;
                    self.flash = Some((message, false));
                    // Après une écriture, on ouvre le journal pour voir la
                    // ligne fraîchement écrite.
                    self.ouvrir(Onglet::Journal);
                    self.journal.selection = None;
                    if self.saisie_en_vol {
                        // Brouillon réinitialisé : prêt pour la saisie
                        // suivante. Uniquement si l'Ecrit est bien le
                        // résultat d'une saisie (un `Ecrit` budgétaire ne
                        // doit pas effacer un brouillon en cours d'édition).
                        self.saisie =
                            crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
                        self.saisie_en_vol = false;
                    }
                }
                LoadOutcome::Refuse(refus) => {
                    self.flash = Some((refus, true));
                    // Refus d'écriture : la saisie est débloquée, le brouillon
                    // est conservé pour correction (la remise à zéro n'a lieu
                    // qu'à la confirmation, cf. `Ecrit`).
                    self.saisie_en_vol = false;
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
            // Barre des écrans (fenêtres ouvertes/fermées) + flash.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (index, onglet) in Onglet::TOUS.into_iter().enumerate() {
                    let reponse =
                        ui.toggle_value(&mut self.fenetres_ouvertes[index], onglet.libelle());
                    if reponse.changed() && self.fenetres_ouvertes[index] && onglet == Onglet::Git {
                        // Écran Git : re-lit son statut à chaque ouverture.
                        self.en_chargement = true;
                        self.flash = None;
                        self.cmd_tx.send(Command::GitStatut).ok();
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

            if self.donnees.is_none() {
                // Ledger pas encore chargé (ou échec de chargement).
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
            } else if self.fenetres_ouvertes.iter().all(|ouvert| !ouvert) {
                ui.label(
                    RichText::new("Aucun écran ouvert — Ctrl+1 à Ctrl+6 ou la barre au-dessus.")
                        .weak(),
                );
            }
        });

        // Raccourcis clavier Ctrl+1..6 : bascule de la fenêtre d'écran.
        const TOUCHES: [egui::Key; Onglet::TOUS.len()] = [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
        ];
        if self.donnees.is_some() {
            for (touche, onglet) in TOUCHES.into_iter().zip(Onglet::TOUS) {
                if ui.input(|entree| entree.modifiers.ctrl && entree.key_pressed(touche)) {
                    self.basculer(onglet);
                }
            }
        }

        // Les écrans ouverts, en fenêtres flottantes (mémorisation position/
        // taille par la persistance egui/eframe).
        let donnees = self.donnees.clone();
        if let Some(donnees) = donnees {
            for (index, onglet) in Onglet::TOUS.into_iter().enumerate() {
                // Tampon pour la fermeture par la croix de la fenêtre : egui
                // écrit dans ce bool ; on le reporte dans l'état après coup
                // (emprunt partagé avec la fermeture de la vue).
                let mut ouvert = self.fenetres_ouvertes[index];
                if !ouvert {
                    continue;
                }
                let fenetre = egui::Window::new(onglet.libelle())
                    .open(&mut ouvert)
                    .resizable(true)
                    // Defaults seulement tant qu'egui n'a pas d'état mémorisé
                    // (après un redimensionnement utilisateur, ils sont
                    // ignorés) : le Tableau de bord démarre quasi plein
                    // écran, les autres écrans à 900×620.
                    .default_size(if onglet == Onglet::TableauDeBord {
                        ui.ctx().viewport_rect().shrink(40.0).size()
                    } else {
                        egui::vec2(900.0, 620.0)
                    })
                    .min_width(460.0);
                fenetre.show(ui.ctx(), |ui| {
                    self.vue(ui, onglet, &donnees);
                });
                self.fenetres_ouvertes[index] = ouvert;
            }
        }
    }
}

impl FoyerApp {
    /// Dessine l'écran demandé (contenu d'une fenêtre). Ancien corps du
    /// match d'onglets.
    fn vue(&mut self, ui: &mut egui::Ui, onglet: Onglet, donnees: &Arc<UiData>) {
        match onglet {
            Onglet::TableauDeBord => {
                crate::views::dashboard::afficher(ui, &donnees.tableau_de_bord);
            }
            Onglet::Journal => {
                let action =
                    crate::views::journal::afficher(ui, &donnees.journal, &mut self.journal);
                match action {
                    Some(crate::views::journal::ActionJournal::Modifier(index)) => {
                        // Pré-remplit la saisie depuis la ligne du journal.
                        if let Some(ligne) =
                            donnees.journal.lignes.iter().find(|l| l.index == index)
                        {
                            self.saisie = crate::views::saisie::UiState::modification(ligne);
                            self.ouvrir(Onglet::Saisie);
                        }
                    }
                    Some(crate::views::journal::ActionJournal::Supprimer(index)) => {
                        self.demander_suppression(index);
                    }
                    None => {}
                }
            }
            Onglet::Saisie => {
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
                        self.saisie =
                            crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
                        // Annulation pendant une écriture en vol : déverrouille
                        // la saisie ; l'`Ecrit` à venir ne réécrasera pas un
                        // nouveau brouillon (le reset n'a lieu que si la
                        // saisie était en vol au moment du résultat).
                        self.saisie_en_vol = false;
                        self.fenetres_ouvertes[Onglet::Saisie.index()] = false;
                        self.ouvrir(Onglet::Journal);
                    }
                    None => {}
                }
            }
            Onglet::Budget => {
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
            Onglet::Rapports => {
                crate::views::rapports::afficher(ui, &donnees.rapports);
            }
            Onglet::Git => {
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
                        // Écran Git pas encore chargé : spinner (l'ouverture
                        // de la fenêtre a déjà envoyé GitStatut).
                        ui.centered_and_justified(|ui| {
                            ui.add(egui::widgets::Spinner::new().size(30.0));
                        });
                    }
                }
            }
        }
    }
}

impl FoyerApp {
    /// Envoie le brouillon au worker (création ou modification selon le mode).
    fn envoyer_brouillon(&mut self) {
        if self.saisie_en_vol {
            return; // une écriture est déjà en vol : pas de doublon
        }
        self.saisie_en_vol = true;
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
        // Le brouillon n'est PAS réinitialisé ici : en cas de refus
        // (compte fermé, pré-vérification du writer…), l'utilisateur garde
        // sa saisie pour la corriger. La remise à zéro n'a lieu qu'à la
        // réception de `LoadOutcome::Ecrit` (cf. `logic`).
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

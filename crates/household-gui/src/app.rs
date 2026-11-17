//! Application TUI ratatui : état, canaux, écrans, bannière d'erreurs.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use household_core::ResolutionLedger;

use crate::viewmodel::UiData;
use crate::views::{budget, dashboard, git, graphiques, journal, rapports, saisie};
use crate::worker::{Command, LoadOutcome};

/// Durée d'affichage d'un flash de succès ; les erreurs restent affichées
/// (l'utilisateur doit les lire) tant qu'un autre message ne les remplace.
const FLASH_SUCCES: Duration = Duration::from_secs(4);

/// Les écrans de l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Onglet {
    TableauDeBord,
    Journal,
    Saisie,
    Budget,
    Rapports,
    Graphiques,
    Git,
}

impl Onglet {
    const TOUS: [Onglet; 7] = [
        Onglet::TableauDeBord,
        Onglet::Journal,
        Onglet::Saisie,
        Onglet::Budget,
        Onglet::Rapports,
        Onglet::Graphiques,
        Onglet::Git,
    ];

    fn libelle(self) -> &'static str {
        match self {
            Self::TableauDeBord => "Tableau de bord",
            Self::Journal => "Journal",
            Self::Saisie => "Saisie",
            Self::Budget => "Budget",
            Self::Rapports => "Rapports",
            Self::Graphiques => "Graphiques",
            Self::Git => "Git",
        }
    }

    #[must_use]
    pub fn index(self) -> usize {
        Self::TOUS
            .iter()
            .position(|o| *o == self)
            .expect("Onglet::TOUS contient tous les onglets")
    }

    #[must_use]
    pub fn depuis_env(var: Option<&str>) -> Option<Self> {
        match var.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("journal") => Some(Self::Journal),
            Some("saisie") => Some(Self::Saisie),
            Some("budget") => Some(Self::Budget),
            Some("rapports") => Some(Self::Rapports),
            Some("graphiques") | Some("graphes") => Some(Self::Graphiques),
            Some("git") => Some(Self::Git),
            Some("dashboard") | Some("tableau") => Some(Self::TableauDeBord),
            _ => None,
        }
    }
}

/// L'application.
pub struct App {
    main: PathBuf,
    resolu: ResolutionLedger,
    cmd_tx: Sender<Command>,
    outcome_rx: Receiver<LoadOutcome>,
    _watcher: notify::RecommendedWatcher,

    donnees: Option<Arc<UiData>>,
    echec: Option<String>,
    en_chargement: bool,
    flash: Option<(String, bool)>,
    /// Instant du dernier flash (pour l'expiration des succès).
    flash_depuis: Option<Instant>,
    /// Position d'affichage (0-based dans les lignes affichées du journal)
    /// de la transaction dont la suppression attend une confirmation modale.
    confirmation_suppression: Option<usize>,

    ecran_actif: usize,
    params_ledger: bool,
    saisie_en_vol: bool,
    tableau_de_bord: crate::views::dashboard::UiState,
    journal: crate::views::journal::UiState,
    saisie: crate::views::saisie::UiState,
    budget: crate::views::budget::UiState,
    git: crate::views::git::UiState,
    graphiques: crate::views::graphiques::UiState,
    git_donnees: Option<crate::worker::GitDonnees>,
    config: crate::config::Config,

    /// Zones cliquables enregistrées pendant le rendu, pour la gestion de la souris.
    zones_cliquables: Vec<(Rect, ActionClique)>,
}

/// Action déclenchée par un clic souris sur une zone cliquable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionClique {
    /// Changer d'onglet (index dans Onglet::TOUS).
    ChangerOnglet(usize),
    /// Sélectionner une ligne du journal (index dans donnees.journal.lignes).
    JournalLigne(usize),
    /// Sélectionner une ligne de postings dans la saisie.
    SaisieSelection(usize),
    /// Sélectionner une ligne de la grille budgétaire.
    BudgetLigne(usize),
    /// Changer l'onglet des graphiques.
    GraphiquesOnglet(usize),
    /// Valider un commit git.
    GitCommit(String),
    /// Recharger le statut git.
    GitRefresh,
    /// Aucune action.
    Aucun,
}

impl App {
    pub fn new(
        resolu: ResolutionLedger,
        onglet_initial: Option<Onglet>,
        params_ledger: bool,
    ) -> Self {
        let main = resolu.chemin.clone();
        let (cmd_tx, outcome_rx) = crate::worker::demarrer(main.clone());
        let watcher = crate::worker::demarrer_watcher(&main, cmd_tx.clone());

        let mut ecran_actif = Onglet::TableauDeBord.index();
        if let Some(onglet) = onglet_initial {
            ecran_actif = onglet.index();
        }

        let app = Self {
            main,
            resolu,
            cmd_tx,
            outcome_rx,
            _watcher: watcher,
            donnees: None,
            echec: None,
            en_chargement: true,
            flash: None,
            flash_depuis: None,
            confirmation_suppression: None,
            ecran_actif,
            params_ledger,
            saisie_en_vol: false,
            tableau_de_bord: crate::views::dashboard::UiState::default(),
            journal: crate::views::journal::UiState::new(),
            saisie: crate::views::saisie::UiState::default(),
            budget: crate::views::budget::UiState::nouveau(crate::worker::aujourd_hui().year()),
            git: crate::views::git::UiState::nouveau(),
            graphiques: crate::views::graphiques::UiState::nouveau(),
            git_donnees: None,
            config: crate::config::Config::charger(),
            zones_cliquables: Vec::new(),
        };

        if app.ecran_actif == Onglet::Git.index() {
            app.cmd_tx.send(Command::GitStatut).ok();
        }

        app
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), String> {
        loop {
            self.expire_flash();
            terminal.draw(|f| self.ui(f)).map_err(|e| e.to_string())?;

            while let Ok(outcome) = self.outcome_rx.try_recv() {
                self.traiter_outcome(outcome);
            }

            if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
                match event::read().map_err(|e| e.to_string())? {
                    // Seuls les appuis comptent : certains terminaux/OS
                    // émettent aussi des Release — sans filtre, chaque
                    // touche serait traitée deux fois.
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if self.on_key(key) {
                            break Ok(());
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.on_mouse(mouse);
                    }
                    _ => {}
                }
            }
        }
    }

    fn traiter_outcome(&mut self, outcome: LoadOutcome) {
        match outcome {
            LoadOutcome::Pret(donnees) => {
                self.donnees = Some(donnees);
                self.echec = None;
            }
            LoadOutcome::Echec(echec) => {
                self.echec = Some(echec);
                self.saisie_en_vol = false;
            }
            LoadOutcome::Ecrit { donnees, message } => {
                self.donnees = Some(donnees);
                self.echec = None;
                self.notifier(message, false);
                self.ecran_actif = Onglet::Journal.index();
                if self.saisie_en_vol {
                    self.saisie =
                        crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
                    self.saisie_en_vol = false;
                }
            }
            LoadOutcome::EcritBudget { donnees, message } => {
                // Écriture budgétaire : on reste sur l'écran Budget.
                self.donnees = Some(donnees);
                self.echec = None;
                self.notifier(message, false);
            }
            LoadOutcome::Refuse(refus) => {
                self.notifier(refus, true);
                self.saisie_en_vol = false;
            }
            LoadOutcome::Git { donnees, message } => {
                self.git_donnees = Some(donnees.clone());
                self.en_chargement = false;
                // Pré-remplir le message de commit avec la suggestion.
                self.git.set_message_suggere(&donnees.message_suggere);
                if let Some((texte, est_erreur)) = message {
                    self.notifier(texte, est_erreur);
                }
            }
        }
        self.en_chargement = false;
    }

    /// Affiche un message flash : 4 s pour un succès (voir
    /// [`FLASH_SUCCES`]), persistant pour une erreur.
    fn notifier(&mut self, message: impl Into<String>, est_erreur: bool) {
        self.flash = Some((message.into(), est_erreur));
        self.flash_depuis = Some(Instant::now());
    }

    /// Efface un flash de succès expiré ; les erreurs ne s'effacent pas
    /// d'elles-mêmes.
    fn expire_flash(&mut self) {
        let doit_exprier = self
            .flash_depuis
            .is_some_and(|depuis| depuis.elapsed() >= FLASH_SUCCES)
            && self
                .flash
                .as_ref()
                .is_some_and(|(_, est_erreur)| !*est_erreur);
        if doit_exprier {
            self.flash = None;
            self.flash_depuis = None;
        }
    }

    /// Change d'écran, avec le cas particulier Git (recharge le statut).
    fn changer_ecran(&mut self, index: usize) {
        self.ecran_actif = index;
        if Onglet::TOUS[index] == Onglet::Git {
            self.en_chargement = true;
            self.flash = None;
            self.cmd_tx.send(Command::GitStatut).ok();
        }
    }

    fn on_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl+C : quitter (convention terminal), au-dessus de tout, y
        // compris de la modale de confirmation ci-dessous.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }

        // Confirmation de suppression modale : Entrée confirme, toute autre
        // touche annule (et est consommée).
        if self.confirmation_suppression.is_some() {
            if key.code == KeyCode::Enter {
                let pos = self.confirmation_suppression.take().unwrap_or_default();
                let index = self
                    .donnees
                    .as_ref()
                    .and_then(|d| d.journal.lignes.get(pos))
                    .map(|l| l.index);
                if let Some(index) = index {
                    self.demander_suppression(index);
                }
            } else {
                self.confirmation_suppression = None;
            }
            return false;
        }

        // Ctrl+1..7 et F1..F7 : changement d'écran direct, prioritaire sur
        // tout (marche aussi depuis un formulaire).
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && let KeyCode::Char(c) = key.code
            && let Some(digit) = c.to_digit(10)
            && digit >= 1
        {
            let index = (digit - 1) as usize;
            if index < Onglet::TOUS.len() {
                self.changer_ecran(index);
            }
            return false;
        }
        if let KeyCode::F(n) = key.code {
            // F1..F7 → écrans 1..7 (F8+ ignoré, jamais de décalage).
            let index = usize::from(n).saturating_sub(1);
            if index < Onglet::TOUS.len() {
                self.changer_ecran(index);
            }
            return false;
        }

        // Délégation par écran actif. Les écrans avec champ texte (Saisie,
        // Git) consomment TOUTES les touches : les lettres q/r/l y sont du
        // texte, pas des commandes globales.
        if self.ecran_actif == Onglet::TableauDeBord.index() {
            // ↑/↓ et Page↑/Page↓ font défiler le tableau « Budget vs
            // réel » ; les autres touches restent globales (q, r, l, Esc).
            let nb = self
                .donnees
                .as_ref()
                .map_or(0, |d| d.tableau_de_bord.budget_mois.len());
            if dashboard::on_key(key, &mut self.tableau_de_bord, nb) {
                return false;
            }
        }
        if self.ecran_actif == Onglet::Saisie.index() {
            let action = self.saisie.on_key(
                key,
                self.donnees
                    .as_ref()
                    .map(|d| d.comptes.as_slice())
                    .unwrap_or(&[]),
                self.donnees
                    .as_ref()
                    .map(|d| d.payees.as_slice())
                    .unwrap_or(&[]),
            );
            match action {
                Some(crate::views::saisie::Action::Enregistrer) => {
                    self.envoyer_brouillon();
                }
                Some(crate::views::saisie::Action::Annuler) => {
                    self.saisie =
                        crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
                    self.saisie_en_vol = false;
                    self.ecran_actif = Onglet::Journal.index();
                }
                Some(crate::views::saisie::Action::Aucun) | None => {}
            }
            return false;
        }

        if self.ecran_actif == Onglet::Git.index()
            && let Some(action) = crate::views::git::on_key(key, &mut self.git)
        {
            match action {
                crate::views::git::ActionGit::Actualiser => {
                    self.en_chargement = true;
                    self.flash = None;
                    self.cmd_tx.send(Command::GitStatut).ok();
                }
                crate::views::git::ActionGit::Commit(message) => {
                    self.en_chargement = true;
                    self.flash = None;
                    self.cmd_tx.send(Command::GitCommit { message }).ok();
                }
                crate::views::git::ActionGit::Aucun => {}
            }
            return false;
        }

        if self.ecran_actif == Onglet::Graphiques.index()
            && let Some(action) = crate::views::graphiques::on_key(key, &mut self.graphiques)
        {
            match action {
                crate::views::graphiques::ActionGraphiques::ChangerOnglet(n) => {
                    self.graphiques.onglet = n;
                }
                crate::views::graphiques::ActionGraphiques::BasculeRepartition => {
                    self.graphiques.bascule_repartition();
                }
                crate::views::graphiques::ActionGraphiques::Aucun => {}
            }
            return false;
        }

        if self.ecran_actif == Onglet::Budget.index() {
            let grille = self
                .donnees
                .as_ref()
                .and_then(|d| d.grilles.get(&self.budget.annee));
            // Édition de cellule ouverte : toutes les touches y vont (le
            // texte d'un montant peut contenir r, l, e, x…) et sont
            // consommées. Sinon, seules navigation/édition sont prises, les
            // autres (q, r, l) restent globales.
            let pour_le_budget = self.budget.edition.is_some()
                || matches!(
                    key.code,
                    KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Char('e')
                );
            if pour_le_budget {
                let action = budget::on_key(
                    key,
                    &mut self.budget,
                    grille,
                    &|famille| self.config.rollover_actif(famille),
                    &self.cmd_tx,
                );
                if let Some(budget::ActionBudget::Cellule {
                    annee,
                    date,
                    compte,
                    montant,
                }) = action
                {
                    // Part uniquement à la validation explicite (Entrée) :
                    // ouverture, annulation et montant invalide ne
                    // produisent aucune commande.
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
                return false;
            }
        }

        // Journal : flèches/Entrée/Suppr. La sélection navigue dans les
        // MAX_LIGNES lignes affichées, pas dans tout l'historique. Les autres
        // touches tombent dans les commandes globales ci-dessous.
        if self.ecran_actif == Onglet::Journal.index()
            && matches!(
                key.code,
                KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Delete | KeyCode::Backspace
            )
        {
            let nb = self
                .donnees
                .as_ref()
                .map_or(0, |d| d.journal.lignes.len().min(journal::MAX_LIGNES));
            match journal::on_key(key, &mut self.journal, nb) {
                Some(journal::ActionJournal::Modifier(pos)) => {
                    let ligne = self
                        .donnees
                        .as_ref()
                        .and_then(|d| d.journal.lignes.get(pos))
                        .cloned();
                    if let Some(ligne) = ligne {
                        self.saisie = crate::views::saisie::UiState::modification(&ligne);
                        self.ecran_actif = Onglet::Saisie.index();
                    }
                }
                Some(journal::ActionJournal::Supprimer(pos)) => {
                    self.confirmer_suppression(pos);
                }
                Some(journal::ActionJournal::Aucun) | None => {}
            }
            return false;
        }

        // Commandes globales (hors écrans à champ texte).
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Esc => {
                // Esc = retour au tableau de bord, JAMAIS quitter (l'écran
                // Saisie et l'éditeur de cellule Budget interceptent Esc
                // avant pour annuler). Ctrl+C et q restent les issues.
                if self.ecran_actif != Onglet::TableauDeBord.index() {
                    self.changer_ecran(Onglet::TableauDeBord.index());
                }
            }
            KeyCode::Char('r') => self.demander_rechargement(),
            KeyCode::Char('l') => self.params_ledger = !self.params_ledger,
            _ => {}
        }

        false
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        use crossterm::event::MouseEventKind;

        // La modale de confirmation intercepte tout : un clic gauche annule
        // (cohérent avec « toute autre touche annule » au clavier).
        if self.confirmation_suppression.is_some() {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                self.confirmation_suppression = None;
            }
            return;
        }

        // Molette : Journal (déplace la sélection, comme les flèches) et
        // Tableau de bord (fait défiler le tableau « Budget vs réel »).
        // Les autres écrans n'ont pas de défilement pour l'instant.
        if matches!(
            mouse.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            let delta = if mouse.kind == MouseEventKind::ScrollUp {
                -1
            } else {
                1
            };
            if self.ecran_actif == Onglet::Journal.index() {
                let nb = self
                    .donnees
                    .as_ref()
                    .map_or(0, |d| d.journal.lignes.len().min(journal::MAX_LIGNES));
                if delta < 0 {
                    self.journal.monter(nb);
                } else {
                    self.journal.descendre(nb);
                }
                return;
            }
            if self.ecran_actif == Onglet::TableauDeBord.index() {
                let nb = self
                    .donnees
                    .as_ref()
                    .map_or(0, |d| d.tableau_de_bord.budget_mois.len());
                self.tableau_de_bord.defiler(nb, delta);
                return;
            }
        }

        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            let point = ratatui::layout::Position {
                x: mouse.column,
                y: mouse.row,
            };

            for (zone, action) in &self.zones_cliquables {
                if zone.contains(point) {
                    let action = action.clone();
                    match action {
                        ActionClique::ChangerOnglet(n) => {
                            self.changer_ecran(n);
                        }
                        ActionClique::JournalLigne(index) => {
                            let ligne = self.donnees.as_ref().and_then(|d| {
                                d.journal.lignes.iter().find(|l| l.index == index).cloned()
                            });
                            if let Some(ligne) = ligne {
                                self.saisie = crate::views::saisie::UiState::modification(&ligne);
                                self.ecran_actif = Onglet::Saisie.index();
                            }
                        }
                        ActionClique::SaisieSelection(index) => {
                            self.saisie.posting_selectionne = index;
                            self.saisie.champ_actif = crate::views::saisie::Champ::Postings;
                        }
                        ActionClique::BudgetLigne(index) => {
                            self.budget.ligne_selectionne = index;
                        }
                        ActionClique::GraphiquesOnglet(n) => {
                            self.graphiques.onglet = n;
                        }
                        ActionClique::GitCommit(message) => {
                            self.en_chargement = true;
                            self.flash = None;
                            self.cmd_tx.send(Command::GitCommit { message }).ok();
                        }
                        ActionClique::GitRefresh => {
                            self.en_chargement = true;
                            self.flash = None;
                            self.cmd_tx.send(Command::GitStatut).ok();
                        }
                        ActionClique::Aucun => {}
                    }
                    return; // Un seul clic traité à la fois
                }
            }
        }
    }

    #[expect(dead_code)]
    fn traiter_action_journal(&mut self, action: crate::views::journal::ActionJournal) {
        match action {
            crate::views::journal::ActionJournal::Modifier(index) => {
                let donnees = self.donnees.as_ref();
                if let Some(ligne) = donnees
                    .and_then(|d| d.journal.lignes.iter().find(|l| l.index == index).cloned())
                {
                    self.saisie = crate::views::saisie::UiState::modification(&ligne);
                    self.ecran_actif = Onglet::Saisie.index();
                }
            }
            crate::views::journal::ActionJournal::Supprimer(index) => {
                self.demander_suppression(index);
            }
            crate::views::journal::ActionJournal::Aucun => {}
        }
    }

    #[expect(dead_code)]
    fn traiter_action_saisie(&mut self, action: crate::views::saisie::Action) {
        match action {
            crate::views::saisie::Action::Enregistrer => {
                self.envoyer_brouillon();
            }
            crate::views::saisie::Action::Annuler => {
                self.saisie = crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
                self.saisie_en_vol = false;
                self.ecran_actif = Onglet::Journal.index();
            }
            crate::views::saisie::Action::Aucun => {}
        }
    }

    #[expect(dead_code)]
    fn traiter_action_budget(&mut self, action: crate::views::budget::ActionBudget) {
        match action {
            crate::views::budget::ActionBudget::Cellule {
                annee,
                date,
                compte,
                montant,
            } => {
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
            crate::views::budget::ActionBudget::GenererAnnee {
                source,
                cible,
                pourcent,
                pas,
            } => {
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
            crate::views::budget::ActionBudget::BasculerRollover(famille) => {
                match self.config.basculer_rollover(&famille) {
                    Ok(()) => {
                        self.notifier(
                            format!(
                                "rollover {} — préférence enregistrée",
                                if self.config.rollover_actif(&famille) {
                                    "activé"
                                } else {
                                    "désactivé"
                                }
                            ),
                            false,
                        );
                    }
                    Err(e) => self.notifier(e, true),
                }
            }
            crate::views::budget::ActionBudget::Aucun => {}
        }
    }

    #[expect(dead_code)]
    fn traiter_action_graphiques(&mut self, action: crate::views::graphiques::ActionGraphiques) {
        match action {
            crate::views::graphiques::ActionGraphiques::ChangerOnglet(n) => {
                self.graphiques.onglet = n;
            }
            crate::views::graphiques::ActionGraphiques::BasculeRepartition => {
                self.graphiques.bascule_repartition();
            }
            crate::views::graphiques::ActionGraphiques::Aucun => {}
        }
    }

    #[expect(dead_code)]
    fn traiter_action_git(&mut self, action: crate::views::git::ActionGit) {
        match action {
            crate::views::git::ActionGit::Actualiser => {
                self.en_chargement = true;
                self.flash = None;
                self.cmd_tx.send(Command::GitStatut).ok();
            }
            crate::views::git::ActionGit::Commit(message) => {
                self.en_chargement = true;
                self.flash = None;
                self.cmd_tx.send(Command::GitCommit { message }).ok();
            }
            crate::views::git::ActionGit::Aucun => {}
        }
    }

    fn demander_rechargement(&mut self) {
        self.en_chargement = true;
        self.cmd_tx.send(Command::Reload).ok();
    }

    fn envoyer_brouillon(&mut self) {
        if self.saisie_en_vol {
            return;
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
    }

    /// Ouvre la modale de confirmation pour la transaction affichée à la
    /// position `pos` (index d'affichage dans les lignes affichées du journal).
    fn confirmer_suppression(&mut self, pos: usize) {
        if self
            .donnees
            .as_ref()
            .is_some_and(|d| pos < d.journal.lignes.len())
        {
            self.confirmation_suppression = Some(pos);
        }
    }

    /// Supprime la transaction d'index `index` (index du ledger, pas la
    /// position d'affichage) — appelé uniquement après confirmation.
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

    fn ui(&mut self, f: &mut Frame) {
        self.zones_cliquables.clear();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // onglets (une ligne)
                Constraint::Length(4), // en-tête (badge exemple, chemin)
                Constraint::Min(0),    // contenu
                Constraint::Length(1), // flash
            ])
            .split(f.size());

        Self::dessine_barre_ecrans(f, chunks[0], self.ecran_actif, &mut self.zones_cliquables);
        self.dessine_entete(f, chunks[1]);
        self.dessine_contenu(f, chunks[2]);
        self.dessine_flash(f, chunks[3]);
        if self.confirmation_suppression.is_some() {
            self.dessine_modal(f, f.size());
        }
    }

    /// Modale de confirmation de suppression, superposée au contenu.
    fn dessine_modal(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let Some(pos) = self.confirmation_suppression else {
            return;
        };
        let resume = self
            .donnees
            .as_ref()
            .and_then(|d| d.journal.lignes.get(pos))
            .map(|l| format!("{} — {}", l.date, l.libelle))
            .unwrap_or_default();

        let zone = rect_centre(72, 5, area);
        f.render_widget(Clear, zone);
        let texte = vec![
            Line::from(format!(" Supprimer : {resume} ")),
            Line::from(" Entrée : supprimer — toute autre touche : annuler "),
        ];
        let style = Style::default().fg(Color::White).bg(Color::Red);
        let modal = Paragraph::new(texte).style(style).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirmer la suppression ")
                .style(style),
        );
        f.render_widget(modal, zone);
    }

    fn dessine_entete(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mut lignes: Vec<ListItem> = vec![];

        if self.resolu.mode.est_exemple() {
            lignes.push(
                ListItem::new(" LEDGER D'EXEMPLE — données fictives").style(
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                ),
            );
        }

        let chemin = format!(
            " {} — {}",
            self.main.display(),
            self.resolu.source.libelle()
        );
        lignes.push(ListItem::new(chemin).style(Style::default().fg(Color::Gray)));

        let list = List::new(lignes).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Comptabilité du foyer "),
        );
        f.render_widget(list, area);
    }

    fn dessine_contenu(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        if self.donnees.is_none() {
            let texte = if self.en_chargement {
                "Chargement du ledger…"
            } else {
                "⚠ Chargement impossible — voir les messages ci-dessus."
            };
            let paragraphe = Paragraph::new(texte)
                .block(Block::default().borders(Borders::ALL).title(" Contenu "))
                .style(if self.en_chargement {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Red)
                });
            f.render_widget(paragraphe, area);
            return;
        }

        let donnees = self.donnees.as_ref().unwrap();

        match Onglet::TOUS[self.ecran_actif] {
            Onglet::TableauDeBord => {
                dashboard::afficher(
                    f,
                    area,
                    &donnees.tableau_de_bord,
                    &mut self.tableau_de_bord,
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Journal => {
                journal::afficher(
                    f,
                    area,
                    &donnees.journal,
                    &mut self.journal,
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Saisie => {
                saisie::afficher(
                    f,
                    area,
                    &mut self.saisie,
                    &donnees.comptes,
                    &donnees.payees,
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Budget => {
                budget::afficher(
                    f,
                    area,
                    donnees.grilles.get(&self.budget.annee),
                    &mut self.budget,
                    &|famille| self.config.rollover_actif(famille),
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Rapports => {
                rapports::afficher(f, area, &donnees.rapports, &mut self.zones_cliquables);
            }
            Onglet::Graphiques => {
                graphiques::afficher(
                    f,
                    area,
                    donnees,
                    &mut self.graphiques,
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Git => match &self.git_donnees {
                Some(git) => {
                    git::afficher(f, area, git, &mut self.git, &mut self.zones_cliquables);
                }
                None => {
                    let paragraphe = Paragraph::new("Chargement des données Git…")
                        .block(Block::default().borders(Borders::ALL).title(" Git "))
                        .style(Style::default().fg(Color::Gray));
                    f.render_widget(paragraphe, area);
                }
            },
        }
    }

    fn dessine_barre_ecrans(
        f: &mut Frame,
        area: ratatui::layout::Rect,
        ecran_actif: usize,
        zones_cliquables: &mut Vec<(Rect, ActionClique)>,
    ) {
        let titres = titres_onglets();

        // Zones cliquables calculées avec la MÊME arithmétique que le rendu
        // ci-dessous : chaque onglet occupe son libellé encadré d'un espace,
        // séparés par une barre verticale de 1 colonne.
        for (zone, index) in zones_onglets(&titres, area) {
            zones_cliquables.push((zone, ActionClique::ChangerOnglet(index)));
        }

        let mut spans: Vec<Span> = Vec::new();
        for (i, titre) in titres.iter().enumerate() {
            let style = if i == ecran_actif {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(format!(" {titre} "), style));
            if i + 1 < titres.len() {
                spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
            }
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn dessine_flash(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        // Message prioritaire, sinon dernière erreur de chargement. Sans
        // bordure : la rangée fait 1 ligne, une bordure masquerait le texte.
        if let Some((message, est_erreur)) = &self.flash {
            let style = if *est_erreur {
                Style::default().fg(Color::White).bg(Color::Red)
            } else {
                Style::default().fg(Color::White).bg(Color::Green)
            };
            let paragraphe = Paragraph::new(format!(" {message} ")).style(style);
            f.render_widget(paragraphe, area);
        } else if let Some(echec) = &self.echec {
            let paragraphe = Paragraph::new(format!(" Erreur : {echec} "))
                .style(Style::default().bg(Color::Red));
            f.render_widget(paragraphe, area);
        } else {
            // Barre d'aide permanente : la découvrabilité des commandes
            // globales ne coûte qu'une ligne.
            let aide = Paragraph::new(
                " Ctrl+1..7 ou F1..F7 : écrans · Esc : retour au tableau de bord · q : quitter",
            )
            .style(Style::default().fg(Color::DarkGray));
            f.render_widget(aide, area);
        }
    }
}

/// Libellés des onglets, sans le padding d'affichage — partagé entre le
/// rendu de la barre d'onglets et les tests de cohérence clic/rendu.
#[must_use]
pub fn titres_onglets() -> Vec<String> {
    Onglet::TOUS
        .iter()
        .enumerate()
        .map(|(i, onglet)| format!("{ctrl} {nom}", ctrl = i + 1, nom = onglet.libelle()))
        .collect()
}

/// Rectangle centré, borné à l'aire — pour la modale de confirmation.
#[must_use]
fn rect_centre(largeur: u16, hauteur: u16, area: Rect) -> Rect {
    let largeur = largeur.min(area.width);
    let hauteur = hauteur.min(area.height);
    Rect {
        x: area.x + (area.width - largeur) / 2,
        y: area.y + (area.height - hauteur) / 2,
        width: largeur,
        height: hauteur,
    }
}

/// Zones cliquables des onglets pour une aire d'écran donnée.
///
/// Même arithmétique que le rendu de [`App::dessine_barre_ecrans`] : chaque
/// libellé est encadré d'un espace (padding), onglets séparés par une barre
/// verticale de 1 colonne. Les onglets tronqués par un écran étroit n'ont
/// pas de zone (un clic y serait hors rendu de toute façon).
#[must_use]
pub fn zones_onglets(titres: &[String], area: Rect) -> Vec<(Rect, usize)> {
    let mut zones = Vec::new();
    let mut x = area.left() as usize;
    let droite = area.right() as usize;
    for (i, titre) in titres.iter().enumerate() {
        // Libellés ASCII (conventions du dépôt) : nombre de caractères =
        // largeur d'affichage.
        let largeur = titre.chars().count() + 2; // padding 1 colonne de chaque côté
        if x + largeur > droite {
            break;
        }
        zones.push((
            Rect::new(
                x as u16,
                area.top(),
                u16::try_from(largeur).unwrap_or(u16::MAX),
                1,
            ),
            i,
        ));
        x += largeur + 1; // + la barre verticale de séparation
    }
    zones
}

/// Tests unitaires du routage clavier et du flash (accès privé aux champs).
#[cfg(test)]
mod tests {
    use super::*;

    /// App de test : chemin de ledger inexistant — le worker répond
    /// `Echec` tout de suite et le watcher reste désactivé (dossier
    /// absent). Suffisant pour tester le clavier et le flash sans ledger
    /// réel, ni toucher à l'exemple du dépôt.
    fn app_test() -> App {
        App::new(
            ResolutionLedger {
                chemin: std::env::temp_dir()
                    .join("inexistant-compta-test")
                    .join("main.beancount"),
                mode: household_core::ModeLedger::Reel,
                source: household_core::SourceResolution::Argument,
                avertissement: None,
            },
            None,
            false,
        )
    }

    fn touche(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn flash_succes_expire_apres_4_s() {
        let mut app = app_test();
        app.notifier("écrit ✓", false);
        app.expire_flash();
        assert!(app.flash.is_some(), "un succès frais ne s'efface pas");

        // Simule le passage des 4 s (pas d'attente réelle dans le test).
        app.flash_depuis = Some(Instant::now() - FLASH_SUCCES);
        app.expire_flash();
        assert!(app.flash.is_none(), "le succès expire après FLASH_SUCCES");
    }

    #[test]
    fn flash_erreur_ne_s_expire_pas() {
        let mut app = app_test();
        app.notifier("écriture refusée", true);
        app.flash_depuis = Some(Instant::now() - FLASH_SUCCES);
        app.expire_flash();
        assert!(
            app.flash.is_some(),
            "une erreur reste affichée jusqu'à remplacement"
        );
    }

    #[test]
    fn esc_retourne_au_tableau_de_bord_sans_quitter() {
        let mut app = app_test();
        app.ecran_actif = Onglet::Journal.index();

        let quitte = app.on_key(touche(KeyCode::Esc));
        assert!(!quitte, "Esc ne quitte jamais l'application");
        assert_eq!(
            app.ecran_actif,
            Onglet::TableauDeBord.index(),
            "Esc ramène au tableau de bord"
        );
    }

    #[test]
    fn esc_sur_tableau_de_bord_ne_change_rien() {
        let mut app = app_test();
        let quitte = app.on_key(touche(KeyCode::Esc));
        assert!(!quitte);
        assert_eq!(app.ecran_actif, Onglet::TableauDeBord.index());
    }

    #[test]
    fn q_quitte_et_ctrl_c_aussi() {
        let mut app = app_test();
        assert!(app.on_key(touche(KeyCode::Char('q'))), "q quitte");
        assert!(
            app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            "Ctrl+C quitte"
        );
    }

    #[test]
    fn touches_fonction_changent_d_ecran() {
        // F1..F7 = Ctrl+1..7 : F1 → Tableau de bord … F7 → Git.
        let attendus = [
            Onglet::TableauDeBord,
            Onglet::Journal,
            Onglet::Saisie,
            Onglet::Budget,
            Onglet::Rapports,
            Onglet::Graphiques,
            Onglet::Git,
        ];
        for (n, attendu) in attendus.iter().enumerate() {
            let mut app = app_test();
            app.ecran_actif = 0;
            let quitte = app.on_key(touche(KeyCode::F(n as u8 + 1)));
            assert!(!quitte);
            assert_eq!(
                app.ecran_actif,
                attendu.index(),
                "F{} doit ouvrir l'écran {}",
                n + 1,
                attendu.index() + 1
            );
        }
    }

    #[test]
    fn f8_et_au_dela_ne_font_rien() {
        let mut app = app_test();
        app.ecran_actif = Onglet::Budget.index();
        for n in [8u8, 9, 12] {
            let quitte = app.on_key(touche(KeyCode::F(n)));
            assert!(!quitte);
            assert_eq!(
                app.ecran_actif,
                Onglet::Budget.index(),
                "F{n} ne doit pas changer d'écran"
            );
        }
    }
}

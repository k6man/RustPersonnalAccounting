//! Application TUI ratatui : état, canaux, écrans, bannière d'erreurs.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};

use household_core::ResolutionLedger;

use crate::viewmodel::UiData;
use crate::views::{budget, dashboard, git, graphiques, journal, rapports, saisie};
use crate::worker::{Command, LoadOutcome};

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

    ecran_actif: usize,
    params_ledger: bool,
    saisie_en_vol: bool,
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
#[derive(Debug, Clone)]
pub enum ActionClique {
    /// Changer d'onglet (index dans Onglet::TOUS).
    ChangerOnglet(usize),
    /// Sélectionner une ligne du journal (index dans donnees.journal.lignes).
    JournalLigne(usize),
    /// Écrire dans la saisie (nouveau brouillon).
    SaisieNouveau,
    /// Modifier une transaction existante (index dans donnees.journal.lignes).
    SaisieModifier(usize),
    /// Modifier une cellule budgétaire.
    BudgetCellule {
        annee: i16,
        date: rustledger_core::NaiveDate,
        compte: String,
        montant: rust_decimal::Decimal,
    },
    /// Basculer le rollover d'une famille.
    BudgetRollover(String),
    /// Changer l'onglet des graphiques.
    GraphiquesOnglet(usize),
    /// Basculer année/mois dans les graphiques.
    GraphiquesBascule,
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
            ecran_actif,
            params_ledger,
            saisie_en_vol: false,
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
            terminal.draw(|f| self.ui(f)).map_err(|e| e.to_string())?;

            while let Ok(outcome) = self.outcome_rx.try_recv() {
                self.traiter_outcome(outcome);
            }

            if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
                match event::read().map_err(|e| e.to_string())? {
                    Event::Key(key) => {
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
                self.flash = Some((message, false));
                self.ecran_actif = Onglet::Journal.index();
                if self.saisie_en_vol {
                    self.saisie =
                        crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
                    self.saisie_en_vol = false;
                }
            }
            LoadOutcome::Refuse(refus) => {
                self.flash = Some((refus, true));
                self.saisie_en_vol = false;
            }
            LoadOutcome::Git { donnees, message } => {
                self.git_donnees = Some(donnees.clone());
                self.en_chargement = false;
                // Pré-remplir le message de commit avec la suggestion.
                self.git.set_message_suggere(&donnees.message_suggere);
                if let Some((texte, est_erreur)) = message {
                    self.flash = Some((texte, est_erreur));
                }
            }
        }
        self.en_chargement = false;
    }

    fn on_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('r') => {
                self.demander_rechargement();
                return false;
            }
            KeyCode::Char('l') => {
                self.params_ledger = !self.params_ledger;
                return false;
            }
            _ => {}
        }

        if key.modifiers == KeyModifiers::CONTROL
            && let KeyCode::Char(c) = key.code
            && let Some(digit) = c.to_digit(10)
        {
            let index = digit as usize - 1;
            if index < Onglet::TOUS.len() {
                self.ecran_actif = index;
                let onglet = Onglet::TOUS[index];
                if onglet == Onglet::Git {
                    self.en_chargement = true;
                    self.flash = None;
                    self.cmd_tx.send(Command::GitStatut).ok();
                }
                return false;
            }
        }

        // Délégation vers la vue active pour les touches spécifiques au
        // contexte (flèches, Entrée, Suppr…). L'emprunt de donnees est limité
        // à un bloc pour pouvoir modifier self ensuite.
        let action = {
            let donnees = self.donnees.as_ref();
            match donnees {
                Some(d) => {
                    let nb = d.journal.lignes.len();
                    crate::views::journal::on_key(key, &mut self.journal, nb)
                }
                None => None,
            }
        };

        match action {
            Some(crate::views::journal::ActionJournal::Modifier(index)) => {
                let donnees = self.donnees.as_ref();
                if let Some(ligne) = donnees
                    .and_then(|d| d.journal.lignes.iter().find(|l| l.index == index).cloned())
                {
                    self.saisie = crate::views::saisie::UiState::modification(&ligne);
                    self.ecran_actif = Onglet::Saisie.index();
                }
            }
            Some(crate::views::journal::ActionJournal::Supprimer(index)) => {
                self.demander_suppression(index);
            }
            _ => {}
        }

        // Délégation vers la vue Saisie pour l'édition des champs.
        if self.ecran_actif == Onglet::Saisie.index()
            && let Some(action) = self.saisie.on_key(
                key,
                self.donnees
                    .as_ref()
                    .map(|d| d.comptes.as_slice())
                    .unwrap_or(&[]),
                self.donnees
                    .as_ref()
                    .map(|d| d.payees.as_slice())
                    .unwrap_or(&[]),
            )
        {
            match action {
                crate::views::saisie::Action::Enregistrer => {
                    self.envoyer_brouillon();
                }
                crate::views::saisie::Action::Annuler => {
                    self.saisie =
                        crate::views::saisie::UiState::nouveau(crate::worker::aujourd_hui());
                    self.saisie_en_vol = false;
                    self.ecran_actif = Onglet::Journal.index();
                }
                crate::views::saisie::Action::Aucun => {}
            }
        }

        // Délégation vers la vue Graphiques pour les touches de navigation
        // entre mini-onglets (flèches gauche/droite, 1/2/3, t pour basculer
        // année/mois).
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
        }

        // Délégation vers la vue Git pour les touches (flèches, 1/2/3, t,
        // Entrée pour commiter, r pour actualiser).
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
        }

        false
    }

    fn on_mouse(&mut self, mouse: MouseEvent) -> bool {
        use crossterm::event::MouseEventKind;

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
                            self.ecran_actif = n;
                            let onglet = Onglet::TOUS[n];
                            if onglet == Onglet::Git {
                                self.en_chargement = true;
                                self.flash = None;
                                self.cmd_tx.send(Command::GitStatut).ok();
                            }
                        }
                        ActionClique::JournalLigne(index) => {
                            let donnees = self.donnees.as_ref();
                            if let Some(ligne) = donnees.and_then(|d| {
                                d.journal.lignes.iter().find(|l| l.index == index).cloned()
                            }) {
                                self.saisie = crate::views::saisie::UiState::modification(&ligne);
                                self.ecran_actif = Onglet::Saisie.index();
                            }
                        }
                        ActionClique::SaisieNouveau => {}
                        ActionClique::SaisieModifier(index) => {
                            let donnees = self.donnees.as_ref();
                            if let Some(ligne) = donnees.and_then(|d| {
                                d.journal.lignes.iter().find(|l| l.index == index).cloned()
                            }) {
                                self.saisie = crate::views::saisie::UiState::modification(&ligne);
                                self.ecran_actif = Onglet::Saisie.index();
                            }
                        }
                        ActionClique::BudgetCellule {
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
                        ActionClique::BudgetRollover(famille) => {
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
                        ActionClique::GraphiquesOnglet(n) => {
                            self.graphiques.onglet = n;
                        }
                        ActionClique::GraphiquesBascule => {
                            self.graphiques.bascule_repartition();
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
                    return false; // Un seul clic traité à la fois
                }
            }
        }

        false
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(f.size());

        self.dessine_entete(f, chunks[0]);
        self.dessine_contenu(f, chunks[1]);
        self.dessine_flash(f, chunks[2]);
    }

    fn dessine_entete(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mut lignes: Vec<ListItem> = vec![];

        lignes.push(
            ListItem::new(" Comptabilité du foyer ").style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        );

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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        Self::dessine_barre_ecrans(f, chunks[0], self.ecran_actif, &mut self.zones_cliquables);

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
            f.render_widget(paragraphe, chunks[1]);
            return;
        }

        let donnees = self.donnees.as_ref().unwrap();

        match Onglet::TOUS[self.ecran_actif] {
            Onglet::TableauDeBord => {
                dashboard::afficher(f, &donnees.tableau_de_bord, &mut self.zones_cliquables);
            }
            Onglet::Journal => {
                journal::afficher(
                    f,
                    &donnees.journal,
                    &mut self.journal,
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Saisie => {
                saisie::afficher(
                    f,
                    &mut self.saisie,
                    &donnees.comptes,
                    &donnees.payees,
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Budget => {
                budget::afficher(
                    f,
                    donnees.grilles.get(&self.budget.annee),
                    &mut self.budget,
                    &|famille| self.config.rollover_actif(famille),
                    &mut self.zones_cliquables,
                );
            }
            Onglet::Rapports => {
                rapports::afficher(f, &donnees.rapports, &mut self.zones_cliquables);
            }
            Onglet::Graphiques => {
                graphiques::afficher(f, donnees, &mut self.graphiques, &mut self.zones_cliquables);
            }
            Onglet::Git => match &self.git_donnees {
                Some(git) => {
                    git::afficher(f, git, &mut self.git, &mut self.zones_cliquables);
                }
                None => {
                    let paragraphe = Paragraph::new("Chargement des données Git…")
                        .block(Block::default().borders(Borders::ALL).title(" Git "))
                        .style(Style::default().fg(Color::Gray));
                    f.render_widget(paragraphe, chunks[1]);
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
        let titres: Vec<String> = Onglet::TOUS
            .iter()
            .enumerate()
            .map(|(i, onglet)| format!(" {ctrl} {nom}", ctrl = i + 1, nom = onglet.libelle()))
            .collect();

        let tabs = Tabs::new(titres)
            .select(ecran_actif)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Écrans (Ctrl+1..7) "),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(tabs, area);

        // Enregistrer les zones cliquables des onglets.
        // Le widget Tabs avec bordure affiche le texte sur la ligne intérieure
        // (area.top() + 1), pas sur toute la hauteur de la barre.
        let nb_onglets = Onglet::TOUS.len();
        let largeur_onglet = area.width as usize / nb_onglets;
        for (i, _onglet) in Onglet::TOUS.iter().enumerate() {
            let largeur = if i == nb_onglets - 1 {
                // Le dernier onglet prend le reste de la largeur pour éviter les arrondis.
                area.width as usize - i * largeur_onglet
            } else {
                largeur_onglet
            };
            let zone = Rect::new(
                area.left() + (i * largeur_onglet) as u16,
                area.top() + 1, // Ligne intérieure de la bordure du Tabs
                largeur as u16,
                1, // Une seule ligne : celle du texte des onglets
            );
            zones_cliquables.push((zone, ActionClique::ChangerOnglet(i)));
        }
    }

    fn dessine_flash(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some((message, est_erreur)) = &self.flash {
            let style = if *est_erreur {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            };
            let paragraphe = Paragraph::new(format!(" {message} "))
                .block(Block::default().borders(Borders::ALL))
                .style(style);
            f.render_widget(paragraphe, area);
        } else if let Some(echec) = &self.echec {
            let paragraphe = Paragraph::new(format!(" Erreur : {echec} "))
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Red));
            f.render_widget(paragraphe, area);
        }
    }
}

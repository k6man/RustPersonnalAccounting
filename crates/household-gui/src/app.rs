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
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
        // Ctrl+1..7 : changement d'écran direct, prioritaire sur tout
        // (marche aussi depuis un formulaire).
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
        // Ctrl+C : quitter (convention terminal).
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }

        // Délégation par écran actif. Les écrans avec champ texte (Saisie,
        // Git) consomment TOUTES les touches : les lettres q/r/l y sont du
        // texte, pas des commandes globales.
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
                Some(journal::ActionJournal::Modifier(index)) => {
                    let ligne = self
                        .donnees
                        .as_ref()
                        .and_then(|d| d.journal.lignes.iter().find(|l| l.index == index).cloned());
                    if let Some(ligne) = ligne {
                        self.saisie = crate::views::saisie::UiState::modification(&ligne);
                        self.ecran_actif = Onglet::Saisie.index();
                    }
                }
                Some(journal::ActionJournal::Supprimer(index)) => {
                    self.demander_suppression(index);
                }
                Some(journal::ActionJournal::Aucun) | None => {}
            }
            return false;
        }

        // Commandes globales (hors écrans à champ texte).
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('r') => self.demander_rechargement(),
            KeyCode::Char('l') => self.params_ledger = !self.params_ledger,
            _ => {}
        }

        false
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
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

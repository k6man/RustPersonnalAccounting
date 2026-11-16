//! Tests d'intégration de l'interface TUI (ratatui).
//!
//! Ces tests vérifient les actions retournées par les vues (navigation,
//! édition, validation) ainsi que le rendu sans affichage réel via un
//! `Buffer` ratatui.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::mpsc::channel;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use household_git::{CommitInfo, EtatFichier, FichierStatut, ResumeDiff};
use ratatui::Frame;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use rust_decimal::Decimal;

use household_budget::BudgetGrid;
use household_gui::app::{titres_onglets, zones_onglets};
use household_gui::viewmodel::{
    BudgetVsReel, CascadeAnnee, GraphesData, LigneJournal, PostingVue, RapportsData, UiData,
};
use household_gui::views::{budget, git, graphiques, journal, saisie};
use household_gui::widgets::text_input::TextInput;
use household_gui::worker::GitDonnees;

// ----------------------------------------------------------------- helpers ---

fn buffer_rect() -> Rect {
    Rect::new(0, 0, 120, 40)
}

/// Construit un `UiData` minimal pour les tests.
fn donnees_test() -> Arc<UiData> {
    let mut grilles = BTreeMap::new();
    let grille = BudgetGrid {
        year: 2026,
        rows: vec![],
        totals: vec![],
        warnings: vec![],
    };
    grilles.insert(2026, grille);

    let rapport = RapportsData {
        annee: 2026,
        depenses_categories: BTreeMap::new(),
        depenses_mois: [Decimal::ZERO; 12],
        depenses_annee_precedente: None,
        revenus_mois: [Decimal::ZERO; 12],
        epargne_mois: [Decimal::ZERO; 12],
    };

    let graphiques = GraphesData {
        annee: 2026,
        mois: 1,
        repartition_annee: vec![
            ("Logement".to_string(), Decimal::new(407751, 2)),
            ("Courses".to_string(), Decimal::new(85000, 2)),
        ],
        repartition_mois: vec![],
        cascade: CascadeAnnee {
            revenus: Decimal::new(235000, 2),
            depenses: Decimal::new(839329, 2),
            variation_patrimoine: Decimal::new(493216, 2),
            epargne: Decimal::new(90000, 2),
        },
    };

    Arc::new(UiData {
        main: std::path::PathBuf::from("examples/ledger/main.beancount"),
        charge_le: "2026-11-14 00:00:00".to_string(),
        tableau_de_bord: household_gui::viewmodel::DashboardData {
            solde_cc: Some(Decimal::new(687739, 2)),
            solde_livret_a: Some(Decimal::new(1845012, 2)),
            solde_ldd: Some(Decimal::new(987055, 2)),
            jauges: vec![],
            patrimoine_net: Decimal::new(3694687, 2),
            budget_mois: vec![BudgetVsReel {
                compte: "Expenses:Alimentation:Courses".to_string(),
                budgete: Decimal::new(90000, 2),
                reel: Decimal::new(30985, 2),
            }],
            budget_mois_label: "2026-11".to_string(),
            budget_avertissements: vec![],
            courbe: vec![("2026-01".to_string(), Decimal::new(300000, 2))],
        },
        journal: household_gui::viewmodel::JournalData {
            lignes: vec![
                LigneJournal {
                    index: 0,
                    date: "2026-03-15".parse().unwrap(),
                    flag: '*',
                    tiers: Some("Carrefour".to_string()),
                    libelle: "Courses hebdomadaires".to_string(),
                    lignes_posting: vec![PostingVue {
                        compte: "Expenses:Alimentation:Courses".to_string(),
                        montant: Some((Decimal::new(8500, 2), "EUR".to_string())),
                    }],
                    categories: "Alimentation".to_string(),
                    tags: vec!["#courses".to_string()],
                    liens: vec![],
                    fichier: "transactions/2026/03.beancount".to_string(),
                    ligne: 42,
                    montant: Some(Decimal::new(8500, 2)),
                },
                LigneJournal {
                    index: 1,
                    date: "2026-03-20".parse().unwrap(),
                    flag: '*',
                    tiers: Some("SNCF".to_string()),
                    libelle: "Billet de train".to_string(),
                    lignes_posting: vec![PostingVue {
                        compte: "Expenses:Transport:Train".to_string(),
                        montant: Some((Decimal::new(4500, 2), "EUR".to_string())),
                    }],
                    categories: "Transport".to_string(),
                    tags: vec![],
                    liens: vec![],
                    fichier: "transactions/2026/03.beancount".to_string(),
                    ligne: 57,
                    montant: Some(Decimal::new(4500, 2)),
                },
            ],
        },
        comptes: vec![],
        payees: vec![],
        grilles,
        directives_budget: vec![],
        rapports: rapport,
        graphiques,
        erreurs: vec![],
    })
}

/// Construit un `GitDonnees` minimal pour les tests.
fn donnees_git_test() -> GitDonnees {
    GitDonnees {
        disponible: true,
        fichiers: vec![FichierStatut {
            chemin: "transactions/2026/03.beancount".to_string(),
            etats: vec![EtatFichier::Modifie],
        }],
        resume: ResumeDiff {
            fichiers: 2,
            insertions: 10,
            suppressions: 3,
        },
        historique: vec![CommitInfo {
            oid: "abc1234".to_string(),
            date: "2026-03-15 10:00".to_string(),
            auteur: "Test".to_string(),
            sujet: "ledger: 2026-03 — saisie/édition".to_string(),
        }],
        message_suggere: "ledger: 2026-03 — saisie/édition (2 fichiers)".to_string(),
    }
}

/// Rend une vue dans un terminal de test et vérifie qu'il n'y a pas de panic.
fn render_safe<F: FnOnce(&mut Frame)>(area: Rect, f: F) {
    let backend = TestBackend::new(area.width, area.height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(f).unwrap();
}

/// Crée un KeyEvent avec `KeyEventKind::Press` et `KeyEventState::empty()`.
fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

// ----------------------------------------------------------------- TextInput ---

#[test]
fn text_input_insere_et_deplace_curseur() {
    let mut input = TextInput::with_value("abc");
    input.on_key(key(KeyCode::Char('X'), KeyModifiers::NONE));
    assert_eq!(input.value, "abcX");
    assert_eq!(input.cursor, 4);
}

#[test]
fn text_input_supprime_et_deplace() {
    let mut input = TextInput::with_value("abcde");
    input.cursor = 3; // entre 'c' et 'd'
    input.on_key(key(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(input.value, "abce");
    assert_eq!(input.cursor, 3);

    input.on_key(key(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(input.value, "abe");
    assert_eq!(input.cursor, 2);
}

// ----------------------------------------------------------------- Journal ---

#[test]
fn journal_deplace_selection_et_actions() {
    let donnees = donnees_test();
    let mut etat = journal::UiState::new();
    etat.set_selection(0);

    // Flèche bas déplace la sélection.
    let action = journal::on_key(
        key(KeyCode::Down, KeyModifiers::NONE),
        &mut etat,
        donnees.journal.lignes.len(),
    );
    assert!(action.is_none());
    assert_eq!(etat.selection, Some(1));

    // Entrée sur la sélection retourne Modifier.
    let action = journal::on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &mut etat,
        donnees.journal.lignes.len(),
    );
    assert!(
        matches!(action, Some(journal::ActionJournal::Modifier(1))),
        "Entrée retourne Modifier : {action:?}"
    );

    // Flèche haut revient à 0.
    let action = journal::on_key(
        key(KeyCode::Up, KeyModifiers::NONE),
        &mut etat,
        donnees.journal.lignes.len(),
    );
    assert!(action.is_none());
    assert_eq!(etat.selection, Some(0));
}

#[test]
fn journal_supprime_selection() {
    let donnees = donnees_test();
    let mut etat = journal::UiState::new();
    etat.set_selection(0);

    let action = journal::on_key(
        key(KeyCode::Delete, KeyModifiers::NONE),
        &mut etat,
        donnees.journal.lignes.len(),
    );
    assert!(
        matches!(action, Some(journal::ActionJournal::Supprimer(0))),
        "Suppr retourne Supprimer : {action:?}"
    );
}

#[test]
fn journal_render_ne_panique_pas() {
    let donnees = donnees_test();
    let mut etat = journal::UiState::new();
    etat.set_selection(0);

    render_safe(buffer_rect(), |frame| {
        journal::afficher(
            frame,
            buffer_rect(),
            &donnees.journal,
            &mut etat,
            &mut Vec::new(),
        );
    });
}

#[test]
fn journal_zones_lignes_apres_en_tete() {
    let donnees = donnees_test();
    let mut etat = journal::UiState::new();
    etat.set_selection(0);

    let area = buffer_rect();
    let mut zones = Vec::new();
    render_safe(area, |frame| {
        journal::afficher(frame, area, &donnees.journal, &mut etat, &mut zones);
    });

    // Deux transactions affichées → deux zones.
    assert_eq!(zones.len(), 2, "une zone par ligne affichée : {zones:?}");
    // La première rangée de données est 6 lignes sous le haut de l'area :
    // panneau info (3) + bordure (1) + en-tête (1) + marge d'en-tête (1).
    assert_eq!(zones[0].0.y, area.y + 6, "zone à cheval sur l'en-tête");
    assert_eq!(zones[0].0.y + 1, zones[1].0.y, "zones contiguës");
    // Les zones portent les index de transaction, pas les positions.
    assert_eq!(
        zones[0].1,
        household_gui::app::ActionClique::JournalLigne(0)
    );
    assert_eq!(
        zones[1].1,
        household_gui::app::ActionClique::JournalLigne(1)
    );
}

// ----------------------------------------------------------------- Saisie ---

#[test]
fn saisie_navigue_champs_avec_tab() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    assert_eq!(etat.champ_actif, saisie::Champ::Date);

    // Tab passe au champ suivant (Flag).
    let action = etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.champ_actif, saisie::Champ::Flag);

    // Un autre Tab passe à Tiers.
    let action = etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.champ_actif, saisie::Champ::Tiers);

    // Maj+Tab revient au précédent (Flag).
    let action = etat.on_key(
        key(KeyCode::Tab, KeyModifiers::SHIFT),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.champ_actif, saisie::Champ::Flag);
}

#[test]
fn saisie_edite_champ_libelle() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    // Se positionne sur Libellé (3 tabs depuis Date : Date -> Flag -> Tiers -> Libelle).
    for _ in 0..3 {
        etat.on_key(
            key(KeyCode::Tab, KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }
    assert_eq!(etat.champ_actif, saisie::Champ::Libelle);

    // Tape du texte dans le libellé.
    etat.on_key(
        key(KeyCode::Char('T'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert_eq!(etat.brouillon.libelle, "T");
}

#[test]
fn saisie_saisit_les_lettres_commandes_dans_un_champ_texte() {
    // Régression : e, x, q, r étaient interceptés comme commandes même
    // dans les champs texte — impossible de taper « Boulangerie ».
    // ('+' reste un raccourci global d'ajout de posting, testé à part.)
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    for _ in 0..3 {
        etat.on_key(
            key(KeyCode::Tab, KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }
    for c in "Boulangeriexqr".chars() {
        etat.on_key(
            key(KeyCode::Char(c), KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }
    assert_eq!(etat.brouillon.libelle, "Boulangeriexqr");
    assert_eq!(
        etat.champ_actif,
        saisie::Champ::Libelle,
        "pas de saut de champ"
    );
    assert_eq!(etat.brouillon.lignes.len(), 0, "pas de posting ajouté");
}

#[test]
fn saisie_ajoute_et_supprime_posting() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    // '+' ajoute une ligne de posting.
    let action = etat.on_key(
        key(KeyCode::Char('+'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.brouillon.lignes.len(), 1);
    assert_eq!(etat.posting_selectionne, 0);

    // '×' supprime la ligne.
    let action = etat.on_key(
        key(KeyCode::Char('×'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.brouillon.lignes.len(), 0);
}

#[test]
fn saisie_valide_et_annule() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    // Navigue jusqu'au champ Postings, puis Entrée valide.
    for _ in 0..6 {
        etat.on_key(
            key(KeyCode::Tab, KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }
    assert_eq!(etat.champ_actif, saisie::Champ::Postings);

    let action = etat.on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(
        matches!(action, Some(saisie::Action::Enregistrer)),
        "Entrée sur Postings retourne Enregistrer : {action:?}"
    );

    // Nouveau brouillon, puis Échap annule.
    let mut etat2 = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    let action = etat2.on_key(
        key(KeyCode::Esc, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(
        matches!(action, Some(saisie::Action::Annuler)),
        "Échap retourne Annuler : {action:?}"
    );
}

#[test]
fn saisie_render_ne_panique_pas() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    render_safe(buffer_rect(), |frame| {
        saisie::afficher(
            frame,
            buffer_rect(),
            &mut etat,
            &donnees.comptes,
            &donnees.payees,
            &mut Vec::new(),
        );
    });
}

// ----------------------------------------------------------------- Budget ---

#[test]
fn budget_affiche_sans_panique() {
    let donnees = donnees_test();
    let mut etat = budget::UiState::nouveau(2026);

    let _grille = Some(&donnees.grilles[&2026]);
    render_safe(buffer_rect(), |frame| {
        budget::afficher(
            frame,
            buffer_rect(),
            _grille,
            &mut etat,
            &|_| false,
            &mut Vec::new(),
        );
    });
}

#[test]
fn budget_navigue_lignes_et_mois() {
    let donnees = donnees_test();
    let mut etat = budget::UiState::nouveau(2026);

    let _grille = Some(&donnees.grilles[&2026]);
    assert_eq!(etat.ligne_index(Some(&donnees.grilles[&2026])), 0);

    etat.mois_suivant();
    assert_eq!(etat.mois_selectionne, 1);

    etat.mois_precedent();
    assert_eq!(etat.mois_selectionne, 0);
}

#[test]
fn budget_action_edition_cellule_retourne_none_si_grille_vide() {
    let donnees = donnees_test();
    let mut etat = budget::UiState::nouveau(2026);

    let grille = Some(&donnees.grilles[&2026]);
    let (_cmd_tx, _cmd_rx) = channel::<household_gui::worker::Command>();
    let action = budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        grille,
        &|_| false,
        &_cmd_tx,
    );
    // Avec une grille vide, on_key retourne None (pas de ligne).
    assert!(action.is_none());
}

// ----------------------------------------------------------------- Graphiques ---

#[test]
fn graphiques_bascule_onglets() {
    let _donnees = donnees_test();
    let mut etat = graphiques::UiState::nouveau();
    assert_eq!(etat.onglet, 0);

    etat.onglet_suivant();
    assert_eq!(etat.onglet, 1);

    etat.onglet_suivant();
    assert_eq!(etat.onglet, 2);

    etat.onglet_precedent();
    assert_eq!(etat.onglet, 1);
}

#[test]
fn graphiques_bascule_repartition() {
    let mut etat = graphiques::UiState::nouveau();
    assert!(!etat.repartition_mois);

    etat.bascule_repartition();
    assert!(etat.repartition_mois);

    etat.bascule_repartition();
    assert!(!etat.repartition_mois);
}

#[test]
fn graphiques_render_ne_panique_pas() {
    let _donnees = donnees_test();
    let mut etat = graphiques::UiState::nouveau();

    render_safe(buffer_rect(), |frame| {
        graphiques::afficher(frame, buffer_rect(), &_donnees, &mut etat, &mut Vec::new());
    });
}

// ----------------------------------------------------------------- Git ---

#[test]
fn git_edite_message_et_commite() {
    let mut etat = git::UiState::nouveau();

    // Tape dans le message de commit (champ Message actif) : la frappe est
    // consommée (Aucun), jamais relâchée vers les commandes globales.
    let action = git::on_key(key(KeyCode::Char('M'), KeyModifiers::NONE), &mut etat);
    assert!(
        matches!(action, Some(git::ActionGit::Aucun)),
        "frappe dans le message consommée : {action:?}"
    );
    assert_eq!(etat.message.value, "M");

    // Entrée sur le champ Commit retourne Commit.
    etat.champ_actif = git::ChampGit::Commit;
    let action = git::on_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut etat);
    assert!(
        matches!(action, Some(git::ActionGit::Commit(ref m)) if m == "M"),
        "Entrée sur Commit retourne Commit : {action:?}"
    );
}

#[test]
fn git_pre_remplit_message_suggere() {
    let mut etat = git::UiState::nouveau();
    let _git = donnees_git_test();

    // Simule le pré-remplissage fait dans app.rs.
    etat.message = TextInput::with_value("ledger: 2026-03 — saisie/édition (2 fichiers)");
    etat.message.cursor = etat.message.value.len();

    assert_eq!(
        etat.message.value,
        "ledger: 2026-03 — saisie/édition (2 fichiers)"
    );
}

#[test]
fn git_render_ne_panique_pas() {
    let git = donnees_git_test();
    let mut etat = git::UiState::nouveau();

    render_safe(buffer_rect(), |frame| {
        git::afficher(frame, buffer_rect(), &git, &mut etat, &mut Vec::new());
    });
}

// ------------------------------------------------- Cohérence clic / rendu ---

#[test]
fn onglets_zones_largeurs_reelles() {
    // Régression : les zones de clic étaient découpées en parts égales
    // alors que le rendu proportionne chaque onglet à son libellé — les
    // clics sélectionnaient le mauvais onglet.
    let titres = titres_onglets();
    assert_eq!(titres.len(), 7);

    let area = buffer_rect();
    let zones = zones_onglets(&titres, area);
    assert_eq!(
        zones.len(),
        7,
        "tous les onglets tiennent dans 120 colonnes"
    );

    // Largeurs proportionnelles aux libellés, donc inégales…
    let largeur_0 = zones[0].0.width;
    let largeur_1 = zones[1].0.width;
    assert_ne!(
        largeur_0, largeur_1,
        "« Tableau de bord » et « Journal » n'ont pas la même largeur"
    );
    assert!(largeur_0 > largeur_1);

    // …contiguës, alignées sur la ligne d'onglets, dans l'aire.
    let mut x = area.x;
    for (i, (zone, index)) in zones.iter().enumerate() {
        assert_eq!(zone.y, area.y, "onglets sur la première ligne");
        assert_eq!(zone.x, x, "zones contiguës sans chevauchement");
        assert!(zone.right() <= area.right());
        assert_eq!(*index, i, "zone i ↔ onglet i");
        x = zone.right() + 1; // + barre verticale de séparation
    }
}

#[test]
fn onglets_clic_renomme_le_bon_onglet() {
    // Le point médian de chaque zone doit résoudre vers l'onglet rendu à
    // cette position (particulièrement les onglets fins comme « 7 Git »,
    // écrasés par la découpe en parts égales).
    let titres = titres_onglets();
    let area = buffer_rect();
    let zones = zones_onglets(&titres, area);

    for (zone, index) in &zones {
        let point = ratatui::layout::Position {
            x: zone.x + zone.width / 2,
            y: zone.y,
        };
        assert!(
            zone.contains(point),
            "le centre de la zone {index} est dedans"
        );
        // L'onglet attendu à ce x est bien celui de la zone : aucun autre
        // onglet ne doit contenir le point (pas de recouvrement).
        for (autre, autre_index) in &zones {
            if autre_index != index {
                assert!(
                    !autre.contains(point),
                    "point de l'onglet {index}capturé par l'onglet {autre_index}"
                );
            }
        }
    }
}

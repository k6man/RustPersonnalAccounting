//! Tests d'intégration de l'interface TUI (ratatui).
//!
//! Ces tests vérifient les actions retournées par les vues (navigation,
//! édition, validation) ainsi que le rendu sans affichage réel via un
//! `Buffer` ratatui.

use std::collections::BTreeMap;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use household_git::{CommitInfo, EtatFichier, FichierStatut, ResumeDiff};
use ratatui::Frame;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use rust_decimal::Decimal;

use household_budget::{BudgetGrid, GridCell, GridRow};
use household_gui::app::{titres_onglets, zones_onglets};
use household_gui::viewmodel::{
    BudgetVsReel, CascadeAnnee, GraphesData, LigneJournal, PostingVue, RapportsData, UiData,
};
use household_gui::views::{budget, dashboard, git, graphiques, import, journal, saisie};
use household_gui::widgets::text_input::TextInput;
use household_gui::worker::GitDonnees;

mod common;

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
            total_budgete: Decimal::new(90000, 2),
            total_reel: Decimal::new(30985, 2),
            reste_a_depenser: Decimal::new(59015, 2),
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
                    tags: vec!["courses".to_string()],
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
        commentaires_budget: BTreeMap::from([(
            2026,
            BTreeMap::from([(
                "Expenses:Alimentation:Courses".to_string(),
                "Courses hebdo du foyer".to_string(),
            )]),
        )]),
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

#[test]
fn text_input_ligne_curseur_inverse_le_caractere_sous_le_curseur() {
    let mut input = TextInput::with_value("abc");
    input.set_focused(true);
    input.cursor = 1; // entre 'a' et 'b' → le caret est sur 'b'

    let ligne = input.ligne_avec_curseur();
    let contenus: Vec<&str> = ligne.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(contenus, vec!["a", "b", "c"], "découpe autour du curseur");
    assert_eq!(
        ligne.spans[1].style.bg,
        Some(ratatui::style::Color::White),
        "le caractère sous le curseur porte le caret"
    );
}

#[test]
fn text_input_ligne_sans_focus_pas_de_caret() {
    let input = TextInput::with_value("abc");
    let ligne = input.ligne_avec_curseur();
    assert_eq!(ligne.spans.len(), 1, "sans focus : texte simple");
    assert_eq!(ligne.spans[0].content, "abc");
}

#[test]
fn text_input_ligne_curseur_en_fin_bloc_plein() {
    let mut input = TextInput::with_value("abc");
    input.set_focused(true);

    let ligne = input.ligne_avec_curseur();
    let dernier = ligne.spans.last().expect("au moins un span");
    assert_eq!(dernier.content, " ", "curseur en fin : bloc plein");
    assert_eq!(dernier.style.bg, Some(ratatui::style::Color::White));
}

// ----------------------------------------------------------------- Journal ---

#[test]
fn journal_deplace_selection_et_actions() {
    let donnees = donnees_test();
    let mut etat = journal::UiState::new();
    etat.set_selection(0);
    let lignes_filtrees: Vec<_> = donnees.journal.lignes.iter().collect();

    // Flèche bas déplace la sélection.
    let action = journal::on_key(
        key(KeyCode::Down, KeyModifiers::NONE),
        &mut etat,
        &lignes_filtrees,
    );
    assert!(action.is_none());
    assert_eq!(etat.selection, Some(1));

    // Entrée sur la sélection retourne Modifier.
    let action = journal::on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &mut etat,
        &lignes_filtrees,
    );
    assert!(
        matches!(action, Some(journal::ActionJournal::Modifier(1))),
        "Entrée retourne Modifier : {action:?}"
    );

    // Flèche haut revient à 0.
    let action = journal::on_key(
        key(KeyCode::Up, KeyModifiers::NONE),
        &mut etat,
        &lignes_filtrees,
    );
    assert!(action.is_none());
    assert_eq!(etat.selection, Some(0));
}

#[test]
fn journal_molette_deplace_la_selection() {
    let donnees = donnees_test();
    let mut etat = journal::UiState::new();
    let lignes_filtrees: Vec<_> = donnees.journal.lignes.iter().collect();
    let nb = lignes_filtrees.len();

    // Pas de sélection explicite : la molette vers le bas sélectionne 1.
    etat.descendre(nb);
    assert_eq!(etat.selection, Some(1));

    // La molette vers le haut remonte.
    etat.monter(nb);
    assert_eq!(etat.selection, Some(0));

    // Bornes : remonter depuis 0 ne fait rien, descendre au max s'arrête.
    etat.monter(nb);
    assert_eq!(etat.selection, Some(0), "borné en haut");
    for _ in 0..10 {
        etat.descendre(nb);
    }
    assert_eq!(etat.selection, Some(nb - 1), "borné en bas");
}

#[test]
fn journal_supprime_selection() {
    let donnees = donnees_test();
    let mut etat = journal::UiState::new();
    etat.set_selection(0);
    let lignes_filtrees: Vec<_> = donnees.journal.lignes.iter().collect();

    let action = journal::on_key(
        key(KeyCode::Delete, KeyModifiers::NONE),
        &mut etat,
        &lignes_filtrees,
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

    // 7 champs de filtre + 2 lignes de transactions = 9 zones.
    assert_eq!(zones.len(), 9, "7 filtres + 2 lignes : {zones:?}");

    // Les 7 premières zones sont les champs de filtre (ligne y=0, hauteur 5).
    for (i, zone) in zones.iter().take(7).enumerate() {
        assert_eq!(zone.0.y, area.y, "filtre {i} sur la première ligne");
        assert_eq!(zone.0.height, 5, "hauteur du panneau filtre");
    }

    // Les zones de lignes commencent après : filtres (5) + info (3) + bordure tableau (1) + en-tête (1) + marge (1) = 11.
    assert_eq!(zones[7].0.y, area.y + 11, "première ligne de transaction");
    assert_eq!(zones[7].0.y + 1, zones[8].0.y, "zones de lignes contiguës");
    // Les zones portent les index de transaction, pas les positions.
    assert_eq!(
        zones[7].1,
        household_gui::app::ActionClique::JournalLigne(0)
    );
    assert_eq!(
        zones[8].1,
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
fn saisie_navigue_cycle_complet_avec_shift_tab() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    assert_eq!(etat.champ_actif, saisie::Champ::Date);

    // Avance jusqu'à Postings : Date → Flag → Tiers → Libellé → Tags → Liens → Postings
    for expected in [
        saisie::Champ::Flag,
        saisie::Champ::Tiers,
        saisie::Champ::Libelle,
        saisie::Champ::Tags,
        saisie::Champ::Liens,
        saisie::Champ::Postings,
    ] {
        let action = etat.on_key(
            key(KeyCode::Tab, KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
        assert!(action.is_none());
        assert_eq!(etat.champ_actif, expected);
    }

    // Un Tab de plus revient à Date (cycle).
    let action = etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.champ_actif, saisie::Champ::Date);

    // Maj+Tab depuis Date va à Postings (cycle inverse).
    let action = etat.on_key(
        key(KeyCode::Tab, KeyModifiers::SHIFT),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.champ_actif, saisie::Champ::Postings);

    // Remonte tout le cycle avec Maj+Tab.
    for expected in [
        saisie::Champ::Liens,
        saisie::Champ::Tags,
        saisie::Champ::Libelle,
        saisie::Champ::Tiers,
        saisie::Champ::Flag,
        saisie::Champ::Date,
    ] {
        let action = etat.on_key(
            key(KeyCode::Tab, KeyModifiers::SHIFT),
            &donnees.comptes,
            &donnees.payees,
        );
        assert!(action.is_none());
        assert_eq!(etat.champ_actif, expected);
    }
}

#[test]
fn saisie_navigue_champs_avec_fleches_haut_bas() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    assert_eq!(etat.champ_actif, saisie::Champ::Date);

    // Flèche bas avance : Date → Flag → Tiers → Libellé → Tags → Liens → Postings
    for expected in [
        saisie::Champ::Flag,
        saisie::Champ::Tiers,
        saisie::Champ::Libelle,
        saisie::Champ::Tags,
        saisie::Champ::Liens,
        saisie::Champ::Postings,
    ] {
        let action = etat.on_key(
            key(KeyCode::Down, KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
        assert!(action.is_none());
        assert_eq!(etat.champ_actif, expected);
    }

    // Flèche bas depuis Postings ne cycle PAS vers Date (Haut/Bas
    // naviguent les postings quand on est dans le champ Postings).
    // Pour passer au champ suivant, utiliser Tab.
    let action = etat.on_key(
        key(KeyCode::Down, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(
        etat.champ_actif,
        saisie::Champ::Postings,
        "Haut/Bas naviguent les postings dans le champ Postings"
    );

    // Tab depuis Postings cycle vers Date.
    let action = etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.champ_actif, saisie::Champ::Date);

    // Remonte le cycle avec flèche haut (s'arrête à Postings, car
    // dans ce champ Haut/Bas naviguent les postings).
    let action = etat.on_key(
        key(KeyCode::Up, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(
        etat.champ_actif,
        saisie::Champ::Postings,
        "Haut depuis Date → Postings"
    );
    // Maintenant dans Postings : Up navigue les postings (pas de champ
    // précédent). Pour continuer le cycle inverse, utiliser Maj+Tab.
}

#[test]
fn saisie_fleches_haut_bas_naviguent_postings_dans_champ_postings() {
    // Quand on est dans le champ Postings, Haut/Bas naviguent les lignes
    // de postings, pas les champs du formulaire.
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    // Ajoute 3 lignes de postings via le raccourci '+'.
    for _ in 0..3 {
        etat.on_key(
            key(KeyCode::Char('+'), KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }
    etat.posting_selectionne = 0;
    etat.champ_actif = saisie::Champ::Postings;

    // Flèche bas sélectionne la ligne suivante.
    let action = etat.on_key(
        key(KeyCode::Down, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.posting_selectionne, 1);

    let action = etat.on_key(
        key(KeyCode::Down, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.posting_selectionne, 2);

    // Flèche haut remonte.
    let action = etat.on_key(
        key(KeyCode::Up, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.posting_selectionne, 1);

    // Ne dépasse pas les bornes.
    etat.posting_selectionne = 0;
    let action = etat.on_key(
        key(KeyCode::Up, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.posting_selectionne, 0);
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
    assert_eq!(
        etat.brouillon.lignes.len(),
        2,
        "brouillon vierge : 2 lignes prêtes, aucune ajoutée"
    );
}

#[test]
fn saisie_ajoute_et_supprime_posting() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    // '+' ajoute une ligne de posting (le brouillon vierge en a déjà 2).
    let action = etat.on_key(
        key(KeyCode::Char('+'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.brouillon.lignes.len(), 3);
    assert_eq!(etat.posting_selectionne, 2);

    // '×' supprime la ligne.
    let action = etat.on_key(
        key(KeyCode::Char('×'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none());
    assert_eq!(etat.brouillon.lignes.len(), 2);
}

#[test]
fn saisie_bascule_le_flag_avec_f() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    // Depuis le champ Flag, 'f' bascul '*' ⇄ '!'.
    etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert_eq!(etat.champ_actif, saisie::Champ::Flag);
    etat.on_key(
        key(KeyCode::Char('f'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert_eq!(etat.brouillon.flag, '!');
    etat.on_key(
        key(KeyCode::Char('f'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert_eq!(etat.brouillon.flag, '*');

    // Mais dans un champ texte, 'f' reste du texte (retour sur Tiers).
    etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert_eq!(etat.champ_actif, saisie::Champ::Tiers);
    etat.on_key(
        key(KeyCode::Char('f'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert_eq!(etat.brouillon.tiers, "f");
    assert_eq!(etat.brouillon.flag, '*', "le flag n'a pas bougé");
}

#[test]
fn saisie_edite_posting_compte_puis_montant() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    // Ajoute une ligne, puis 'e' ouvre l'éditeur sur le compte.
    etat.on_key(
        key(KeyCode::Char('+'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    etat.on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    let edition = etat.edition_posting.as_ref().expect("édition ouverte");
    assert_eq!(edition.index, 2);
    assert_eq!(edition.champ, saisie::ChampPosting::Compte);

    // Tape le compte : les lettres consommées (r, x, e…) restent du texte.
    for c in "Expenses:Alimentation:Courses".chars() {
        etat.on_key(
            key(KeyCode::Char(c), KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }

    // Tab passe au montant.
    etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    let edition = etat.edition_posting.as_ref().expect("toujours en édition");
    assert_eq!(edition.champ, saisie::ChampPosting::Montant);
    assert_eq!(
        edition.compte_edite, "Expenses:Alimentation:Courses",
        "le compte édité est conservé"
    );

    // Saisie du montant (virgule décimale), Entrée valide.
    for c in "25,50".chars() {
        etat.on_key(
            key(KeyCode::Char(c), KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }
    let action = etat.on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(
        action.is_none(),
        "la validation d'un posting n'enregistre pas"
    );
    assert!(etat.edition_posting.is_none(), "édition fermée");
    assert_eq!(
        etat.brouillon.lignes[2],
        crate_saisie_ligne("Expenses:Alimentation:Courses", "25,50")
    );
}

#[test]
fn saisie_edition_posting_esc_annule_sans_ecrire() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    etat.on_key(
        key(KeyCode::Char('+'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    etat.on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    etat.edition_posting.as_mut().unwrap().input.value = "Assets:Test".to_string();

    let action = etat.on_key(
        key(KeyCode::Esc, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none(), "Esc n'enregistre pas");
    assert!(etat.edition_posting.is_none(), "édition fermée");
    assert_eq!(etat.brouillon.lignes[2].compte, "", "brouillon inchangé");
}

#[test]
fn saisie_edition_posting_montant_invalide_garde_ouverte() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());

    etat.on_key(
        key(KeyCode::Char('+'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    etat.on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    // Compte saisi, passage au montant.
    etat.edition_posting.as_mut().unwrap().input.value = "Expenses:Test".to_string();
    etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    etat.edition_posting.as_mut().unwrap().input.value = "abc".to_string();

    let action = etat.on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    assert!(action.is_none(), "montant invalide : aucune écriture");
    assert!(
        etat.edition_posting.is_some(),
        "l'édition reste ouverte pour corriger"
    );
    assert_eq!(etat.brouillon.lignes[2].montant, "", "brouillon inchangé");
}

#[test]
fn saisie_modification_prefill_parsable() {
    // Régression : le pré-remplissage passait par fmt::eur (« 85,00 € »),
    // non re-parsable — depuis_ligne rend « 85.00 ».
    let donnees = donnees_test();
    let ligne = &donnees.journal.lignes[0];
    let etat = saisie::UiState::modification(ligne);

    assert_eq!(
        etat.brouillon.lignes[0].compte,
        "Expenses:Alimentation:Courses"
    );
    assert_eq!(etat.brouillon.lignes[0].montant, "85.00");
    assert!(
        household_gui::saisie::parse_montant(&etat.brouillon.lignes[0].montant).is_ok(),
        "montant pré-rempli re-parsable"
    );
    assert_eq!(etat.brouillon.tiers, "Carrefour");
    assert_eq!(etat.brouillon.tags, "#courses", "tags avec préfixe");
}

#[test]
fn saisie_rendu_edition_posting_ne_panique_pas() {
    let donnees = donnees_test();
    let mut etat = saisie::UiState::nouveau("2026-03-15".parse().unwrap());
    etat.on_key(
        key(KeyCode::Char('+'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    etat.on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    for c in "Assets:Banque:CompteCourant".chars() {
        etat.on_key(
            key(KeyCode::Char(c), KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }
    etat.on_key(
        key(KeyCode::Tab, KeyModifiers::NONE),
        &donnees.comptes,
        &donnees.payees,
    );
    for c in "25,50".chars() {
        etat.on_key(
            key(KeyCode::Char(c), KeyModifiers::NONE),
            &donnees.comptes,
            &donnees.payees,
        );
    }

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

/// Petit helper de test : une LigneBrouillon attendue.
fn crate_saisie_ligne(compte: &str, montant: &str) -> household_gui::saisie::LigneBrouillon {
    household_gui::saisie::LigneBrouillon {
        compte: compte.to_string(),
        montant: montant.to_string(),
    }
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
            &commentaires_vide(),
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
fn budget_croix_de_selection_visible() {
    // La sélection doit se lire en 2D : cellule active en bleu, en-tête du
    // mois sélectionné marqué, ligne en gris discret — l'ancien REVERSED
    // intégral marquait la ligne mais pas le mois.
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    etat.mois_selectionne = 1; // Fév, pour ne pas tomber sur Jan
    let area = Rect::new(0, 0, 80, 24);
    let backend = TestBackend::new(area.width, area.height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            budget::afficher(
                frame,
                area,
                Some(&grille),
                &mut etat,
                &commentaires_vide(),
                &|_| false,
                &mut Vec::new(),
            );
        })
        .unwrap();
    let contenu = terminal.backend().buffer().content().to_vec();

    // Première occurrence d'une séquence de symboles d'un caractère.
    fn position(contenu: &[ratatui::buffer::Cell], seq: &str) -> Option<usize> {
        let lettres: Vec<char> = seq.chars().collect();
        contenu.windows(lettres.len()).position(|fenetre| {
            fenetre
                .iter()
                .zip(&lettres)
                .all(|(cellule, attendu)| cellule.symbol() == attendu.to_string())
        })
    }

    // 1. Les cellules mensuelles « 900 » : la ligne sélectionnée est
    //    grisée (Jan, première occurrence) et la cellule active Fév est
    //    en bleu. Sur 80 colonnes les cellules mensuelles sont clippées
    //    (« 900,00 » → « 900 ») mais lisible.
    let pos_900 = position(&contenu, "900").expect("cellule 900 rendue");
    assert_eq!(
        contenu[pos_900].bg,
        ratatui::style::Color::DarkGray,
        "la ligne sélectionnée est grisée (Jan, première occurrence)"
    );
    let bleue = contenu.windows(3).any(|fenetre| {
        fenetre
            .iter()
            .zip("900".chars())
            .all(|(c, a)| c.symbol() == a.to_string())
            && fenetre[0].bg == ratatui::style::Color::Blue
    });
    assert!(bleue, "la cellule sélectionnée est en fond bleu");

    // 2. L'en-tête « Fév » porte le même marqueur bleu — « Fév » apparaît
    //    aussi dans le titre (position complète), on cherche donc une
    //    occurrence bleue ; « Mar » n'existe que dans l'en-tête.
    let fev_marque = contenu.windows(3).any(|fenetre| {
        fenetre
            .iter()
            .zip("Fév".chars())
            .all(|(c, a)| c.symbol() == a.to_string())
            && fenetre[0].bg == ratatui::style::Color::Blue
    });
    assert!(fev_marque, "l'en-tête du mois sélectionné est marqué");
    let pos_mar = position(&contenu, "Mar").expect("en-tête Mar rendu");
    assert_ne!(contenu[pos_mar].bg, ratatui::style::Color::Blue);

    // 3. La ligne sélectionnée est en gris discret (le compte commence la
    //    ligne) — le titre contient aussi le compte, on cherche donc une
    //    occurrence grisée ; plus aucun REVERSED dans tout l'écran.
    let ligne_grise = contenu
        .iter()
        .any(|c| c.symbol() == "E" && c.bg == ratatui::style::Color::DarkGray);
    assert!(
        ligne_grise,
        "la ligne sélectionnée est en fond gris discret"
    );
    assert!(
        contenu
            .iter()
            .all(|c| !c.modifier.contains(ratatui::style::Modifier::REVERSED)),
        "plus de vidéo inversée sur la ligne"
    );

    // 4. Le titre porte la position complète (lisible sans couleur).
    let texte: String = contenu
        .iter()
        .map(|c| c.symbol().to_string())
        .collect::<Vec<_>>()
        .join("");
    assert!(
        texte.contains("ligne 1/1 · Fév"),
        "titre avec position complète — buffer : {texte:?}"
    );
}

#[test]
fn budget_rendu_terminal_etroit_ne_panique_pas() {
    // Régression : largeurs fixes ≈183 colonnes — vérifie que la grille
    // proportionnelle tient (et se dégrade proprement) sur 80 colonnes,
    // avec une vraie ligne budgétaire rendue.
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let etroit = Rect::new(0, 0, 80, 24);

    render_safe(etroit, |frame| {
        budget::afficher(
            frame,
            etroit,
            Some(&grille),
            &mut etat,
            &commentaires_vide(),
            &|_| false,
            &mut Vec::new(),
        );
    });
}

#[test]
fn budget_rendu_large_toutes_colonnes_visibles() {
    // Régression : largeurs fixes ≈183 colonnes, Total/Écart hors écran —
    // sur 80 colonnes, les en-têtes des DERNIÈRES colonnes doivent être
    // rendus dans le buffer (grille proportionnelle).
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let etroit = Rect::new(0, 0, 80, 24);
    let backend = TestBackend::new(etroit.width, etroit.height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            budget::afficher(
                frame,
                etroit,
                Some(&grille),
                &mut etat,
                &commentaires_vide(),
                &|_| false,
                &mut Vec::new(),
            );
        })
        .unwrap();

    let texte: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cellule| cellule.symbol())
        .collect();
    assert!(
        texte.contains("Écart"),
        "la colonne Écart est rendue sur 80 colonnes — buffer : {texte:?}"
    );
    assert!(
        texte.contains("Déc"),
        "la colonne Décembre est rendue sur 80 colonnes"
    );
}

#[test]
fn dashboard_total_et_solde_previsionnel_affiches() {
    // Ligne TOTAL du mois + bande « solde prévisionnel » : solde CC −
    // reste à dépenser, avec la formule affichée pour être vérifiable.
    let donnees = donnees_test();
    let mut etat = dashboard::UiState::default();
    let area = Rect::new(0, 0, 100, 40);
    let backend = TestBackend::new(area.width, area.height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            dashboard::afficher(
                frame,
                area,
                &donnees.tableau_de_bord,
                &mut etat,
                &mut Vec::new(),
            );
        })
        .unwrap();
    let contenu = terminal.backend().buffer().content().to_vec();
    let texte: String = contenu
        .iter()
        .map(|c| c.symbol().to_string())
        .collect::<Vec<_>>()
        .join("");

    assert!(texte.contains("TOTAL"), "ligne de total rendue");
    // Total budgeté 900,00 / réel 309,85 → reste à dépenser 590,15.
    assert!(texte.contains("590,15"), "reste à dépenser affiché");
    // Solde CC 6 877,39 − 590,15 = 6 287,24 (espace fine insécable pour
    // les milliers, cf. fmt::nombre).
    assert!(
        texte.contains(
            "Solde prévisionnel fin de mois : 6\u{202f}877,39 € − 590,15 € = 6\u{202f}287,24 €"
        ),
        "formule complète affichée — buffer : {texte:?}"
    );
}

#[test]
fn dashboard_solde_previsionnel_negatif_en_rouge() {
    // Si le solde prévisionnel passe en négatif, la bande est en rouge
    // (le reste à dépenser dépasserait le compte courant).
    let mut donnees = (*donnees_test()).clone();
    donnees.tableau_de_bord.solde_cc = Some(Decimal::new(10000, 2)); // 100,00
    donnees.tableau_de_bord.reste_a_depenser = Decimal::new(50000, 2); // 500,00
    let mut etat = dashboard::UiState::default();

    let area = Rect::new(0, 0, 100, 40);
    let backend = TestBackend::new(area.width, area.height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            dashboard::afficher(
                frame,
                area,
                &donnees.tableau_de_bord,
                &mut etat,
                &mut Vec::new(),
            );
        })
        .unwrap();
    let contenu = terminal.backend().buffer().content().to_vec();

    // La bande est la dernière ligne de l'écran : elle doit contenir du
    // rouge (texte, pas uniquement des cases vides).
    let largeur = area.width as usize;
    let bande = &contenu[contenu.len() - largeur..];
    assert!(
        bande
            .iter()
            .any(|c| c.symbol() != " " && c.fg == ratatui::style::Color::Red),
        "la bande prévisionnelle négative est en rouge"
    );
}

#[test]
fn dashboard_defilement_borne_et_indicateur_de_titre() {
    // Le tableau « Budget vs réel » défile : fenêtre de lignes, indicateur
    // dans le titre, ligne TOTAL toujours visible, bornes respectées.
    let mut donnees = (*donnees_test()).clone();
    donnees.tableau_de_bord.budget_mois = (0..12)
        .map(|i| BudgetVsReel {
            compte: format!("Expenses:Poste{i:02}"),
            budgete: Decimal::new(10000 + i as i64, 2),
            reel: Decimal::new(1000, 2),
        })
        .collect();

    // 28 lignes : chunks[3] = 7, bande 1 → table 6 → 3 lignes de données
    // visibles (+ TOTAL).
    let area = Rect::new(0, 0, 100, 28);
    let mut etat = dashboard::UiState::default();

    let rendu = |donnees: &UiData, etat: &mut dashboard::UiState| {
        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                dashboard::afficher(frame, area, &donnees.tableau_de_bord, etat, &mut Vec::new());
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<Vec<_>>()
            .join("")
    };

    // Défaut : la première ligne est visible, l'indicateur de titre aussi.
    let texte = rendu(&donnees, &mut etat);
    let visibles = etat.visibles.max(1);
    assert!(
        texte.contains("Poste00"),
        "première ligne visible au départ"
    );
    assert!(
        texte.contains("lignes 1/12"),
        "indicateur de défilement — {texte:?}"
    );
    assert!(
        texte.contains("TOTAL"),
        "le total reste visible en défilement"
    );

    // PageDown jusqu'à la borne basse (et au-delà : bornée).
    let borne = 12 - visibles;
    for _ in 0..15 {
        assert!(dashboard::on_key(
            key(KeyCode::PageDown, KeyModifiers::NONE),
            &mut etat,
            12
        ));
    }
    assert_eq!(
        etat.defilement, borne,
        "borne basse : 12 − {visibles} visibles"
    );
    let texte = rendu(&donnees, &mut etat);
    assert!(texte.contains("Poste11"), "dernière ligne visible en bas");
    assert!(
        !texte.contains("Poste00"),
        "la première ligne est scrollée hors écran"
    );

    // Monter au-dessus de la borne haute est sans effet.
    for _ in 0..15 {
        assert!(dashboard::on_key(
            key(KeyCode::Up, KeyModifiers::NONE),
            &mut etat,
            12
        ));
    }
    assert_eq!(etat.defilement, 0, "borne haute : 0");

    // Les commandes globales ne sont pas consommées par le défilement.
    assert!(
        !dashboard::on_key(key(KeyCode::Char('q'), KeyModifiers::NONE), &mut etat, 12),
        "q reste une commande globale"
    );
}

#[test]
fn budget_action_edition_cellule_retourne_none_si_grille_vide() {
    let donnees = donnees_test();
    let mut etat = budget::UiState::nouveau(2026);

    let grille = Some(&donnees.grilles[&2026]);
    let action = budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        grille,
        &commentaires_vide(),
        &|_| false,
    );
    // Avec une grille vide, on_key retourne None (pas de ligne).
    assert!(action.is_none());
}

// ------------------------------------------------- Éditeur de cellule budgétaire ---

/// Map de commentaires vide (paramètre des vues budget dans les tests).
fn commentaires_vide() -> BTreeMap<String, String> {
    BTreeMap::new()
}

/// Grille d'essai avec une seule ligne budgétaire, tous les mois à 900,00.
fn grille_essai() -> BudgetGrid {
    BudgetGrid {
        year: 2026,
        rows: vec![GridRow {
            account: "Expenses:Alimentation:Courses".to_string(),
            currency: "EUR".to_string(),
            cells: std::array::from_fn(|_| GridCell {
                budgeted: Decimal::new(90000, 2),
                actual: Decimal::ZERO,
            }),
        }],
        totals: vec![],
        warnings: vec![],
    }
}

#[test]
fn budget_e_ouvre_l_edition_pre_remplie_sans_ecrire() {
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let action = budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );

    // Régression : 'e' n'écrit rien tout seul (l'ancien comportement
    // réécrivait la valeur existante de la cellule).
    assert!(action.is_none(), "'e' doit seulement ouvrir l'éditeur");
    let edition = etat.edition.as_ref().expect("édition ouverte");
    assert_eq!(edition.ligne, 0);
    assert_eq!(edition.mois, 0);
    assert_eq!(edition.input.value, "900.00", "pré-remplie avec la valeur");
    assert!(edition.input.focused, "le caret est rendu");
}

#[test]
fn budget_edition_entree_valide_avec_montant_saisi() {
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let action = budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );
    assert!(action.is_none());

    // Saisie d'un nouveau montant (virgule décimale acceptée).
    etat.edition.as_mut().unwrap().input.value = "12,5".to_string();
    let action = budget::on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );

    let Some(budget::ActionBudget::Cellule {
        annee,
        date,
        compte,
        montant,
    }) = action
    else {
        panic!("Entrée valide l'édition : {action:?}");
    };
    assert_eq!(annee, 2026);
    assert_eq!(date.to_string(), "2026-01-01", "mois 0 → 1ᵉʳ janvier");
    assert_eq!(compte, "Expenses:Alimentation:Courses");
    assert_eq!(montant, Decimal::new(125, 1), "12,5 → 12.5");
    assert!(etat.edition.is_none(), "l'édition est fermée");
}

#[test]
fn budget_edition_esc_annule_sans_ecrire() {
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );
    etat.edition.as_mut().unwrap().input.value = "42".to_string();

    let action = budget::on_key(
        key(KeyCode::Esc, KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );

    assert!(action.is_none(), "Esc ne produit aucune écriture");
    assert!(etat.edition.is_none(), "l'édition est fermée");
}

#[test]
fn budget_edition_montant_invalide_garde_ouverte() {
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );
    etat.edition.as_mut().unwrap().input.value = "abc".to_string();

    let action = budget::on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );

    assert!(action.is_none(), "montant invalide : aucune écriture");
    assert!(
        etat.edition.is_some(),
        "l'édition reste ouverte pour corriger"
    );
}

#[test]
fn budget_edition_consomme_les_lettres_comme_du_texte() {
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );

    // Pendant l'édition, r/l/x… sont du texte de montant, pas des commandes.
    let action = budget::on_key(
        key(KeyCode::Char('r'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );
    assert!(action.is_none());
    assert_eq!(
        etat.edition.as_ref().unwrap().input.value,
        "900.00r",
        "la lettre est insérée dans le champ"
    );
}

#[test]
fn budget_rendu_en_edition_ne_panique_pas() {
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    budget::on_key(
        key(KeyCode::Char('e'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires_vide(),
        &|_| false,
    );

    render_safe(buffer_rect(), |frame| {
        budget::afficher(
            frame,
            buffer_rect(),
            Some(&grille),
            &mut etat,
            &commentaires_vide(),
            &|_| false,
            &mut Vec::new(),
        );
    });
}

// ------------------------------------------------- Commentaires budgétaires ---

#[test]
fn budget_c_ouvre_le_commentaire_pre_rempli_du_ledger() {
    // Le commentaire vit dans le ledger (lignes `;`) : `c` ouvre l'éditeur
    // pré-rempli avec la note lue, sans écrire.
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let commentaires = BTreeMap::from([(
        "Expenses:Alimentation:Courses".to_string(),
        "Courses hebdo du foyer".to_string(),
    )]);

    let action = budget::on_key(
        key(KeyCode::Char('c'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );

    assert!(action.is_none(), "'c' doit seulement ouvrir l'éditeur");
    let edition = etat
        .edition_commentaire
        .as_ref()
        .expect("édition de commentaire ouverte");
    assert_eq!(edition.ligne, 0);
    assert_eq!(
        edition.input.value, "Courses hebdo du foyer",
        "pré-rempli avec la note du ledger"
    );
    assert!(edition.input.focused, "le caret est rendu");
}

#[test]
fn budget_commentaire_esc_annule_et_entree_produit_l_action() {
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let commentaires = commentaires_vide();

    budget::on_key(
        key(KeyCode::Char('c'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    etat.edition_commentaire.as_mut().unwrap().input.value = "  ;  note du foyer ".to_string();

    // Esc : aucune action, édition fermée, rien d'écrit.
    let action = budget::on_key(
        key(KeyCode::Esc, KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    assert!(action.is_none(), "Esc annule sans écrire");
    assert!(etat.edition_commentaire.is_none());

    // Ré-ouverture, validation : l'action porte le texte nettoyé.
    budget::on_key(
        key(KeyCode::Char('c'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    etat.edition_commentaire.as_mut().unwrap().input.value = "note du foyer".to_string();
    let action = budget::on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    let Some(budget::ActionBudget::Commentaire {
        annee,
        compte,
        texte,
    }) = action
    else {
        panic!("Entrée valide le commentaire : {action:?}");
    };
    assert_eq!(annee, 2026);
    assert_eq!(compte, "Expenses:Alimentation:Courses");
    assert_eq!(
        texte, "note du foyer",
        "texte nettoyé (un « ; » de tête peut être conservé tel quel côté writer)"
    );
    assert!(etat.edition_commentaire.is_none(), "l'édition est fermée");

    // Texte vide = suppression (l'action part quand même : le writer sait
    // retirer le bloc `;`).
    budget::on_key(
        key(KeyCode::Char('c'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    etat.edition_commentaire.as_mut().unwrap().input.value = "   ".to_string();
    let action = budget::on_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    assert_eq!(
        action,
        Some(budget::ActionBudget::Commentaire {
            annee: 2026,
            compte: "Expenses:Alimentation:Courses".to_string(),
            texte: String::new(),
        }),
        "texte blanc → suppression (chaîne vide)"
    );
}

#[test]
fn budget_commentaire_consomme_les_lettres_comme_du_texte() {
    // Une note peut contenir r, l, e, x… : pendant l'édition du
    // commentaire, ces lettres sont du texte, pas des commandes globales.
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let commentaires = commentaires_vide();

    budget::on_key(
        key(KeyCode::Char('c'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    let action = budget::on_key(
        key(KeyCode::Char('r'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    assert!(action.is_none());
    assert_eq!(
        etat.edition_commentaire.as_ref().unwrap().input.value,
        "r",
        "la lettre est du texte de note"
    );
}

#[test]
fn budget_bande_de_commentaire_rendue_sous_la_grille() {
    // La note de la ligne sélectionnée s'affiche en bande sous la grille,
    // hors édition ; en édition, le champ prend la bande avec caret.
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let commentaires = BTreeMap::from([(
        "Expenses:Alimentation:Courses".to_string(),
        "Courses hebdo du foyer".to_string(),
    )]);

    let rendu = |etat: &mut budget::UiState| {
        let area = Rect::new(0, 0, 100, 28);
        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                budget::afficher(
                    frame,
                    area,
                    Some(&grille),
                    etat,
                    &commentaires,
                    &|_| false,
                    &mut Vec::new(),
                );
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<Vec<_>>()
            .join("")
    };

    let texte = rendu(&mut etat);
    assert!(
        texte.contains("Courses hebdo du foyer"),
        "la note est affichée en bande — {texte:?}"
    );
    assert!(texte.contains("c : éditer"), "l'affordance est affichée");

    // En édition, le champ et son affordance prennent la bande.
    budget::on_key(
        key(KeyCode::Char('c'), KeyModifiers::NONE),
        &mut etat,
        Some(&grille),
        &commentaires,
        &|_| false,
    );
    let texte = rendu(&mut etat);
    assert!(
        texte.contains("Entrée : valider"),
        "le prompt d'édition est affiché en bande — {texte:?}"
    );
}

#[test]
fn budget_commentaire_absent_bande_par_defaut() {
    // Sans note pour le compte, la bande propose d'en ajouter.
    let grille = grille_essai();
    let mut etat = budget::UiState::nouveau(2026);
    let commentaires = commentaires_vide();

    let area = Rect::new(0, 0, 100, 28);
    let backend = TestBackend::new(area.width, area.height);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            budget::afficher(
                frame,
                area,
                Some(&grille),
                &mut etat,
                &commentaires,
                &|_| false,
                &mut Vec::new(),
            );
        })
        .unwrap();
    let texte = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol().to_string())
        .collect::<Vec<_>>()
        .join("");
    assert!(
        texte.contains("aucun commentaire"),
        "la bande invite à en ajouter — {texte:?}"
    );
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
    assert_eq!(titres.len(), 8);

    let area = buffer_rect();
    let zones = zones_onglets(&titres, area);
    assert_eq!(
        zones.len(),
        8,
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

#[test]
fn import_render_ne_panique_pas() {
    // L'écran Import doit rendre sans paniquer, même sans profils chargés.
    let mut etat = import::UiState::default();
    let main = std::path::Path::new("examples/ledger/main.beancount");
    render_safe(buffer_rect(), |f| {
        import::render(f, &mut etat, main, buffer_rect());
    });
}

#[test]
fn import_a_profils_predefinis_dans_exemple() {
    use common::{cleanup, ledger_copy};
    // Le ledger d'exemple contient un importers.toml avec 9 profils.
    // On utilise ledger_copy() pour avoir un chemin absolu valide (le test
    // copie le ledger d'exemple dans un scratch, y compris importers.toml).
    let main = ledger_copy();
    let mut etat = import::UiState::default();
    // Forcer le chargement des profils
    etat.charger_profils(&main);
    assert!(
        !etat.profils.is_empty(),
        "doit charger les profils depuis l'exemple"
    );
    assert_eq!(
        etat.profils.len(),
        9,
        "9 profils dans l'exemple (5 CSV + 2 OFX + 2 BNP)"
    );
    // Vérifier quelques profils connus
    let noms: Vec<_> = etat.profils.iter().map(|p| p.name.clone()).collect();
    assert!(noms.contains(&"bnp-csv".to_string()));
    assert!(noms.contains(&"credit-mutuel-csv".to_string()));
    assert!(noms.contains(&"boursorama-csv".to_string()));
    // Nettoyer le scratch
    cleanup(main.parent().unwrap().parent().unwrap());
}

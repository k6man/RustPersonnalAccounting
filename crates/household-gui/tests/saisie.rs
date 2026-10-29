//! Tests du brouillon de saisie (pur) : parsing, équilibrage, erreurs,
//! pré-remplissage depuis le journal, autocomplétion.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rustledger_core::NaiveDate;

use household_gui::saisie::{
    Brouillon, LigneBrouillon, completer_comptes, completer_payees, parse_montant, parse_symboles,
};
use household_gui::viewmodel::{InfoCompte, LigneJournal};

fn brouillon_complet() -> Brouillon {
    Brouillon {
        date: "2026-04-10".to_string(),
        flag: '*',
        tiers: "Carrefour City".to_string(),
        libelle: "Courses de la semaine".to_string(),
        tags: "#courses #weekend".to_string(),
        liens: "^course-042".to_string(),
        lignes: vec![
            LigneBrouillon {
                compte: "Expenses:Alimentation:Courses".to_string(),
                montant: "25.00".to_string(),
            },
            LigneBrouillon {
                compte: "Assets:Banque:CompteCourant".to_string(),
                montant: String::new(), // équilibrage
            },
        ],
    }
}

#[test]
fn parse_montant_devises_et_virgule() {
    assert_eq!(parse_montant("38.42"), Ok((dec!(38.42), "EUR".to_string())));
    assert_eq!(parse_montant("38,42"), Ok((dec!(38.42), "EUR".to_string())));
    assert_eq!(parse_montant("10 USD"), Ok((dec!(10), "USD".to_string())));
    assert_eq!(parse_montant("10 usd"), Ok((dec!(10), "USD".to_string())));
    assert!(parse_montant("").is_err());
    assert!(parse_montant("abc").is_err());
    assert!(parse_montant("10 123").is_err(), "devise numérique refusée");
    assert!(parse_montant("-5.20").is_ok(), "montant négatif permis");
}

#[test]
fn parse_symboles_tolere_labsence_de_prefixe() {
    assert_eq!(
        parse_symboles("#courses  #weekend", '#'),
        vec!["courses", "weekend"]
    );
    assert_eq!(
        parse_symboles("courses doublon doublon", '#'),
        vec!["courses", "doublon"]
    );
    assert_eq!(parse_symboles("^facture-042", '^'), vec!["facture-042"]);
    assert!(parse_symboles("  #  ", '#').is_empty());
}

#[test]
fn brouillon_valide_se_convertit_en_transaction_equilibree() {
    let transaction = brouillon_complet().vers_transaction().expect("valide");
    assert_eq!(transaction.date.to_string(), "2026-04-10");
    assert_eq!(transaction.flag, '*');
    assert_eq!(
        transaction.payee.as_ref().map(|p| p.as_str().to_string()),
        Some("Carrefour City".to_string())
    );
    assert_eq!(transaction.narration.as_str(), "Courses de la semaine");
    assert_eq!(
        transaction
            .tags
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>(),
        vec!["courses", "weekend"]
    );
    assert_eq!(
        transaction
            .links
            .iter()
            .map(|l| l.as_str())
            .collect::<Vec<_>>(),
        vec!["course-042"]
    );
    // Deux postings, contrepartie explicite = −25.00.
    assert_eq!(transaction.postings.len(), 2);
    let contrepartie = &transaction.postings[1].value;
    match &contrepartie.units {
        Some(rustledger_core::IncompleteAmount::Complete(montant)) => {
            assert_eq!(montant.number, dec!(-25.00));
            assert_eq!(montant.currency.as_str(), "EUR");
        }
        autre => panic!("montant explicite attendu : {autre:?}"),
    }
    // Équilibrée (booking moteur ne doit rien avoir à interpoler).
    let total: Decimal = transaction
        .postings
        .iter()
        .filter_map(|p| match &p.value.units {
            Some(rustledger_core::IncompleteAmount::Complete(a)) => Some(a.number),
            _ => None,
        })
        .sum();
    assert_eq!(total, Decimal::ZERO);
}

#[test]
fn brouillon_erreurs_forme() {
    let mut brouillon = brouillon_complet();
    brouillon.date = "10/04/2026".to_string();
    brouillon.libelle = String::new();
    assert_eq!(
        brouillon.erreurs(),
        vec![
            "date invalide (attendu AAAA-MM-JJ)".to_string(),
            "libellé obligatoire".to_string(),
        ]
    );
}

#[test]
fn brouillon_refuse_deux_lignes_elidees_et_desequilibre() {
    let mut brouillon = brouillon_complet();
    brouillon.lignes.push(LigneBrouillon {
        compte: "Assets:Epargne:LivretA".to_string(),
        montant: String::new(),
    });
    assert!(
        brouillon
            .erreurs()
            .iter()
            .any(|e| e.contains("une seule ligne")),
        "{:?}",
        brouillon.erreurs()
    );

    let mut brouillon = brouillon_complet();
    brouillon.lignes[1].montant = "-20.00".to_string(); // plus d'équilibrage
    assert!(
        brouillon
            .erreurs()
            .iter()
            .any(|e| e.contains("déséquilibre")),
        "{:?}",
        brouillon.erreurs()
    );
}

#[test]
fn brouillon_refuse_devises_melangees_avec_elidee() {
    let brouillon = Brouillon {
        date: "2026-04-10".to_string(),
        flag: '*',
        libelle: "Achat en devise".to_string(),
        lignes: vec![
            LigneBrouillon {
                compte: "Expenses:Divers".to_string(),
                montant: "10 USD".to_string(),
            },
            LigneBrouillon {
                compte: "Expenses:Divers".to_string(),
                montant: "5 EUR".to_string(),
            },
            LigneBrouillon {
                compte: "Assets:Banque:CompteCourant".to_string(),
                montant: String::new(),
            },
        ],
        ..Brouillon::default()
    };
    assert!(
        brouillon
            .erreurs()
            .iter()
            .any(|e| e.contains("devises mélangées")),
        "{:?}",
        brouillon.erreurs()
    );
}

#[test]
fn brouillon_multidevise_equilibre_par_groupe_de_devise() {
    // Beancount équilibre PAR DEVISE : deux groupes indépendants (USD et
    // EUR) qui s'équilibrent chacun — valide sans annotation de prix.
    let brouillon = Brouillon {
        date: "2026-04-10".to_string(),
        flag: '*',
        libelle: "Change et frais".to_string(),
        lignes: vec![
            LigneBrouillon {
                compte: "Assets:CompteDevise".to_string(),
                montant: "90 USD".to_string(),
            },
            LigneBrouillon {
                compte: "Equity:Change".to_string(),
                montant: "-90 USD".to_string(),
            },
            LigneBrouillon {
                compte: "Assets:Banque:CompteCourant".to_string(),
                montant: "-10 EUR".to_string(),
            },
            LigneBrouillon {
                compte: "Expenses:Frais-Bancaires".to_string(),
                montant: "10 EUR".to_string(),
            },
        ],
        ..Brouillon::default()
    };
    assert!(brouillon.erreurs().is_empty(), "{:?}", brouillon.erreurs());

    // Un groupe déséquilibré suffit à refuser (EUR non soldé).
    let brouillon = Brouillon {
        date: "2026-04-10".to_string(),
        flag: '*',
        libelle: "Change".to_string(),
        lignes: vec![
            LigneBrouillon {
                compte: "Assets:Banque:CompteCourant".to_string(),
                montant: "-100 EUR".to_string(),
            },
            LigneBrouillon {
                compte: "Assets:CompteDevise".to_string(),
                montant: "90 USD".to_string(),
            },
            LigneBrouillon {
                compte: "Equity:Change".to_string(),
                montant: "-90 USD".to_string(),
            },
        ],
        ..Brouillon::default()
    };
    assert!(
        brouillon.erreurs().iter().any(|e| e.contains("-100 EUR")),
        "{:?}",
        brouillon.erreurs()
    );
}

#[test]
fn prefill_depuis_le_journal_round_trip() {
    let ligne = LigneJournal {
        index: 42,
        date: "2026-03-15".parse().unwrap(),
        flag: '!',
        tiers: Some("Boulangerie".to_string()),
        libelle: "Pain".to_string(),
        lignes_posting: vec![
            household_gui::viewmodel::PostingVue {
                compte: "Expenses:Alimentation:Restaurants".to_string(),
                montant: Some((dec!(3.20), "EUR".to_string())),
            },
            household_gui::viewmodel::PostingVue {
                compte: "Assets:Banque:CompteCourant".to_string(),
                montant: None, // élidée dans le ledger
            },
        ],
        categories: "Expenses:Alimentation:Restaurants".to_string(),
        tags: vec!["courses".to_string()],
        liens: vec!["pain-001".to_string()],
        fichier: "x.beancount".to_string(),
        ligne: 7,
        montant: Some(dec!(3.20)),
    };
    let brouillon = household_gui::saisie::depuis_ligne(&ligne);
    assert_eq!(brouillon.date, "2026-03-15");
    assert_eq!(brouillon.flag, '!');
    assert_eq!(brouillon.tiers, "Boulangerie");
    assert_eq!(brouillon.tags, "#courses");
    assert_eq!(brouillon.liens, "^pain-001");
    assert_eq!(
        brouillon.lignes[1].montant, "",
        "posting élidé → ligne d'équilibrage"
    );

    // Round-trip : la conversion retrouve les deux postings, l'élidée remplie.
    let transaction = brouillon.vers_transaction().expect("valide");
    assert_eq!(transaction.flag, '!');
    let total: Decimal = transaction
        .postings
        .iter()
        .filter_map(|p| match &p.value.units {
            Some(rustledger_core::IncompleteAmount::Complete(a)) => Some(a.number),
            _ => None,
        })
        .sum();
    assert_eq!(total, Decimal::ZERO, "contrepartie = −3.20");
}

fn plan() -> Vec<InfoCompte> {
    vec![
        InfoCompte {
            nom: "Assets:Banque:CompteCourant".to_string(),
            ouverture: "2025-01-01".parse().unwrap(),
            fermeture: None,
        },
        InfoCompte {
            nom: "Assets:Epargne:LivretA".to_string(),
            ouverture: "2025-01-01".parse().unwrap(),
            fermeture: None,
        },
        InfoCompte {
            nom: "Expenses:Alimentation:Courses".to_string(),
            ouverture: "2026-01-01".parse().unwrap(),
            fermeture: None,
        },
        // Fermé fin février : invisible pour une saisie en mars.
        InfoCompte {
            nom: "Expenses:Ancien".to_string(),
            ouverture: "2026-01-01".parse().unwrap(),
            fermeture: Some("2026-03-01".parse().unwrap()),
        },
    ]
}

#[test]
fn completion_comptes_filtre_par_ouverture_et_fermeture() {
    let date: NaiveDate = "2026-03-20".parse().unwrap();
    let tout = completer_comptes(&plan(), "", date, 50);
    assert_eq!(tout.len(), 3, "compte fermé exclu");
    assert!(!tout.iter().any(|c| c == "Expenses:Ancien"));

    // Une saisie avant l'ouverture de Courses : invisible.
    let avant: NaiveDate = "2025-12-01".parse().unwrap();
    let tout_avant = completer_comptes(&plan(), "", avant, 50);
    assert_eq!(tout_avant.len(), 2, "Courses pas encore ouverte");

    // Filtre par sous-chaîne, insensible à la casse, borné.
    let filtres = completer_comptes(&plan(), "alim", date, 50);
    assert_eq!(filtres, vec!["Expenses:Alimentation:Courses"]);
    let bornes = completer_comptes(&plan(), "", date, 2);
    assert_eq!(bornes.len(), 2);
}

#[test]
fn completion_payees_par_frequence() {
    let payees = vec![
        ("Carrefour".to_string(), 12),
        ("Boulangerie".to_string(), 8),
        ("Amazon".to_string(), 20),
    ];
    assert_eq!(
        completer_payees(&payees, "", 50),
        vec!["Amazon", "Carrefour", "Boulangerie"]
    );
    assert_eq!(completer_payees(&payees, "ca", 50), vec!["Carrefour"]);
    assert!(completer_payees(&payees, "zzz", 50).is_empty());
}

#[test]
fn empreinte_identifie_la_ligne() {
    // L'empreinte sert à relocaliser une transaction après rechargement ;
    // sa construction depuis une ligne de journal est couverte ici.
    let ligne = LigneJournal {
        index: 3,
        date: "2026-02-14".parse().unwrap(),
        flag: '*',
        tiers: None,
        libelle: "Retrait".to_string(),
        lignes_posting: vec![],
        categories: String::new(),
        tags: vec![],
        liens: vec![],
        fichier: "f.beancount".to_string(),
        ligne: 1,
        montant: None,
    };
    let empreinte = household_gui::saisie::Empreinte::de_ligne(&ligne);
    assert_eq!(empreinte.date.to_string(), "2026-02-14");
    assert_eq!(empreinte.tiers, None);
    assert_eq!(empreinte.libelle, "Retrait");
}

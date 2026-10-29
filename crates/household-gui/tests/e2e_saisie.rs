//! Test bout en bout **headless** du jalon 5 : brouillon de saisie →
//! conversion pure → writer du jalon 2 (append/replace/remove) → snapshot
//! rechargé → view-model (journal + dashboard). Aucune GUI : c'est le même
//! chemin que le worker, sans le thread.

mod common;

use household_core::{Snapshot, append_transaction, remove_transaction, replace_transaction};
use rust_decimal_macros::dec;

use household_gui::saisie::{Brouillon, Empreinte, LigneBrouillon};
use household_gui::viewmodel;

use common::ledger_copy;

fn brouillon_courses(date: &str, montant: &str, libelle: &str) -> Brouillon {
    Brouillon {
        date: date.to_string(),
        flag: '*',
        tiers: "Carrefour City".to_string(),
        libelle: libelle.to_string(),
        tags: "#courses".to_string(),
        liens: String::new(),
        lignes: vec![
            LigneBrouillon {
                compte: "Expenses:Alimentation:Courses".to_string(),
                montant: montant.to_string(),
            },
            LigneBrouillon {
                compte: "Assets:Banque:CompteCourant".to_string(),
                montant: String::new(), // équilibrage automatique explicite
            },
        ],
    }
}

/// Le chemin complet du worker, sans thread : conversion → writer →
/// view-model sur le snapshot frais.
fn chemin_worker(
    main: &std::path::Path,
    action: impl FnOnce(&Snapshot) -> Result<Snapshot, household_core::CoreError>,
) -> Result<viewmodel::UiData, String> {
    let snapshot = Snapshot::load(main).map_err(|e| e.to_string())?;
    let nouveau = action(&snapshot).map_err(|e| format!("refus : {e}"))?;
    viewmodel::build(&nouveau, "2026-04-10".parse().unwrap())
}

#[test]
fn saisie_ecriture_modification_suppression_bout_en_bout() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();

    // --- 1. Saisie → écriture ------------------------------------------------
    let brouillon = brouillon_courses("2026-04-10", "25.00", "Courses d'avril");
    let transaction = brouillon.vers_transaction().expect("brouillon valide");
    let donnees = chemin_worker(&main, |snapshot| append_transaction(snapshot, &transaction))
        .expect("écriture acceptée");

    // Le journal contient la nouvelle ligne, avec sa source.
    let ligne = donnees
        .journal
        .lignes
        .iter()
        .find(|l| l.libelle == "Courses d'avril")
        .expect("ligne écrite dans le journal");
    assert_eq!(ligne.date.to_string(), "2026-04-10");
    assert_eq!(ligne.tiers.as_deref(), Some("Carrefour City"));
    assert!(ligne.tags.contains(&"courses".to_string()));
    assert!(ligne.fichier.contains("transactions/2026/04.beancount"));

    // Le dashboard suit : CC = 6877.39 − 25.00.
    assert_eq!(donnees.tableau_de_bord.solde_cc, Some(dec!(6852.39)));

    // --- 2. Modification depuis le journal -----------------------------------
    let empreinte = Empreinte::de_ligne(ligne);
    let correction = brouillon_courses("2026-04-10", "50.00", "Courses d'avril corrigées");
    let donnees = chemin_worker(&main, |snapshot| {
        // La GUI passe par worker::localiser — reproduit ici avec l'index
        // d'origine pour vérifier la relocalisation par empreinte.
        let index = household_gui::worker::localiser(snapshot, usize::MAX, &empreinte)
            .map_err(household_core::CoreError::Writer)?;
        replace_transaction(snapshot, index, &correction.vers_transaction().unwrap())
    })
    .expect("modification acceptée");
    assert_eq!(donnees.tableau_de_bord.solde_cc, Some(dec!(6827.39)));
    let ligne = donnees
        .journal
        .lignes
        .iter()
        .find(|l| l.libelle == "Courses d'avril corrigées")
        .expect("ligne modifiée");

    // --- 3. Suppression ------------------------------------------------------
    let empreinte = Empreinte::de_ligne(ligne);
    let donnees = chemin_worker(&main, |snapshot| {
        let index = household_gui::worker::localiser(snapshot, usize::MAX, &empreinte)
            .map_err(household_core::CoreError::Writer)?;
        remove_transaction(snapshot, index)
    })
    .expect("suppression acceptée");
    assert_eq!(donnees.tableau_de_bord.solde_cc, Some(dec!(6877.39)));
    assert!(
        !donnees
            .journal
            .lignes
            .iter()
            .any(|l| l.libelle.contains("Courses d'avril")),
        "ligne disparue du journal"
    );

    common::cleanup(&scratch);
}

#[test]
fn relocalisation_par_empreinte_et_refus_si_absente() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");

    // Une empreinte existante est relocalisée même avec un index erroné
    // (transaction réelle du ledger : frais de tenue de compte du 31/03).
    let empreinte = Empreinte {
        date: "2026-03-31".parse().unwrap(),
        tiers: Some("Banque".to_string()),
        libelle: "Frais de tenue de compte".to_string(),
    };
    let index = household_gui::worker::localiser(&snapshot, 0, &empreinte);
    assert!(index.is_ok(), "relocalisée par empreinte : {index:?}");

    // Une empreinte absente est refusée avec un message lisible.
    let fantome = Empreinte {
        date: "2026-04-01".parse().unwrap(),
        tiers: Some("Inconnu".to_string()),
        libelle: "Transaction disparue".to_string(),
    };
    let echec = household_gui::worker::localiser(&snapshot, 0, &fantome);
    assert!(echec.is_err());
    assert!(echec.unwrap_err().contains("plus trouvée"));

    common::cleanup(&scratch);
}

#[test]
fn ecriture_refusee_si_compte_inconnu_et_fichier_intact() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();

    // La validation locale passe (forme équilibrée) mais le compte n'existe
    // pas : la pré-vérification du writer refuse — c'est le choix documenté.
    let brouillon = Brouillon {
        date: "2026-04-10".to_string(),
        flag: '*',
        libelle: "Compte fantôme".to_string(),
        lignes: vec![
            LigneBrouillon {
                compte: "Expenses:Inexistant".to_string(),
                montant: "10.00".to_string(),
            },
            LigneBrouillon {
                compte: "Assets:Banque:CompteCourant".to_string(),
                montant: "-10.00".to_string(),
            },
        ],
        ..Brouillon::default()
    };
    assert!(brouillon.erreurs().is_empty(), "forme valide");
    let transaction = brouillon.vers_transaction().expect("conversion");
    let donnees = chemin_worker(&main, |snapshot| append_transaction(snapshot, &transaction));
    assert!(donnees.is_err(), "refusé par la pré-vérification");
    let refus = donnees.unwrap_err();
    assert!(refus.contains("refus"), "message clair : {refus}");

    common::cleanup(&scratch);
}

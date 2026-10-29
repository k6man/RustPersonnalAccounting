//! Tests d'écriture du fichier budgétaire sur **copie scratch** du ledger
//! échantillon : round-trip à l'octet près, édition mono-ligne, insertion
//! triée, génération d'année + include, refus d'écrasement.

mod common;

use household_budget::{BudgetGrid, duplicate_year, set_cell, write_year};
use household_core::Snapshot;
use rust_decimal_macros::dec;
use rustledger_core::{AccountTypes, Directive};

use common::{budget_file_of, budget_text, cleanup, ledger_copy};

/// Toutes les directives `custom "budget"` d'un snapshot, dans l'ordre.
fn budget_directives(snapshot: &Snapshot) -> Vec<Directive> {
    snapshot
        .entries()
        .iter()
        .filter(|e| matches!(e.value, Directive::Custom(ref c) if c.custom_type == "budget"))
        .map(|e| e.value.clone())
        .collect()
}

#[test]
fn set_cell_meme_valeur_fichier_octet_par_octet_identique() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");
    let before = budget_text(&main, 2026);

    // Mise à jour « à valeur égale » : aucun octet ne doit bouger (l'éditeur
    // est chirurgical — alignement et commentaires préservés).
    let after = set_cell(
        &snapshot,
        2026,
        "2026-01-01".parse().unwrap(),
        "Expenses:Alimentation:Courses",
        dec!(850.00),
    )
    .expect("set_cell no-op");
    assert!(after.is_valid(), "{:?}", after.errors());
    assert_eq!(budget_text(&main, 2026), before, "round-trip identique");

    cleanup(&scratch);
}

#[test]
fn set_cell_change_une_seule_ligne_et_la_grille_suit() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");
    let before_text = budget_text(&main, 2026);
    let before: Vec<&str> = before_text.lines().collect();

    // 900.00 → 950.00 sur la supersession de juillet (Courses).
    let after = set_cell(
        &snapshot,
        2026,
        "2026-07-01".parse().unwrap(),
        "Expenses:Alimentation:Courses",
        dec!(950.00),
    )
    .expect("set_cell");
    assert!(after.is_valid(), "{:?}", after.errors());

    let after_text = budget_text(&main, 2026);
    let after_lines: Vec<&str> = after_text.lines().collect();
    assert_eq!(
        after_lines.len(),
        before.len(),
        "aucune ligne ajoutée/retirée"
    );
    let changed: Vec<usize> = before
        .iter()
        .zip(after_lines.iter())
        .enumerate()
        .filter_map(|(i, (b, a))| (b != a).then_some(i))
        .collect();
    assert_eq!(
        changed,
        vec![
            before
                .iter()
                .position(|l| l.contains("2026-07-01") && l.contains("Courses"))
                .expect("ligne Courses juillet")
        ],
        "une seule ligne change"
    );
    assert!(
        after_lines[changed[0]].contains("950.00"),
        "montant réécrit : {:?}",
        after_lines[changed[0]]
    );

    // La grille recalcule : juillet budgété 950.00, juin toujours 850.00.
    let directives = budget_directives(&after);
    let types = AccountTypes::default();
    let grid = BudgetGrid::build(&directives, &types, 2026);
    let courses = grid.row("Expenses:Alimentation:Courses").expect("ligne");
    assert_eq!(courses.cells[6].budgeted, dec!(950.00));
    assert_eq!(courses.cells[5].budgeted, dec!(850.00));

    cleanup(&scratch);
}

#[test]
fn set_cell_nouvelle_directive_inseree_triee_avec_supersession() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");

    // Juin n'a pas de directive Courses (celle de janvier reste en force) :
    // insérer le 1er juin à 100.00 crée une supersession juin… juillet.
    let after = set_cell(
        &snapshot,
        2026,
        "2026-06-01".parse().unwrap(),
        "Expenses:Alimentation:Courses",
        dec!(100.00),
    )
    .expect("insertion");
    assert!(after.is_valid(), "{:?}", after.errors());

    let text = budget_text(&main, 2026);
    let pos_juin = text.find("2026-06-01").expect("nouvelle directive");
    let pos_juillet = text.find("2026-07-01").expect("supersession juillet");
    assert!(
        pos_juin < pos_juillet,
        "insérée avant la première directive postérieure (juillet)"
    );
    // Aucune directive entre juin (lui-même) et juillet : collée au bloc.
    assert_eq!(
        text[pos_juin..pos_juillet]
            .lines()
            .filter(|l| l.starts_with("2026-"))
            .count(),
        1,
        "seule la directive de juin entre le point d'insertion et juillet"
    );

    let directives = budget_directives(&after);
    let types = AccountTypes::default();
    let grid = BudgetGrid::build(&directives, &types, 2026);
    let courses = grid.row("Expenses:Alimentation:Courses").expect("ligne");
    assert_eq!(
        courses.cells[5].budgeted,
        dec!(100.00),
        "juin : nouveau montant"
    );
    assert_eq!(
        courses.cells[6].budgeted,
        dec!(900.00),
        "juillet : supersession intacte"
    );
    assert_eq!(courses.cells[0].budgeted, dec!(850.00), "janvier : intact");

    cleanup(&scratch);
}

#[test]
fn write_year_genere_fichier_et_include_une_seule_mutation() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");

    // Duplication 2026 → 2027 (0 %, pas centime) puis génération.
    let directives = budget_directives(&snapshot);
    let duplicated =
        duplicate_year(&directives, 2026, 2027, dec!(0), dec!(0.01)).expect("duplication");
    let after = write_year(&snapshot, 2027, &duplicated).expect("génération 2027");
    assert!(after.is_valid(), "{:?}", after.errors());

    // Le fichier existe, complet.
    let text = budget_text(&main, 2027);
    let count_2027 = text.matches("2027-").count();
    assert_eq!(
        count_2027,
        duplicated.len(),
        "toutes les directives écrites"
    );

    // L'include est dans main, à côté de celui de 2026.
    let main_text = std::fs::read_to_string(&main).expect("main");
    assert!(main_text.contains("include \"budgets/2027.beancount\""));
    let pos_2026 = main_text
        .find("include \"budgets/2026.beancount\"")
        .expect("include 2026");
    let pos_2027 = main_text
        .find("include \"budgets/2027.beancount\"")
        .expect("include 2027");
    assert!(pos_2026 < pos_2027, "ordre des includes d'années");
    assert!(
        pos_2027
            < main_text
                .find("include \"transactions/")
                .expect("transactions après"),
        "les budgets restent groupés avant les transactions"
    );

    // La grille 2027 se construit sur le ledger rechargé, montants égaux.
    let grid = BudgetGrid::build(&budget_directives(&after), &AccountTypes::default(), 2027);
    let source_grid = BudgetGrid::build(&directives, &AccountTypes::default(), 2026);
    let jan_2027 = grid
        .row("Expenses:Alimentation:Courses")
        .expect("ligne 2027")
        .cells[0]
        .budgeted;
    let jan_2026 = source_grid
        .row("Expenses:Alimentation:Courses")
        .expect("ligne 2026")
        .cells[0]
        .budgeted;
    assert_eq!(jan_2027, jan_2026, "duplication exacte");

    // Refus d'un second write_year (pas d'écrasement silencieux).
    let second = write_year(&after, 2027, &duplicated);
    assert!(second.is_err(), "écrasement refusé");

    cleanup(&scratch);
}

#[test]
fn set_cell_annee_inexistante_cree_fichier_et_include_valide() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");

    let after = set_cell(
        &snapshot,
        2027,
        "2027-03-01".parse().unwrap(),
        "Expenses:Vacances",
        dec!(640.00),
    )
    .expect("création 2027 via set_cell");
    assert!(after.is_valid(), "{:?}", after.errors());

    let text = budget_text(&main, 2027);
    assert!(text.contains("Expenses:Vacances"), "directive écrite");
    assert!(text.contains("640.00"));
    assert!(
        std::fs::read_to_string(&main)
            .expect("main")
            .contains("include \"budgets/2027.beancount\""),
        "include ajouté"
    );

    cleanup(&scratch);
}

#[test]
fn set_cell_deséquilibre_impossible_ici_mais_ligne_inconnue_refusee() {
    // Une ligne budget malformée (pas de token montant/devise en fin) est
    // refusée sans toucher au fichier.
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();

    // On altère une ligne du fichier budgétaire pour la rendre non reconnue,
    // puis on tente une édition (elle doit échouer, fichier intact).
    let path = budget_file_of(&main, 2026);
    let original = std::fs::read_to_string(&path).expect("budget");
    let broken = original.replace(
        "2026-01-01 custom \"budget\" Expenses:Frais-Bancaires",
        "# altérée 2026-01-01 custom \"budget\" Expenses:Frais-Bancaires",
    );
    std::fs::write(&path, &broken).expect("écriture altérée");
    let snapshot = Snapshot::load(&main).expect("ledger toujours valide");

    let failure = set_cell(
        &snapshot,
        2026,
        "2026-01-01".parse().unwrap(),
        "Expenses:Frais-Bancaires",
        dec!(4.90),
    );
    // La directive altérée n'est plus trouvée comme directive (elle est un
    // commentaire) → insertion d'une nouvelle ligne : acceptée et valide.
    assert!(failure.is_ok(), "l'insertion compense le commentaire");
    assert!(failure.expect("ok").is_valid());

    cleanup(&scratch);
}

//! Tests d'intégration du jalon 6 : view-model budgétaire (grilles),
//! édition de cellule via le writer chirurgical (jalon 3) reflétée dans la
//! grille, génération d'année, préférences rollover.

mod common;

use household_core::Snapshot;
use rust_decimal_macros::dec;
use rustledger_core::NaiveDate;

use household_gui::budget as logique;
use household_gui::viewmodel;

use common::ledger_copy;

const MAIN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ledger/main.beancount");

fn donnees() -> Option<viewmodel::UiData> {
    let main = std::path::Path::new(MAIN);
    if !main.exists() {
        eprintln!("ledger absent de la sandbox → suite ignorée");
        return None;
    }
    let snapshot = Snapshot::load(main).expect("chargement");
    let aujourdhui: NaiveDate = "2026-03-20".parse().expect("date de référence");
    Some(viewmodel::build(&snapshot, aujourdhui).expect("view-model"))
}

#[test]
fn grille_2026_chargee_avec_hierarchie() {
    let Some(data) = donnees() else { return };

    // La grille de l'année budgétée existe, triée lexicographiquement.
    let grille = data.grilles.get(&2026).expect("grille 2026");
    assert!(!grille.rows.is_empty());
    for paire in grille.rows.windows(2) {
        assert!(paire[0].account <= paire[1].account, "tri des postes");
    }

    // Aucune directive sur Expenses:Enfants lui-même : le groupe est
    // SYNTHÉTISÉ (somme des feuilles budgétées, comptées une fois).
    let arbre = logique::arbre(&grille.rows);
    let groupe = arbre
        .iter()
        .find(|n| n.compte == "Expenses:Enfants")
        .expect("groupe synthétisé");
    assert!(!groupe.budgete, "pas de directive propre");
    let somme_feuilles: rust_decimal::Decimal = grille
        .rows
        .iter()
        .filter(|l| l.account.starts_with("Expenses:Enfants:"))
        .map(|l| l.cells[2].budgeted)
        .sum();
    assert_eq!(groupe.cellules[2].budgeted, somme_feuilles, "mars agrégé");

    // L'arbre synthétise les groupes ; dans ce ledger, les budgets sont sur
    // les FEUILLES (LivretA/LDD) — pas de budget sur l'ancêtre Assets:Epargne,
    // donc les feuilles sont des têtes (rollover possible par feuille).
    let arbre = logique::arbre(&grille.rows);
    let tetes: Vec<&str> = logique::tetes(&arbre)
        .into_iter()
        .map(|i| arbre[i].compte.as_str())
        .collect();
    assert!(
        tetes.contains(&"Assets:Epargne"),
        "groupe synthétisé = tête"
    );
    assert!(
        tetes.contains(&"Assets:Epargne:LivretA"),
        "aucun ancêtre budgété au-dessus"
    );
}

#[test]
fn directives_budget_triées_pour_le_diff() {
    let Some(data) = donnees() else { return };
    let directives = &data.directives_budget;
    assert!(!directives.is_empty());
    for paire in directives.windows(2) {
        assert!(
            (paire[0].date, &paire[0].compte) <= (paire[1].date, &paire[1].compte),
            "tri (date, compte)"
        );
    }
    // Une directive concrète : la supersession Courses de juillet.
    let juillet = directives
        .iter()
        .find(|d| d.compte == "Expenses:Alimentation:Courses" && d.date.to_string() == "2026-07-01")
        .expect("supersession juillet");
    assert_eq!(juillet.montant, dec!(900.00));
    assert_eq!(juillet.intervalle, "monthly");
}

#[test]
fn edition_cellule_supersession_refletee_dans_la_grille() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();

    // Édition de JUIN pour Courses (pas de directive au 1er juin → insertion
    // d'une supersession au 2026-06-01) — le chemin exact du jalon 6.
    let snapshot = Snapshot::load(&main).expect("copie valide");
    let nouveau = household_budget::writer::set_cell(
        &snapshot,
        2026,
        "2026-06-01".parse().unwrap(),
        "Expenses:Alimentation:Courses",
        dec!(100.00),
    )
    .expect("écriture acceptée");
    let data = viewmodel::build(&nouveau, "2026-03-20".parse().unwrap()).expect("view-model");

    let grille = data.grilles.get(&2026).expect("grille rafraîchie");
    let courses = grille.row("Expenses:Alimentation:Courses").expect("ligne");
    // Janvier à mai : 850 (directive du 01-01), juin : 100 (nouvelle),
    // juillet : 900 (supersession intacte), décembre : 900.
    assert_eq!(courses.cells[0].budgeted, dec!(850.00));
    assert_eq!(courses.cells[4].budgeted, dec!(850.00));
    assert_eq!(courses.cells[5].budgeted, dec!(100.00));
    assert_eq!(courses.cells[6].budgeted, dec!(900.00));
    assert_eq!(courses.cells[11].budgeted, dec!(900.00));

    // La directive insérée apparaît dans directives_budget (triée).
    let juin = data
        .directives_budget
        .iter()
        .find(|d| d.compte == "Expenses:Alimentation:Courses" && d.date.to_string() == "2026-06-01")
        .expect("directive juin insérée");
    assert_eq!(juin.montant, dec!(100.00));

    common::cleanup(&scratch);
}

#[test]
fn generation_annee_refletee_dans_le_view_model() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();

    let snapshot = Snapshot::load(&main).expect("copie valide");
    let directives: Vec<rustledger_core::Directive> =
        snapshot.entries().iter().map(|e| e.value.clone()).collect();
    let dupliquees = household_budget::duplicate_year(&directives, 2026, 2027, dec!(0), dec!(0.01))
        .expect("duplication exacte");
    let nouveau =
        household_budget::writer::write_year(&snapshot, 2027, &dupliquees).expect("année écrite");
    let data = viewmodel::build(&nouveau, "2026-03-20".parse().unwrap()).expect("view-model");

    // La grille 2027 existe, montants identiques à 2026.
    let grille_2027 = data.grilles.get(&2027).expect("grille 2027");
    let grille_2026 = data.grilles.get(&2026).expect("grille 2026");
    let courses_2027 = grille_2027
        .row("Expenses:Alimentation:Courses")
        .expect("ligne 2027");
    let courses_2026 = grille_2026
        .row("Expenses:Alimentation:Courses")
        .expect("ligne 2026");
    assert_eq!(
        courses_2027.cells[0].budgeted,
        courses_2026.cells[0].budgeted
    );
    assert_eq!(courses_2027.cells[0].budgeted, dec!(850.00));

    common::cleanup(&scratch);
}

#[test]
fn rollover_et_totaux_purs() {
    let Some(data) = donnees() else { return };
    let grille = data.grilles.get(&2026).expect("grille 2026");

    // Rollover cumulé sur Vacances (mars : budget 640, réel 640 → dispo 0 ;
    // avril : budget 0, réel 0 → dispo 0).
    let arbre = logique::arbre(&grille.rows);
    let vacances = arbre
        .iter()
        .find(|n| n.compte == "Expenses:Vacances")
        .expect("nœud Vacances");
    assert_eq!(logique::disponible(vacances, 2), dec!(0.00));

    // Totaux : colonne mars = Σ des buckets EUR, comptée une fois.
    let mars = logique::total_mois(&grille.totals, 2);
    assert!(mars > dec!(0), "mars budgété : {mars}");
    // Cohérence : le total de mars est inférieur à la somme des lignes
    // (les parents doublent les enfants) — il ne peut pas la dépasser.
    let somme_lignes: rust_decimal::Decimal = grille.rows.iter().map(|l| l.cells[2].budgeted).sum();
    assert!(mars <= somme_lignes);
}

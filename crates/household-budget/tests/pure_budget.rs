//! Tests unitaires du cœur budgétaire pur : grille (accrual Fava), rollover,
//! duplication. Aucun I/O — mini-ledgers en mémoire.

mod common;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use household_budget::{BudgetGrid, GridCell, RolloverSeries, duplicate_year, grid};
use rustledger_core::{AccountTypes, Directive};

use common::{budget, open, spend};

/// Mini-ledger : opens + directives passées.
fn base() -> Vec<Directive> {
    vec![
        open("2026-01-01", "Expenses:Test"),
        open("2026-01-01", "Assets:Banque:CompteCourant"),
    ]
}

// --------------------------------------------------------------- grille ------

#[test]
fn fevrier_bissextile_denominateur_calendaire_exact() {
    // Budget journalier 1.00 EUR : février 2027 (28 jours) = 28.00,
    // février 2028 (bissextile, 29 jours) = 29.00 — le dénominateur est la
    // durée calendaire, jamais 30 ni 365/12.
    let mut directives = base();
    directives.push(budget("2027-01-01", "Expenses:Test", "daily", dec!(1.00)));

    let types = AccountTypes::default();
    let grid_2027 = BudgetGrid::build(&directives, &types, 2027);
    let row = grid_2027.row("Expenses:Test").expect("ligne budget");
    assert_eq!(row.cells[1].budgeted, dec!(28.00), "février 2027");

    let grid_2028 = BudgetGrid::build(&directives, &types, 2028);
    let row = grid_2028.row("Expenses:Test").expect("ligne budget");
    assert_eq!(
        row.cells[1].budgeted,
        dec!(29.00),
        "février 2028 (bissextile)"
    );
}

#[test]
fn supersession_en_cours_de_mois_split_pro_rata() {
    // 90.00 EUR/mois à partir du 1er avril, remplacé (supersession) par
    // 300.00 EUR/mois à partir du 11 avril : avril = 90×10/30 + 300×20/30
    // = 230.00 exact (dénominateur = durée calendaire d'avril = 30 jours).
    let mut directives = base();
    directives.push(budget(
        "2026-04-01",
        "Expenses:Test",
        "monthly",
        dec!(90.00),
    ));
    directives.push(budget(
        "2026-04-11",
        "Expenses:Test",
        "monthly",
        dec!(300.00),
    ));

    let types = AccountTypes::default();
    let grid = BudgetGrid::build(&directives, &types, 2026);
    let row = grid.row("Expenses:Test").expect("ligne budget");
    assert_eq!(
        row.cells[3].budgeted,
        dec!(230.00),
        "avril (split 10/20 jours)"
    );
    assert_eq!(row.cells[4].budgeted, dec!(300.00), "mai : 300 plein");
    assert_eq!(row.cells[2].budgeted, dec!(0), "mars : rien en force");
}

#[test]
fn directive_datee_en_milieu_de_mois_pro_rata() {
    // Un budget mensuel de 300.00 daté du 11 avril (sans directive au 1er) :
    // avril = 300×20/30 = 200.00, mai = 300.00.
    let mut directives = base();
    directives.push(budget(
        "2026-04-11",
        "Expenses:Test",
        "monthly",
        dec!(300.00),
    ));

    let types = AccountTypes::default();
    let grid = BudgetGrid::build(&directives, &types, 2026);
    let row = grid.row("Expenses:Test").expect("ligne budget");
    assert_eq!(row.cells[3].budgeted, dec!(200.00));
    assert_eq!(row.cells[4].budgeted, dec!(300.00));
}

#[test]
fn reel_mensuel_et_fraction_consommee() {
    let mut directives = base();
    directives.push(budget(
        "2026-01-01",
        "Expenses:Test",
        "monthly",
        dec!(100.00),
    ));
    directives.push(spend("2026-01-10", "Expenses:Test", dec!(60.00)));
    directives.push(spend("2026-02-05", "Expenses:Test", dec!(75.00)));

    let types = AccountTypes::default();
    let grid = BudgetGrid::build(&directives, &types, 2026);
    let row = grid.row("Expenses:Test").expect("ligne budget");

    assert_eq!(
        row.cells[0],
        GridCell {
            budgeted: dec!(100.00),
            actual: dec!(60.00)
        }
    );
    assert_eq!(row.cells[0].used_fraction(), Some(0.6));
    assert_eq!(row.cells[1].actual, dec!(75.00));
    assert_eq!(row.cells[1].used_fraction().unwrap().round(), 1.0, "75/100");
    assert_eq!(row.budgeted_year(), dec!(1200.00));
    assert_eq!(row.actual_year(), dec!(135.00));
    assert_eq!(row.remaining_year(), dec!(1065.00));
    // Un budget mensuel reste en force tous les mois suivants (sémantique
    // Fava) : décembre est budgété 100.00 même sans écriture.
    assert_eq!(
        row.cells[11],
        GridCell {
            budgeted: dec!(100.00),
            actual: dec!(0.00)
        }
    );
}

#[test]
fn groupement_parent_enfants_addition_pas_absorption() {
    let mut directives = base();
    // Budget sur le parent ET sur l'enfant.
    directives.push(budget(
        "2026-01-01",
        "Expenses:Loisirs",
        "monthly",
        dec!(30.00),
    ));
    directives.push(budget(
        "2026-01-01",
        "Expenses:Loisirs:Sport",
        "monthly",
        dec!(20.00),
    ));
    directives.push(open("2026-01-01", "Expenses:Loisirs"));
    directives.push(open("2026-01-01", "Expenses:Loisirs:Sport"));
    directives.push(spend("2026-01-15", "Expenses:Loisirs:Sport", dec!(10.00)));

    let types = AccountTypes::default();
    let grid = BudgetGrid::build(&directives, &types, 2026);

    // Chaque poste garde SA ligne (pas d'absorption).
    let parent = grid.row("Expenses:Loisirs").expect("ligne parent");
    let child = grid.row("Expenses:Loisirs:Sport").expect("ligne enfant");

    // Le parent agrège l'enfant (30 + 20) ; l'enfant reste à son montant.
    assert_eq!(
        parent.cells[0].budgeted,
        dec!(50.00),
        "parent = soi + enfants"
    );
    assert_eq!(child.cells[0].budgeted, dec!(20.00), "enfant = soi seul");
    // Le réel remonte au parent par la même occasion.
    assert_eq!(parent.cells[0].actual, dec!(10.00));
    assert_eq!(child.cells[0].actual, dec!(10.00));

    // Les totaux de colonne comptent chaque budget une seule fois :
    // expenses = 50.00 budgété / 10.00 réel (pas 70.00 ni 20.00).
    let january = &grid.totals[0];
    let expenses = january
        .iter()
        .find(|t| t.bucket == "expenses")
        .expect("total expenses");
    assert_eq!(expenses.budgeted, dec!(50.00));
    assert_eq!(expenses.actual, dec!(10.00));
}

// -------------------------------------------------------------- rollover -----

#[test]
fn rollover_cumule_avec_report_negatif() {
    let cells = [
        GridCell {
            budgeted: dec!(100.00),
            actual: dec!(60.00),
        }, // +40
        GridCell {
            budgeted: dec!(100.00),
            actual: dec!(130.00),
        }, // −30 → 10
        GridCell {
            budgeted: dec!(100.00),
            actual: dec!(150.00),
        }, // −50 → −40
        GridCell {
            budgeted: dec!(100.00),
            actual: dec!(0.00),
        }, // +100 → 60
        GridCell::default(),
        GridCell::default(),
        GridCell::default(),
        GridCell::default(),
        GridCell::default(),
        GridCell::default(),
        GridCell::default(),
        GridCell::default(),
    ];
    let series = RolloverSeries::from_cells(&cells, Decimal::ZERO);

    assert_eq!(series.available[0], dec!(40.00), "janvier : 100 − 60");
    assert_eq!(series.available[1], dec!(10.00), "février : +100 − 130");
    assert_eq!(
        series.available[2],
        dec!(-40.00),
        "mars : dépassement reporté (négatif)"
    );
    assert_eq!(series.available[3], dec!(60.00), "avril : remonte à +100");
    assert_eq!(
        series.carry_out(),
        dec!(60.00),
        "report sortant = disponible décembre"
    );
    assert_eq!(
        series.cumulative_budget[3],
        dec!(400.00),
        "budget cumulé + report initial (0)"
    );
    assert_eq!(series.cumulative_actual[2], dec!(340.00));

    // Avec un report entrant de 25, tout se décale d'autant.
    let shifted = RolloverSeries::from_cells(&cells, dec!(25.00));
    assert_eq!(shifted.available[0], dec!(65.00));
    assert_eq!(shifted.carry_out(), dec!(85.00));
}

// ------------------------------------------------------------ duplication ----

#[test]
fn duplication_0_pour_cent_idempotente() {
    let directives = vec![
        open("2026-01-01", "Expenses:Test"),
        budget("2026-01-01", "Expenses:Test", "monthly", dec!(850.00)),
        budget("2026-03-01", "Expenses:Test", "monthly", dec!(0.00)),
    ];
    let duplicated =
        duplicate_year(&directives, 2026, 2027, dec!(0), dec!(0.01)).expect("duplication exacte");
    assert_eq!(duplicated.len(), 2);

    // Les dates au 1er du mois se décalent sans surprise (janvier → janvier,
    // mars → mars).
    let dates: Vec<String> = duplicated
        .iter()
        .filter_map(|d| match d {
            Directive::Custom(c) => Some(c.date.to_string()),
            _ => None,
        })
        .collect();
    assert_eq!(dates, vec!["2027-01-01", "2027-03-01"]);
    // Montants identiques (idempotence : refaire la même duplication avec
    // 0 % donne les mêmes lignes).
    let again = duplicate_year(&directives, 2026, 2027, dec!(0), dec!(0.01)).expect("refait");
    let amounts = |list: &[Directive]| -> Vec<String> {
        list.iter()
            .filter_map(|d| match d {
                Directive::Custom(c) => c.values.iter().find_map(|v| match v {
                    rustledger_core::MetaValue::Amount(a) => Some(a.number.to_string()),
                    _ => None,
                }),
                _ => None,
            })
            .collect()
    };
    assert_eq!(amounts(&duplicated), amounts(&again));
}

#[test]
fn duplication_plus_pour_cent_arrondi_multiple_superieur() {
    let directives = vec![
        open("2026-01-01", "Expenses:Test"),
        // 850.00 × 1.06 = 901.00 exact ; 31.99 × 1.06 = 33.9094 → 34.00
        // (pas 1.00 €) ; 0.00 × 1.06 = 0.00.
        budget("2026-01-01", "Expenses:Test", "monthly", dec!(850.00)),
        budget("2026-01-01", "Expenses:Internet", "monthly", dec!(31.99)),
        budget("2026-01-01", "Expenses:Zero", "monthly", dec!(0.00)),
    ];
    let duplicated =
        duplicate_year(&directives, 2026, 2027, dec!(6), dec!(1.00)).expect("duplication +6 %");
    // Tri canonique (date, compte) : Internet < Test < Zero.
    let amounts: Vec<Decimal> = duplicated
        .iter()
        .filter_map(|d| match d {
            Directive::Custom(c) => c.values.iter().find_map(|v| match v {
                rustledger_core::MetaValue::Amount(a) => Some(a.number),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(amounts, vec![dec!(34.00), dec!(901.00), dec!(0.00)]);

    // Pas de 0.01 (centimes) : 33.9094 → 33.91.
    let duplicated_cents =
        duplicate_year(&directives, 2026, 2027, dec!(6), dec!(0.01)).expect("pas centime");
    let internet = duplicated_cents
        .iter()
        .filter_map(|d| match d {
            Directive::Custom(c) => match &c.values.first() {
                Some(rustledger_core::MetaValue::Account(account))
                    if account.as_str() == "Expenses:Internet" =>
                {
                    c.values.iter().find_map(|v| match v {
                        rustledger_core::MetaValue::Amount(a) => Some(a.number),
                        _ => None,
                    })
                }
                _ => None,
            },
            _ => None,
        })
        .next()
        .expect("montant Internet");
    assert_eq!(internet, dec!(33.91));
}

#[test]
fn duplication_refusee_si_annee_cible_deja_budgatee() {
    let directives = vec![
        open("2026-01-01", "Expenses:Test"),
        budget("2026-01-01", "Expenses:Test", "monthly", dec!(100.00)),
        budget("2027-01-01", "Expenses:Test", "monthly", dec!(110.00)),
    ];
    let error = duplicate_year(&directives, 2026, 2027, dec!(0), dec!(0.01))
        .expect_err("année cible déjà budgétée");
    assert!(
        error.to_string().contains("déjà présent"),
        "message clair : {error}"
    );
}

#[test]
fn duplication_refuse_pas_darrondi_nul() {
    let directives = vec![budget("2026-01-01", "Expenses:Test", "monthly", dec!(1.00))];
    assert!(duplicate_year(&directives, 2026, 2027, dec!(0), dec!(0)).is_err());
}

// ------------------------------------------------------------------ divers ---

#[test]
fn bornes_de_fenetres_mensuelles_sont_exactes() {
    let january = grid::month_start(2026, 1);
    let february = grid::month_start_exclusive(2026, 1);
    assert_eq!(january.to_string(), "2026-01-01");
    assert_eq!(february.to_string(), "2026-02-01");
    // Décembre → 1er janvier de l'année suivante.
    assert_eq!(
        grid::month_start_exclusive(2026, 12).to_string(),
        "2027-01-01"
    );
}

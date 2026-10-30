//! Tests d'intégration du jalon 7 : données de rapports (catégories × mois,
//! revenus, épargne, taux) sur le ledger réel, et le message suggéré de
//! l'écran Git (`household_git::suggere_message` est testé sur dépôts
//! réels dans le crate `household-git`).

mod common;

use household_core::Snapshot;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rustledger_core::{AccountTypes, Directive, NaiveDate};

use household_gui::viewmodel;

const MAIN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ledger/main.beancount");

fn donnees(annee: i16) -> Option<(Vec<Directive>, AccountTypes, viewmodel::RapportsData)> {
    let main = std::path::Path::new(MAIN);
    if !main.exists() {
        eprintln!("ledger absent de la sandbox → suite ignorée");
        return None;
    }
    let snapshot = Snapshot::load(main).expect("chargement");
    let types = AccountTypes {
        assets: snapshot.ledger().options.name_assets.clone(),
        liabilities: snapshot.ledger().options.name_liabilities.clone(),
        equity: snapshot.ledger().options.name_equity.clone(),
        income: snapshot.ledger().options.name_income.clone(),
        expenses: snapshot.ledger().options.name_expenses.clone(),
    };
    let directives: Vec<Directive> = snapshot.entries().iter().map(|e| e.value.clone()).collect();
    Some((
        directives.clone(),
        types.clone(),
        viewmodel::rapports(&directives, &types, annee),
    ))
}

#[test]
fn rapports_2026_categories_et_totaux() {
    let Some((_d, _t, rapports)) = donnees(2026) else {
        return;
    };
    assert_eq!(rapports.annee, 2026);

    // Cohérence : Σ des catégories × mois = Σ des dépenses totales.
    let somme_categories: Decimal = rapports
        .depenses_categories
        .values()
        .flat_map(|c| c.iter().copied())
        .sum();
    // Chaque budget est un posting Expenses → les catégories ne
    // double-comptent pas leur parent.
    assert_eq!(somme_categories, rapports.depenses_annee());

    // Catégorie concrète : Courses a des dépenses (janv. réel cohérent avec
    // le tableau de bord du jalon 4 : real ~ 309.85 en mars vient
    // d'Alimentation:Courses; la catégorie = Expenses:Alimentation).
    let alimentation = rapports
        .depenses_categories
        .get("Expenses:Alimentation")
        .expect("catégorie présente");
    assert!(alimentation.iter().any(|m| !m.is_zero()));

    // Mars a des dépenses (données réelles des transactions d'exemple).
    assert!(rapports.depenses_mois[2] > Decimal::ZERO);
}

#[test]
fn rapports_epargne_revenus_taux() {
    let Some((_d, _t, rapports)) = donnees(2026) else {
        return;
    };

    // L'épargne a des variations (transfert janv. 2026 : 500 € vers LivretA).
    assert!(rapports.epargne_annee() > dec!(0), "transferts positifs");

    // Revenus positifs (salaire + intérêts).
    assert!(rapports.revenus_annee() > dec!(0));

    // Taux d'épargne plausiblement non négatif sur l'année d'exemple.
    if let Some(taux) = rapports.taux_epargne() {
        assert!(taux >= Decimal::ZERO, "taux = {taux}");
    }
}

#[test]
fn rapports_comparaison_n_vs_n1_absente_ou_presente_selon_les_donnees() {
    let Some((directives, types, _)) = donnees(2026) else {
        return;
    };
    // 2025 contient des transactions d'exemple (déc. 2025) mais aucune
    // dépense Expenses → l'année précédente n'est PAS proposée.
    let precedente = viewmodel::rapports(&directives, &types, 2025);
    assert!(precedente.depenses_annee().is_zero());

    let courante = viewmodel::rapports(&directives, &types, 2026);
    assert!(
        courante.depenses_annee_precedente.is_none(),
        "N−1 (2025) sans dépenses → pas de comparaison tracée"
    );
}

#[test]
fn rapports_dans_le_view_model_complet() {
    let main = std::path::Path::new(MAIN);
    if !main.exists() {
        eprintln!("ledger absent de la sandbox → test ignoré");
        return;
    }
    let snapshot = Snapshot::load(main).expect("chargement");
    let aujourdhui: NaiveDate = "2026-10-30".parse().expect("date de référence");
    let data = viewmodel::build(&snapshot, aujourdhui).expect("view-model");
    assert_eq!(data.rapports.annee, 2026);
    assert!(data.rapports.depenses_annee() > Decimal::ZERO);
}

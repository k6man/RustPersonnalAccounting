//! Tests d'intégration de l'écran « Graphiques » : données pures du
//! view-model sur le ledger réel (aucune GUI — les vues sont de la
//! présentation, vérifiées à la capture).

use household_core::Snapshot;
use rust_decimal_macros::dec;

use household_gui::viewmodel;
use rustledger_core::NaiveDate;

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

/// La cascade suit l'identité comptable Revenus − Dépenses = Δ patrimoine,
/// avec Δ patrimoine vérifié aux deux bornes golden du ledger
/// (ouvertures au 2025-01-01 : patrimoine fin 2025 = 32 014,71 € ;
/// fin de ledger = 36 946,87 €, cf. tests/viewmodel.rs et golden.rs).
#[test]
fn cascade_identite_comptable() {
    let Some(data) = donnees() else { return };
    let cascade = &data.graphiques.cascade;

    // Δ patrimoine = 36 946,87 (fin ledger) − 32 014,71 (fin 2025,
    // comptes aux valeurs des assertions du 2026-01-01 00:00).
    assert_eq!(
        cascade.variation_patrimoine,
        dec!(36946.87) - dec!(32014.71),
        "Δ patrimoine entre les deux bornes golden"
    );

    // L'identité : pas une approximation — tout posting EUR complet
    // appartient à Revenus, Dépenses ou au bilan.
    assert_eq!(
        cascade.revenus - cascade.depenses,
        cascade.variation_patrimoine,
        "Revenus − Dépenses = Δ patrimoine (identité exacte)"
    );

    assert!(cascade.revenus > dec!(0), "salaires + remboursements 2026");
    assert!(cascade.depenses > dec!(0), "dépenses 2026");
    assert!(cascade.epargne > dec!(0), "virements d'épargne + intérêts");
}

/// La répartition couvre exactement les dépenses de la période : top 8
/// catégories + agrégat « Autres » = total (les parts somment à 100 %),
/// tri décroissant hors ligne « Autres ».
#[test]
fn repartition_couvre_les_depenses() {
    let Some(data) = donnees() else { return };
    let graphiques = &data.graphiques;

    // Année.
    assert_eq!(graphiques.annee, 2026);
    assert!(!graphiques.repartition_annee.is_empty());
    let total_annee: rust_decimal::Decimal =
        graphiques.repartition_annee.iter().map(|(_, t)| *t).sum();
    assert_eq!(
        total_annee,
        data.rapports.depenses_annee(),
        "somme = total année"
    );

    // Ordre décroissant sur la partie top (hors « Autres »).
    let top = graphiques
        .repartition_annee
        .iter()
        .filter(|(nom, _)| !nom.starts_with("Autres "))
        .map(|(_, total)| *total)
        .collect::<Vec<_>>();
    for paire in top.windows(2) {
        assert!(paire[0] > paire[1], "tri décroissant");
    }
    assert!(
        graphiques
            .repartition_annee
            .iter()
            .all(|(_, t)| *t > rust_decimal::Decimal::ZERO),
        "les catégories nulles sont filtrées"
    );

    // Mois courant (mars) : même couverture sur le total du mois.
    assert_eq!(graphiques.mois, 3);
    let total_mois: rust_decimal::Decimal =
        graphiques.repartition_mois.iter().map(|(_, t)| *t).sum();
    assert_eq!(
        total_mois,
        data.rapports.depenses_mois[usize::from(graphiques.mois) - 1],
        "somme = total du mois"
    );
}

/// Budget vs réel de la fenêtre = cellules du mois de la grille budget
/// (même donnée que la carte du tableau de bord, même tri).
#[test]
fn budget_reel_cohere_avec_la_grille() {
    let Some(data) = donnees() else { return };
    let bord = &data.tableau_de_bord;
    let grille = data
        .grilles
        .get(&data.graphiques.annee)
        .expect("grille de l'année courante");
    assert!(!bord.budget_mois.is_empty(), "2026 est budgétée");

    let index_mois = usize::from(data.graphiques.mois) - 1;
    // Cohérence cellule par cellule (top 8 affiché, mais vérifions tout
    // le vecteur : c'est la même source).
    for poste in &bord.budget_mois {
        let ligne = grille.row(&poste.compte).expect("ligne de la grille");
        assert_eq!(
            poste.budgete, ligne.cells[index_mois].budgeted,
            "{}",
            poste.compte
        );
        assert_eq!(
            poste.reel, ligne.cells[index_mois].actual,
            "{}",
            poste.compte
        );
    }

    // Tri « plus tendus en tête » : tout dépassement précède tout
    // non-dépassement, puis écart croissant.
    for paire in bord.budget_mois.windows(2) {
        let depasse_a = paire[0].reel > paire[0].budgete;
        let depasse_b = paire[1].reel > paire[1].budgete;
        assert!(
            depasse_a.cmp(&depasse_b) != std::cmp::Ordering::Less,
            "sur-budget d'abord"
        );
        if depasse_a == depasse_b {
            assert!(paire[0].ecart() <= paire[1].ecart(), "écart croissant");
        }
    }
}

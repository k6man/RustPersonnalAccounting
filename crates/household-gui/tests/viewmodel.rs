//! Tests d'intégration du view-model : assemblage depuis le ledger réel
//! (aucune GUI — les structs sont purs).

use household_core::Snapshot;
use rust_decimal_macros::dec;

use household_gui::viewmodel::{self, EtatJauge};
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

#[test]
fn soldes_et_patrimoine_du_tableau_de_bord() {
    let Some(data) = donnees() else { return };
    let bord = &data.tableau_de_bord;

    assert_eq!(bord.solde_cc, Some(dec!(6877.39)));
    assert_eq!(bord.solde_livret_a, Some(dec!(19602.49)));
    assert_eq!(bord.solde_ldd, Some(dec!(10466.99)));

    // Patrimoine = CC + LivretA + LDD (pas de passif ; Equity exclu —
    // l'ouverture historique ne s'annule pas avec les assets).
    assert_eq!(
        bord.patrimoine_net,
        dec!(6877.39) + dec!(19602.49) + dec!(10466.99)
    );
}

#[test]
fn jauges_de_plafond_dans_le_vert() {
    let Some(data) = donnees() else { return };
    let bord = &data.tableau_de_bord;

    assert_eq!(bord.jauges.len(), 2, "LivretA + LDD");
    for jauge in &bord.jauges {
        assert_eq!(jauge.etat, EtatJauge::Ok, "{} sous 90 %", jauge.compte);
        assert!(jauge.courant < jauge.plafond);
    }
    let livret = bord
        .jauges
        .iter()
        .find(|j| j.compte == "Assets:Epargne:LivretA")
        .expect("jauge LivretA");
    assert_eq!(livret.plafond, dec!(22950.00));
    assert_eq!(livret.courant, dec!(19602.49));

    // Les trois états de jauge, purement calculés.
    assert_eq!(EtatJauge::of(dec!(1), dec!(2)), EtatJauge::Ok);
    assert_eq!(
        EtatJauge::of(dec!(21), dec!(22)),
        EtatJauge::Alerte,
        "≥ 90 %"
    );
    assert_eq!(EtatJauge::of(dec!(23), dec!(22)), EtatJauge::Depassement);
    assert_eq!(
        EtatJauge::of(dec!(5), dec!(0)),
        EtatJauge::Depassement,
        "plafond nul"
    );
    assert_eq!(EtatJauge::of(dec!(0), dec!(0)), EtatJauge::Ok);
}

#[test]
fn budget_du_mois_courant_present_et_trie() {
    let Some(data) = donnees() else { return };
    let bord = &data.tableau_de_bord;

    assert_eq!(bord.budget_mois_label, "2026-03");
    assert!(!bord.budget_mois.is_empty(), "le budget 2026 est chargé");

    // Valeurs croisées avec `rledger report budget --from 2026-03-01
    // --to 2026-04-01 --children` : Courses réel de MARS = 309.85 (le
    // 833.05 de `balances` est le cumul depuis janvier).
    let courses = bord
        .budget_mois
        .iter()
        .find(|p| p.compte == "Expenses:Alimentation:Courses")
        .expect("ligne Courses");
    assert_eq!(courses.budgete, dec!(850.00));
    assert_eq!(courses.reel, dec!(309.85));

    // Vacances mars : directive du 2026-03-01 (640.00) en force, dépensée.
    let vacances = bord
        .budget_mois
        .iter()
        .find(|p| p.compte == "Expenses:Vacances")
        .expect("ligne Vacances");
    assert_eq!(vacances.budgete, dec!(640.00));
    assert_eq!(vacances.reel, dec!(640.00));

    // Tri : sur-budget d'abord (préfixe), puis écart croissant.
    // Invariant : après la première ligne NON dépassée, plus aucun dépassement.
    let sur_budget_en_premier = bord
        .budget_mois
        .iter()
        .position(|p| p.reel <= p.budgete)
        .is_none_or(|premier_non_sur_budget| {
            bord.budget_mois[premier_non_sur_budget..]
                .iter()
                .all(|p| p.reel <= p.budgete)
        });
    assert!(sur_budget_en_premier, "les dépassements sont en tête");
    let habillement = bord
        .budget_mois
        .iter()
        .find(|p| p.compte == "Expenses:Habillement")
        .expect("ligne Habillement");
    assert!(habillement.reel > habillement.budgete, "dépassé en mars");
}

#[test]
fn courbe_epargne_mensuelle_exacte() {
    let Some(data) = donnees() else { return };
    let courbe = &data.tableau_de_bord.courbe;

    // De janvier 2025 (mois de l'ouverture) à mars 2026 (dernier mois).
    let etiquettes: Vec<&str> = courbe.iter().map(|(m, _)| m.as_str()).collect();
    assert_eq!(etiquettes.first(), Some(&"2025-01"), "mois de l'ouverture");
    assert_eq!(
        etiquettes.last(),
        Some(&"2026-03"),
        "dernier mois transactionné"
    );
    assert_eq!(etiquettes.len(), 15, "un point par mois, sans trou");

    // Fin janvier 2025 : les soldes d'ouverture (2845.23 + 18450.12 + 9870.55).
    let janvier = courbe
        .iter()
        .find(|(m, _)| m == "2025-01")
        .expect("point janvier 2025");
    assert_eq!(janvier.1, dec!(31165.90));
    // Fin décembre 2025 : après intérêts (2845.23 + 19002.49 + 10166.99).
    let decembre = courbe
        .iter()
        .find(|(m, _)| m == "2025-12")
        .expect("point décembre 2025");
    assert_eq!(decembre.1, dec!(32014.71));
    // Fin mars 2026 = état final.
    assert_eq!(courbe.last().expect("point final").1, dec!(36946.87));
}

#[test]
fn journal_complet_avec_sources_et_montants() {
    let Some(data) = donnees() else { return };
    let lignes = &data.journal.lignes;

    // Toutes les transactions du ledger (3 mois + ouverture/intérêts).
    assert!(lignes.len() > 30, "journal non vide : {}", lignes.len());

    // Trié par date croissante (ordre du ledger).
    for paire in lignes.windows(2) {
        assert!(paire[0].date <= paire[1].date, "dates non décroissantes");
    }

    // Chaque ligne a des postings, un montant principal, une source.
    for ligne in lignes {
        assert!(!ligne.lignes_posting.is_empty());
        assert!(ligne.montant.is_some(), "montant principal calculé");
        assert!(ligne.ligne > 0, "localisation fichier:ligne");
    }

    // La transaction en attente (flag '!') est repérable.
    let en_attente = lignes.iter().find(|l| l.flag == '!');
    assert!(en_attente.is_some(), "le ledger échantillon en a une");

    // Une ligne concrète : le loyer de janvier.
    let loyer = lignes
        .iter()
        .find(|l| l.libelle.contains("Loyer"))
        .expect("transaction loyer");
    assert!(
        loyer
            .lignes_posting
            .iter()
            .any(|p| p.compte == "Expenses:Logement:Loyer-Credit"),
        "posting loyer présent"
    );
    assert!(
        loyer
            .lignes_posting
            .iter()
            .any(|p| p.compte == "Assets:Banque:CompteCourant"),
        "contrepartie CC présente"
    );
    assert!(
        loyer.categories.contains("Expenses:"),
        "catégories calculées"
    );
}

#[test]
fn erreurs_de_validation_propagees() {
    let Some(data) = donnees() else { return };
    assert!(data.erreurs.is_empty(), "ledger échantillon valide");
}

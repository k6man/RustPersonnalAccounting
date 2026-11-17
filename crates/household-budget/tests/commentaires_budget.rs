//! Tests du commentaire de ligne budgétaire **dans le ledger** : écriture
//! (`set_comment`) sur copie scratch du ledger échantillon — insertion,
//! remplacement, suppression (round-trip à l'octet près), refus sans
//! directive — et lecture (`commentaires`) sur le fichier écrit.

mod common;

use household_budget::{commentaires, set_comment};
use household_core::Snapshot;

use common::{budget_text, cleanup, ledger_copy};

const COURSES: &str = "Expenses:Alimentation:Courses";

#[test]
fn set_comment_insere_remplace_supprime_octet_pres() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");
    let original = budget_text(&main, 2026);

    // 1. Insertion : la note apparaît juste au-dessus de la première
    //    directive Courses (2026-01-01), le reste du fichier est intact.
    let after = set_comment(&snapshot, 2026, COURSES, "Courses hebdo du foyer")
        .expect("insertion commentaire");
    assert!(after.is_valid(), "{:?}", after.errors());
    // Le snapshot porte des offsets d'octets : recharger après écriture
    // (c'est ce que fait le worker GUI à chaque commande).
    let snapshot = Snapshot::load(&main).expect("relecture");
    let texte = budget_text(&main, 2026);
    let pos_note = texte
        .find("; Courses hebdo du foyer")
        .expect("note insérée");
    let pos_directive = texte
        .find("2026-01-01 custom \"budget\" Expenses:Alimentation:Courses")
        .expect("directive Courses");
    assert!(
        texte[pos_note..pos_directive].ends_with('\n'),
        "la note est collée à la directive"
    );
    // Les octets hors du bloc inséré sont identiques.
    let sans_note = texte.replace("; Courses hebdo du foyer\n", "");
    assert_eq!(sans_note, original, "reste du fichier intact");

    // 2. Remplacement : la note existante est réécrite sur place (toujours
    //    une seule ligne de commentaire).
    let after2 = set_comment(&snapshot, 2026, COURSES, "budget mensuel complet")
        .expect("remplacement commentaire");
    assert!(after2.is_valid(), "{:?}", after2.errors());
    let texte2 = budget_text(&main, 2026);
    assert!(
        !texte2.contains("Courses hebdo du foyer"),
        "ancienne note remplacée"
    );
    assert_eq!(
        texte2.matches("; budget mensuel complet").count(),
        1,
        "une seule note"
    );
    // La lecture joint ce qu'une édition manuelle aurait pu mettre sur
    // plusieurs lignes ; ici l'écriture est mono-ligne.
    let map = commentaires(&texte2);
    assert_eq!(
        map.get(COURSES).map(String::as_str),
        Some("budget mensuel complet")
    );

    // 3. Suppression : commentaire vide → le fichier retrouve ses octets
    //    d'origine.
    let after3 = set_comment(&snapshot, 2026, COURSES, "").expect("suppression commentaire");
    assert!(after3.is_valid(), "{:?}", after3.errors());
    assert_eq!(
        budget_text(&main, 2026),
        original,
        "round-trip à l'octet près"
    );

    cleanup(&scratch);
}

#[test]
fn set_comment_s_attache_a_la_premiere_directive_du_compte() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");

    // Courses a plusieurs directives (supersessions) : le commentaire se
    // met au-dessus de la première (janvier), pas des suivantes.
    set_comment(&snapshot, 2026, COURSES, "poste n°1 du foyer").expect("écriture");
    let texte = budget_text(&main, 2026);
    let pos_premiere = texte
        .find("2026-01-01 custom \"budget\" Expenses:Alimentation:Courses")
        .expect("directive janvier");
    let pos_autre = texte
        .find("2026-07-01 custom \"budget\" Expenses:Alimentation:Courses")
        .expect("supersession juillet");
    assert!(
        texte[pos_premiere.saturating_sub(80)..pos_premiere].contains("; poste n°1 du foyer"),
        "note au-dessus de la première directive"
    );
    assert!(
        !texte[pos_premiere..pos_autre].contains("; poste"),
        "une seule note : les supersessions ne sont pas touchées"
    );

    cleanup(&scratch);
}

#[test]
fn set_comment_refuse_sans_directive_et_preserve_les_autres() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");
    let avant = budget_text(&main, 2026);

    // Compte absent du budget 2026 : refus.
    let err = set_comment(&snapshot, 2026, "Expenses:Inexistant", "note")
        .expect_err("compte sans directive");
    assert!(
        err.to_string().contains("Expenses:Inexistant"),
        "message explicite : {err}"
    );

    // Année sans fichier : refus aussi.
    let err2 = set_comment(&snapshot, 2099, COURSES, "note").expect_err("année absente");
    assert!(
        err2.to_string().contains("2099"),
        "message explicite : {err2}"
    );

    // Aucun fichier touché par les refus.
    assert_eq!(budget_text(&main, 2026), avant);

    // Deux comptes peuvent avoir des notes différentes : recharger le
    // snapshot entre les deux écritures (offsets d'octets).
    set_comment(&snapshot, 2026, COURSES, "courses").expect("courses");
    let snapshot = Snapshot::load(&main).expect("relecture");
    set_comment(&snapshot, 2026, "Expenses:Assurances", "assurances").expect("assurances");
    let map = commentaires(&budget_text(&main, 2026));
    assert_eq!(map.get(COURSES).map(String::as_str), Some("courses"));
    assert_eq!(
        map.get("Expenses:Assurances").map(String::as_str),
        Some("assurances")
    );

    cleanup(&scratch);
}

#[test]
fn set_comment_sur_ledger_edite_a_la_main() {
    // Un commentaire collé par une édition manuelle juste au-dessus de la
    // directive est remplacé (pas dupliqué) — même convention que la lecture.
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let snapshot = Snapshot::load(&main).expect("copie valide");

    set_comment(&snapshot, 2026, "Assets:Epargne:LDD", "épargne automatique").expect("écriture");
    // Re-snapshot depuis le ledger modifié, puis réécriture.
    let snapshot2 = Snapshot::load(&main).expect("relecture");
    set_comment(&snapshot2, 2026, "Assets:Epargne:LDD", "virement mensuel").expect("réécriture");
    let texte = budget_text(&main, 2026);
    assert_eq!(
        texte.matches("; virement mensuel").count(),
        1,
        "pas de duplication"
    );
    assert!(!texte.contains("épargne automatique"));
    assert_eq!(
        commentaires(&texte)
            .get("Assets:Epargne:LDD")
            .map(String::as_str),
        Some("virement mensuel")
    );

    cleanup(&scratch);
}

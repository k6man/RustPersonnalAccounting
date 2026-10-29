//! Helpers purs de l'éditeur de budget : arbre hiérarchique (avec nœuds
//! d'agrégation synthétisés), visibilité repliable, rollover, totaux.
//! Aucune dépendance egui — testé.

use std::collections::BTreeSet;

use rust_decimal::Decimal;

use household_budget::{GridCell, GridRow, RolloverSeries};

const ZERO_CELLES: GridCell = GridCell {
    budgeted: Decimal::ZERO,
    actual: Decimal::ZERO,
};

/// Un nœud de l'arbre budgétaire : soit un poste **budgété** (cellules
/// propres, déjà agrégées avec ses enfants par le moteur `children=true`),
/// soit un **groupe synthétisé** (somme des lignes budgétées de tête de sa
/// sous-arborescence — chaque budget compté une fois).
#[derive(Debug, Clone)]
pub struct NoeudBudget {
    pub compte: String,
    pub cellules: [GridCell; 12],
    /// A une directive `custom "budget"` propre (donc éditable).
    pub budgete: bool,
    /// A des descendants dans l'arbre (triangle de repli).
    pub a_enfants: bool,
    /// Segments − 1 (0 = racine de type `Expenses`).
    pub profondeur: usize,
}

/// Construit l'arbre : union des préfixes de tous les postes budgétés ; un
/// préfixe intermédiaire (non budgété lui-même) devient un nœud d'agrégation.
#[must_use]
pub fn arbre(lignes: &[GridRow]) -> Vec<NoeudBudget> {
    // 1. Tous les préfixes présents (comptes complets inclus).
    let mut prefixes: BTreeSet<String> = BTreeSet::new();
    for ligne in lignes {
        for ancetre in ancetres(&ligne.account) {
            prefixes.insert(ancetre);
        }
        prefixes.insert(ligne.account.clone());
    }

    // 2. Index des lignes budgétées par compte.
    let par_compte: std::collections::BTreeMap<&str, &GridRow> =
        lignes.iter().map(|l| (l.account.as_str(), l)).collect();

    // 3. Nœuds triés lexicographiquement.
    let mut noeuds = Vec::with_capacity(prefixes.len());
    for compte in &prefixes {
        let budgete = par_compte.contains_key(compte.as_str());
        let cellules = if let Some(ligne) = par_compte.get(compte.as_str()) {
            ligne.cells
        } else {
            somme_sous_arbre(lignes, compte)
        };
        let a_enfants = prefixes
            .iter()
            .any(|autre| autre.starts_with(format!("{compte}:").as_str()));
        noeuds.push(NoeudBudget {
            compte: compte.clone(),
            cellules,
            budgete,
            a_enfants,
            profondeur: compte.split(':').count().saturating_sub(1),
        });
    }
    noeuds
}

/// Somme des lignes budgétées **de tête** sous un préfixe : une ligne dont
/// un ancêtre (strictement sous le préfixe) est lui-même budgété est déjà
/// comptée dedans — on ne la re-somme pas.
fn somme_sous_arbre(lignes: &[GridRow], prefixe: &str) -> [GridCell; 12] {
    let mut total = [ZERO_CELLES; 12];
    for ligne in lignes {
        if !ligne.account.starts_with(format!("{prefixe}:").as_str()) {
            continue;
        }
        let intermediaire_budgete = ancetres(&ligne.account)
            .into_iter()
            .skip_while(|ancetre| ancetre.as_str() != prefixe)
            .skip(1)
            .any(|ancetre| {
                lignes
                    .iter()
                    .any(|l| l.account == ancetre && l.account != prefixe)
            });
        if intermediaire_budgete {
            continue;
        }
        for (total_cellule, cellule) in total.iter_mut().zip(ligne.cells.iter()) {
            total_cellule.budgeted += cellule.budgeted;
            total_cellule.actual += cellule.actual;
        }
    }
    total
}

/// Ancêtres d'un compte, du plus court au plus long (`Expenses:A:B` →
/// `Expenses`, `Expenses:A`).
#[must_use]
pub fn ancetres(compte: &str) -> Vec<String> {
    let segments: Vec<&str> = compte.split(':').collect();
    let mut out = Vec::with_capacity(segments.len().saturating_sub(1));
    let mut courant = String::new();
    for segment in &segments[..segments.len().saturating_sub(1)] {
        if courant.is_empty() {
            courant = (*segment).to_string();
        } else {
            courant = format!("{courant}:{segment}");
        }
        out.push(courant.clone());
    }
    out
}

/// Indices des lignes **visibles** : une ligne est masquée dès qu'un de ses
/// ancêtres de l'arbre est replié. (`repliees` = comptes repliés.)
#[must_use]
pub fn visibles(arbre: &[NoeudBudget], repliees: &BTreeSet<String>) -> Vec<usize> {
    arbre
        .iter()
        .enumerate()
        .filter(|(_, noeud)| {
            !ancetres(&noeud.compte)
                .iter()
                .any(|ancetre| repliees.contains(ancetre.as_str()))
        })
        .map(|(index, _)| index)
        .collect()
}

/// Indices des lignes « de tête » : pas d'ancêtre budgété au-dessus dans
/// l'arbre. Là vivent les toggles rollover.
#[must_use]
pub fn tetes(arbre: &[NoeudBudget]) -> Vec<usize> {
    let budgetes: BTreeSet<&str> = arbre
        .iter()
        .filter(|n| n.budgete)
        .map(|n| n.compte.as_str())
        .collect();
    arbre
        .iter()
        .enumerate()
        .filter(|(_, noeud)| {
            !ancetres(&noeud.compte)
                .iter()
                .any(|ancetre| budgetes.contains(ancetre.as_str()))
        })
        .map(|(index, _)| index)
        .collect()
}

/// Série de rollover d'un nœud (cumul annuel + report vs réel cumulé),
/// report initial nul (sémantique « cumul annuel », TODO Q3/Q6).
#[must_use]
pub fn rollover_de(noeud: &NoeudBudget) -> RolloverSeries {
    RolloverSeries::from_cells(&noeud.cellules, Decimal::ZERO)
}

/// Cellule « disponible » du rollover pour un mois (0-based).
#[must_use]
pub fn disponible(noeud: &NoeudBudget, mois_zero: usize) -> Decimal {
    rollover_de(noeud).available[mois_zero.min(11)]
}

/// Note affichée pour les postes d'épargne (intérêts annuels crédités sur
/// le même compte → gonflent le « réel » ; cf. TODO Q2).
#[must_use]
pub fn note_epargne(compte: &str) -> Option<&'static str> {
    compte
        .starts_with("Assets:Epargne")
        .then_some("les intérêts annuels crédités sur ce compte augmentent aussi son « réel »")
}

/// Total budgété d'un mois (toutes catégories comptées une fois — les
/// totaux de la grille, jamais la somme des lignes qui double-compte).
#[must_use]
pub fn total_mois(totaux: &[Vec<household_budget::ColumnTotal>], mois_zero: usize) -> Decimal {
    totaux
        .get(mois_zero)
        .map(|colonnes| {
            colonnes
                .iter()
                .filter(|t| t.currency == "EUR")
                .map(|t| t.budgeted)
                .sum()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn ligne(compte: &str, budgete: Decimal) -> GridRow {
        GridRow {
            account: compte.to_string(),
            currency: "EUR".to_string(),
            cells: [GridCell {
                budgeted: budgete,
                actual: Decimal::ZERO,
            }; 12],
        }
    }

    #[test]
    fn ancetres_dans_l_ordre() {
        assert_eq!(
            ancetres("Expenses:Enfants:Vetements"),
            vec!["Expenses", "Expenses:Enfants"]
        );
        assert!(ancetres("Expenses").is_empty());
    }

    #[test]
    fn arbre_synthetise_les_groupes_non_budgetes() {
        // Seuls les FEUILLES sont budgétées (cas du ledger réel) : les
        // groupes intermédiaires deviennent des nœuds d'agrégation.
        let lignes = vec![
            ligne("Expenses:Enfants:Vetements", dec!(35)),
            ligne("Expenses:Enfants:Cantine", dec!(70)),
            ligne("Expenses:Vacances", dec!(7)),
        ];
        let arbre = arbre(&lignes);
        let noms: Vec<&str> = arbre.iter().map(|n| n.compte.as_str()).collect();
        assert_eq!(
            noms,
            vec![
                "Expenses",
                "Expenses:Enfants",
                "Expenses:Enfants:Cantine",
                "Expenses:Enfants:Vetements",
                "Expenses:Vacances",
            ]
        );

        // Le groupe synthétisé somme ses feuilles de tête (105), comptées
        // une fois ; le nœud racine Expenses agrège tout (112).
        let groupe = arbre
            .iter()
            .find(|n| n.compte == "Expenses:Enfants")
            .unwrap();
        assert!(!groupe.budgete, "groupe sans directive propre");
        assert!(groupe.a_enfants);
        assert_eq!(groupe.cellules[0].budgeted, dec!(105));
        let racine = arbre.iter().find(|n| n.compte == "Expenses").unwrap();
        assert_eq!(racine.cellules[0].budgeted, dec!(112));
        assert_eq!(racine.profondeur, 0);
    }

    #[test]
    fn arbre_ne_double_compte_pas_les_intermediaires_budgetes() {
        // Budget sur le parent ET l'enfant : le parent est déjà couvert par
        // l'agrégat du groupe (children=true) — pas de re-somme.
        let lignes = vec![
            ligne("Assets:Epargne", dec!(3)),
            ligne("Assets:Epargne:LivretA", dec!(2)),
            ligne("Assets:Epargne:LDD", dec!(1)),
        ];
        let arbre = arbre(&lignes);
        let groupe = arbre.iter().find(|n| n.compte == "Assets:Epargne").unwrap();
        assert!(groupe.budgete, "directive propre → cellules du moteur");
        assert_eq!(groupe.cellules[0].budgeted, dec!(3), "3 = 3+2+1 (children)");
        // Le groupe ne re-somme pas LivretA/LDD par-dessus.
        let racine = arbre.iter().find(|n| n.compte == "Assets").unwrap();
        assert_eq!(racine.cellules[0].budgeted, dec!(3), "une seule fois");
    }

    #[test]
    fn visibilite_repliee_par_ancetre() {
        let lignes = vec![ligne("Expenses:Enfants:Vetements", dec!(35))];
        let arbre = arbre(&lignes);
        let mut repliees = BTreeSet::new();
        assert_eq!(
            visibles(&arbre, &repliees).len(),
            arbre.len(),
            "tout ouvert"
        );

        // Replier une racine : le nœud reste visible (c'est l'en-tête), ses
        // descendants masqués.
        repliees.insert("Expenses".to_string());
        let visibles_liste = visibles(&arbre, &repliees);
        assert_eq!(visibles_liste, vec![0], "seul l'en-tête replié reste");

        repliees.clear();
        repliees.insert("Expenses:Enfants".to_string());
        let visibles_liste = visibles(&arbre, &repliees);
        // Expenses et Expenses:Enfants visibles ; la feuille masquée.
        assert_eq!(visibles_liste.len(), 2);
    }

    #[test]
    fn tetes_excluent_les_noeuds_couverts_par_un_budget_ancetre() {
        let lignes = vec![
            ligne("Assets:Epargne", dec!(3)),
            ligne("Assets:Epargne:LivretA", dec!(2)),
            ligne("Expenses:Vacances", dec!(7)),
        ];
        let arbre = arbre(&lignes);
        let tetes: Vec<&str> = tetes(&arbre)
            .into_iter()
            .map(|i| arbre[i].compte.as_str())
            .collect();
        assert_eq!(
            tetes,
            vec!["Assets", "Assets:Epargne", "Expenses", "Expenses:Vacances"],
            "LivretA couverte par son ancêtre budgété ; les racines sont des têtes"
        );
    }

    #[test]
    fn rollover_disponible_cumule() {
        let mut ligne = ligne("Expenses:Test", dec!(100));
        ligne.cells[0].actual = dec!(60); // +40
        ligne.cells[1].actual = dec!(130); // −30 → 10
        ligne.cells[2].actual = dec!(150); // −50 → −40 (report négatif)
        let noeuds = arbre(&[ligne]);
        let noeud = &noeuds[0];
        assert_eq!(disponible(noeud, 0), dec!(40));
        assert_eq!(disponible(noeud, 1), dec!(10));
        assert_eq!(disponible(noeud, 2), dec!(-40));
        // Report sortant = disponible de décembre (budget 12×100 − réel 340).
        assert_eq!(rollover_de(noeud).carry_out(), dec!(860.00));
    }

    #[test]
    fn note_epargne_sur_les_comptes_plafonnes() {
        assert!(note_epargne("Assets:Epargne:LivretA").is_some());
        assert!(note_epargne("Expenses:Vacances").is_none());
    }

    #[test]
    fn total_mois_somme_les_totaux_une_fois() {
        let totaux = vec![vec![
            household_budget::ColumnTotal {
                bucket: "assets".to_string(),
                currency: "EUR".to_string(),
                budgeted: dec!(300),
                actual: dec!(300),
            },
            household_budget::ColumnTotal {
                bucket: "expenses".to_string(),
                currency: "EUR".to_string(),
                budgeted: dec!(1000),
                actual: dec!(500),
            },
            // Autre devise : ignorée (EUR seul est sommé).
            household_budget::ColumnTotal {
                bucket: "expenses".to_string(),
                currency: "USD".to_string(),
                budgeted: dec!(99),
                actual: dec!(99),
            },
        ]];
        assert_eq!(total_mois(&totaux, 0), dec!(1300));
        assert_eq!(total_mois(&totaux, 1), Decimal::ZERO);
    }
}

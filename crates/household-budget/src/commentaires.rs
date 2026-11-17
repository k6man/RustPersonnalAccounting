//! Commentaires de lignes budgétaires, **dans le ledger** : une ligne
//! `; note` immédiatement au-dessus d'une directive est son commentaire.
//!
//! Convention (décision maîtresse d'ouvrage, 2026-11-17) :
//! - le commentaire vit dans `budgets/AAAA.beancount`, pas dans la config —
//!   il est versionné avec le ledger et lisible partout (cf. ADR-004) ;
//! - une ligne de grille = un compte : le commentaire s'attache à la
//!   **première** directive du compte dans le fichier (le mois le plus
//!   ancien) ;
//! - une ligne de commentaire qui n'est pas directement au-dessus d'une
//!   directive (blanc ou `#` entre les deux) n'attache à rien ;
//! - une seule ligne de commentaire par directive est écrite par
//!   [`crate::writer::set_comment`] ; la lecture joint les lignes
//!   contiguës (tolérant aux fichiers édités à la main).

use std::collections::BTreeMap;

/// Extrait les commentaires d'un texte `budgets/AAAA.beancount` :
/// compte → commentaire. Tolérant aux espaces ; les lignes `#` (en-tête de
/// fichier) n'attachent à rien.
#[must_use]
pub fn commentaires(texte: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    // Commentaires contigus vus avant la directive courante.
    let mut en_attente: Vec<String> = Vec::new();

    for ligne in texte.lines() {
        let rognée = ligne.trim();
        if rognée.is_empty() || rognée.starts_with('#') {
            // Séparateur ou en-tête : les commentaires en attente ne
            // s'attachent à rien.
            en_attente.clear();
            continue;
        }
        if let Some(note) = rognée.strip_prefix(';') {
            en_attente.push(note.trim().to_string());
            continue;
        }
        // Ligne de directive ? Un commentaire `;` en fin de ligne est
        // ignoré (convention : le commentaire vit SUR sa propre ligne).
        if rognée.contains("custom \"budget\"")
            && let Some(compte) = compte_de_directive(rognée)
            && !en_attente.is_empty()
            && !out.contains_key(&compte)
        {
            out.insert(compte, en_attente.join(" "));
        }
        en_attente.clear();
    }
    out
}

/// Compte d'une ligne de directive budget : tokens `date custom "budget"
/// <compte> …` → 4ᵉ token.
fn compte_de_directive(ligne: &str) -> Option<String> {
    ligne.split_whitespace().nth(3).map(str::to_string)
}

/// Nettoie un commentaire saisi : une seule ligne, `;` de tête retiré
/// (l'écriture le rajoute), blancs rognés.
#[must_use]
pub fn nettoie(texte: &str) -> String {
    let une_ligne = texte
        .split('\n')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    une_ligne.trim().trim_start_matches(';').trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commentaire_au_dessus_de_la_directive() {
        let texte = concat!(
            "# En-tête\n",
            "\n",
            "; Courses hebdo du foyer\n",
            "2026-01-01 custom \"budget\" Expenses:Alimentation:Courses \"monthly\" 900.00 EUR\n",
            "2026-02-01 custom \"budget\" Expenses:Alimentation:Courses \"monthly\" 900.00 EUR\n",
        );
        let map = commentaires(texte);
        assert_eq!(
            map.get("Expenses:Alimentation:Courses").map(String::as_str),
            Some("Courses hebdo du foyer")
        );
    }

    #[test]
    fn separateur_vide_casse_le_rattachement() {
        let texte = concat!(
            "; note libre, pas collée\n",
            "\n",
            "2026-01-01 custom \"budget\" Expenses:Logement:Loyer \"monthly\" 1050.00 EUR\n",
        );
        assert!(commentaires(texte).is_empty(), "un blanc sépare");
    }

    #[test]
    fn premier_compte_gagne_et_lignes_contigues_jointes() {
        let texte = concat!(
            "; première ligne\n",
            "; seconde ligne\n",
            "2026-01-01 custom \"budget\" Assets:Epargne:LivretA \"monthly\" 100.00 EUR\n",
            "; autre note\n",
            "2026-01-01 custom \"budget\" Assets:Epargne:LDD \"monthly\" 100.00 EUR\n",
        );
        let map = commentaires(texte);
        assert_eq!(
            map.get("Assets:Epargne:LivretA").map(String::as_str),
            Some("première ligne seconde ligne")
        );
        assert_eq!(
            map.get("Assets:Epargne:LDD").map(String::as_str),
            Some("autre note")
        );
    }

    #[test]
    fn nettoie_rogne_et_retire_le_point_virgule() {
        assert_eq!(
            nettoie("  note \n sur deux lignes "),
            "note sur deux lignes"
        );
        assert_eq!(nettoie("; déjà préfixé"), "déjà préfixé");
        assert_eq!(nettoie("   "), "");
    }
}

//! Filtres du journal et pagination (pur, testé — aucune dépendance egui).

use rust_decimal::Decimal;
use rustledger_core::NaiveDate;

use crate::viewmodel::LigneJournal;

/// Filtres du journal. Les chaînes viennent de la saisie utilisateur ;
/// les comparaisons sont insensibles à la casse.
#[derive(Debug, Clone, Default)]
pub struct JournalFilter {
    /// Recherche plein texte (tiers + libellé).
    pub texte: String,
    /// Préfixe de compte (n'importe quelle ligne de l'écriture).
    pub compte: String,
    /// Tag exact (sans `#`).
    pub tag: String,
    /// Bornes de dates incluses.
    pub date_min: Option<NaiveDate>,
    pub date_max: Option<NaiveDate>,
    /// Bornes de montant (valeur absolue principale) incluses.
    pub montant_min: Option<Decimal>,
    pub montant_max: Option<Decimal>,
}

impl JournalFilter {
    /// `true` si au moins un filtre est actif (affichage de l'état).
    #[must_use]
    pub fn actif(&self) -> bool {
        !self.texte.trim().is_empty()
            || !self.compte.trim().is_empty()
            || !self.tag.trim().is_empty()
            || self.date_min.is_some()
            || self.date_max.is_some()
            || self.montant_min.is_some()
            || self.montant_max.is_some()
    }

    /// L'écriture passe-t-elle tous les filtres actifs ?
    #[must_use]
    pub fn correspond(&self, ligne: &LigneJournal) -> bool {
        let texte = self.texte.trim().to_lowercase();
        if !texte.is_empty() {
            let dans_tiers = ligne
                .tiers
                .as_ref()
                .is_some_and(|t| t.to_lowercase().contains(&texte));
            let dans_libelle = ligne.libelle.to_lowercase().contains(&texte);
            if !dans_tiers && !dans_libelle {
                return false;
            }
        }

        let compte = self.compte.trim().to_lowercase();
        if !compte.is_empty()
            && !ligne
                .lignes_posting
                .iter()
                .any(|p| p.compte.to_lowercase().starts_with(&compte))
        {
            return false;
        }

        let tag = self.tag.trim().trim_start_matches('#');
        if !tag.is_empty() && !ligne.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            return false;
        }

        if let Some(min) = self.date_min
            && ligne.date < min
        {
            return false;
        }
        if let Some(max) = self.date_max
            && ligne.date > max
        {
            return false;
        }

        if self.montant_min.is_some() || self.montant_max.is_some() {
            let Some(montant) = ligne.montant else {
                return false;
            };
            if let Some(min) = self.montant_min
                && montant < min
            {
                return false;
            }
            if let Some(max) = self.montant_max
                && montant > max
            {
                return false;
            }
        }

        true
    }
}

/// Pagination du tableau journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    /// Page courante, à partir de 0.
    pub page: usize,
    pub par_page: usize,
}

impl Pagination {
    /// Nombre de pages (≥ 1, même pour un journal vide).
    #[must_use]
    pub fn pages(&self, total: usize) -> usize {
        ((total / self.par_page) + usize::from(!total.is_multiple_of(self.par_page))).max(1)
    }

    /// Intervalle [début, fin) affiché, borné au total réel.
    #[must_use]
    pub fn tranche(&self, total: usize) -> (usize, usize) {
        let pages = self.pages(total);
        let page = self.page.min(pages.saturating_sub(1));
        let debut = page * self.par_page;
        let fin = (debut + self.par_page).min(total);
        (debut, fin)
    }

    /// Se cale sur la dernière page si la pagination se resserre.
    pub fn borner(&mut self, total: usize) {
        self.page = self.page.min(self.pages(total).saturating_sub(1));
    }
}

/// Parse une date ISO (`AAAA-MM-JJ`) ; chaîne vide → `None` (filtre off).
#[must_use]
pub fn date_optionnelle(texte: &str) -> Option<NaiveDate> {
    let texte = texte.trim();
    if texte.is_empty() {
        return None;
    }
    texte.parse().ok()
}

/// Parse un montant décimal ; chaîne vide → `None` (filtre off).
#[must_use]
pub fn montant_optionnel(texte: &str) -> Option<Decimal> {
    let texte = texte.trim().replace(',', ".");
    if texte.is_empty() {
        return None;
    }
    texte.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne_exemple() -> LigneJournal {
        LigneJournal {
            index: 7,
            date: "2026-03-15".parse().unwrap(),
            flag: '*',
            tiers: Some("Carrefour City".to_string()),
            libelle: "Courses de la semaine".to_string(),
            lignes_posting: vec![
                crate::viewmodel::PostingVue {
                    compte: "Expenses:Alimentation:Courses".to_string(),
                    montant: Some(("38.42".parse().unwrap(), "EUR".to_string())),
                },
                crate::viewmodel::PostingVue {
                    compte: "Assets:Banque:CompteCourant".to_string(),
                    montant: Some(("-38.42".parse().unwrap(), "EUR".to_string())),
                },
            ],
            categories: "Expenses:Alimentation:Courses".to_string(),
            tags: vec!["courses".to_string()],
            liens: vec![],
            fichier: "ledger/transactions/2026/03.beancount".to_string(),
            ligne: 42,
            montant: Some("38.42".parse().unwrap()),
        }
    }

    #[test]
    fn filtre_inactif_passe_tout() {
        assert!(JournalFilter::default().correspond(&ligne_exemple()));
        assert!(!JournalFilter::default().actif());
    }

    #[test]
    fn filtre_texte_insensible_a_la_casse() {
        let filtre = |texte: &str| JournalFilter {
            texte: texte.to_string(),
            ..JournalFilter::default()
        };
        assert!(filtre("CARREFOUR").correspond(&ligne_exemple()));
        assert!(filtre(" courses").correspond(&ligne_exemple()));
        assert!(!filtre("scarabée").correspond(&ligne_exemple()));
    }

    #[test]
    fn filtre_compte_prefixe_et_tag() {
        let filtre = |compte: &str, tag: &str| JournalFilter {
            compte: compte.to_string(),
            tag: tag.to_string(),
            ..JournalFilter::default()
        };
        assert!(filtre("Expenses:Alimentation", "").correspond(&ligne_exemple()));
        assert!(filtre("", "#Courses").correspond(&ligne_exemple()));
        assert!(!filtre("", "vacances").correspond(&ligne_exemple()));
    }

    #[test]
    fn filtre_dates_et_montants() {
        let filtre = |min: &str, max: &str, m_min: &str, m_max: &str| JournalFilter {
            date_min: date_optionnelle(min),
            date_max: date_optionnelle(max),
            montant_min: montant_optionnel(m_min),
            montant_max: montant_optionnel(m_max),
            ..JournalFilter::default()
        };
        assert!(filtre("2026-03-01", "2026-03-31", "30", "50,00").correspond(&ligne_exemple()));
        assert!(!filtre("2026-03-01", "2026-03-31", "50", "").correspond(&ligne_exemple()));
        assert!(!filtre("2026-04-01", "", "", "").correspond(&ligne_exemple()));
    }

    #[test]
    fn pagination_bornes_et_pages() {
        let mut pagination = Pagination {
            page: 0,
            par_page: 100,
        };
        assert_eq!(pagination.pages(0), 1);
        assert_eq!(pagination.pages(250), 3);
        assert_eq!(pagination.tranche(250), (0, 100));
        pagination.page = 2;
        assert_eq!(pagination.tranche(250), (200, 250));
        pagination.page = 9;
        pagination.borner(250);
        assert_eq!(pagination.page, 2, "reborde sur la dernière page");
    }

    #[test]
    fn parsers_souplement_tapes() {
        assert_eq!(date_optionnelle(""), None);
        assert_eq!(
            date_optionnelle("2026-03-15"),
            Some("2026-03-15".parse().unwrap())
        );
        assert_eq!(date_optionnelle("pas une date"), None);
        assert_eq!(montant_optionnel(""), None);
        assert_eq!(montant_optionnel("38,42"), Some("38.42".parse().unwrap()));
        assert_eq!(montant_optionnel("abc"), None);
    }
}

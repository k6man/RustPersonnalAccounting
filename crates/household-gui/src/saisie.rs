//! Brouillon de saisie d'une transaction (pur, testé — aucune dépendance
//! egui).
//!
//! Choix documentés :
//! - **Validation en deux temps** : ce module valide la *forme* (date,
//!   libellé, montants, équilibre) pour l'avis immédiat de l'utilisateur ;
//!   la validation **autorisée** reste la pré-vérification du writer
//!   (`household_core::writer`) qui rejoue le ledger complet sur une copie
//!   scratch — un compte fermé, une devise interdite ou une assertion
//!   cassée refusent l'écriture quoi qu'il arrive.
//! - **Ligne d'équilibrage explicite** : la contrepartie calculée est
//!   *écrite* avec son montant (pas de posting élidé dans le fichier) —
//!   fichiers déterministes, zéro recours à l'interpolation du moteur.
//! - Devise par défaut EUR ; saisie possible « 10 USD ».

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use rustledger_core::{Amount, NaiveDate, Posting, Transaction};

use crate::viewmodel::{InfoCompte, LigneJournal};

/// Une ligne de posting du brouillon.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LigneBrouillon {
    pub compte: String,
    /// Texte du montant ; **vide = ligne d'équilibrage** (une seule permise).
    pub montant: String,
}

/// Brouillon complet d'une transaction — l'état de l'écran Saisie.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Brouillon {
    /// Date ISO (saisie libre, parsée à la volée).
    pub date: String,
    /// `'*'` pointée ou `'!'` en attente.
    pub flag: char,
    pub tiers: String,
    pub libelle: String,
    /// Saisie libre « #courses #marche » (ou « courses marche »).
    pub tags: String,
    /// Saisie libre « ^course-042 ».
    pub liens: String,
    pub lignes: Vec<LigneBrouillon>,
}

/// Empreinte d'identification d'une transaction existante : l'index d'entrée
/// peut avoir bougé si le ledger a changé depuis le chargement de la GUI ;
/// l'empreinte (date, tiers, libellé) permet de la relocaliser sûrement.
#[derive(Debug, Clone, PartialEq)]
pub struct Empreinte {
    pub date: NaiveDate,
    pub tiers: Option<String>,
    pub libelle: String,
}

impl Empreinte {
    #[must_use]
    pub fn de_ligne(ligne: &LigneJournal) -> Self {
        Self {
            date: ligne.date,
            tiers: ligne.tiers.clone(),
            libelle: ligne.libelle.clone(),
        }
    }
}

impl Brouillon {
    /// Un brouillon vierge daté du jour (deux lignes de postings prêtes).
    #[must_use]
    pub fn vierge(aujourdhui: NaiveDate) -> Self {
        Self {
            date: aujourdhui.to_string(),
            flag: '*',
            lignes: vec![LigneBrouillon::default(), LigneBrouillon::default()],
            ..Self::default()
        }
    }

    /// Toutes les erreurs de forme ; liste vide = enregistrable.
    #[must_use]
    pub fn erreurs(&self) -> Vec<String> {
        let mut erreurs = Vec::new();
        if date_optionnelle(&self.date).is_none() {
            erreurs.push("date invalide (attendu AAAA-MM-JJ)".to_string());
        }
        if self.libelle.trim().is_empty() {
            erreurs.push("libellé obligatoire".to_string());
        }
        if self.flag != '*' && self.flag != '!' {
            erreurs.push(format!("flag invalide : {:?}", self.flag));
        }

        // Lignes : montant parsable, compte non vide si montant présent.
        let mut elidees = 0;
        let mut reelles = 0;
        let mut residus: BTreeMap<String, Decimal> = BTreeMap::new();
        for (numero, ligne) in self.lignes.iter().enumerate() {
            let compte = ligne.compte.trim();
            let montant = ligne.montant.trim();
            if compte.is_empty() && montant.is_empty() {
                continue; // ligne entièrement vide : ignorée
            }
            if compte.is_empty() {
                erreurs.push(format!("ligne {} : compte manquant", numero + 1));
                continue;
            }
            if montant.is_empty() {
                elidees += 1;
                continue;
            }
            match parse_montant(montant) {
                Ok((valeur, devise)) => {
                    reelles += 1;
                    *residus.entry(devise).or_default() += valeur;
                }
                Err(e) => erreurs.push(format!("ligne {} : {e}", numero + 1)),
            }
        }
        if reelles == 0 && elidees == 0 {
            erreurs.push("au moins une ligne de posting".to_string());
        }
        if elidees > 1 {
            erreurs.push("une seule ligne peut être laissée vide (équilibrage)".to_string());
        }
        // Équilibre : par devise ; une ligne élidée ne peut couvrir qu'une
        // seule devise résiduelle.
        let residus_non_nuls: Vec<(&String, &Decimal)> =
            residus.iter().filter(|(_, v)| !v.is_zero()).collect();
        if elidees == 0 && !residus_non_nuls.is_empty() {
            for (devise, reste) in residus_non_nuls {
                erreurs.push(format!(
                    "déséquilibre : {reste} {devise} (sans ligne d'équilibrage)"
                ));
            }
        } else if elidees == 1 && residus_non_nuls.len() > 1 {
            erreurs.push(
                "devises mélangées : l'équilibrage automatique ne peut pas couvrir \
                 plusieurs devises"
                    .to_string(),
            );
        }
        erreurs
    }

    /// Résidus par devise (lignes complètes seulement) — l'indicateur « reste
    /// à répartir » de l'écran.
    #[must_use]
    pub fn residus(&self) -> Vec<(String, Decimal)> {
        let mut residus: BTreeMap<String, Decimal> = BTreeMap::new();
        for ligne in &self.lignes {
            if let Ok((valeur, devise)) = parse_montant(ligne.montant.trim()) {
                *residus.entry(devise).or_default() += valeur;
            }
        }
        residus
            .into_iter()
            .filter(|(_, valeur)| !valeur.is_zero())
            .collect()
    }

    /// Conversion en [`Transaction`] complète (contrepartie explicite).
    ///
    /// # Erreurs
    /// La liste des erreurs de [`Self::erreurs`] si le brouillon est
    /// invalide.
    pub fn vers_transaction(&self) -> Result<Transaction, Vec<String>> {
        let erreurs = self.erreurs();
        if !erreurs.is_empty() {
            return Err(erreurs);
        }
        let date: NaiveDate = self
            .date
            .trim()
            .parse()
            .expect("date validée par erreurs()");

        let mut postings = Vec::new();
        // Première passe : lignes complètes, cumul par devise.
        let mut residus: BTreeMap<String, Decimal> = BTreeMap::new();
        let mut elidees: Vec<&LigneBrouillon> = Vec::new();
        for ligne in &self.lignes {
            let compte = ligne.compte.trim();
            let montant = ligne.montant.trim();
            if compte.is_empty() && montant.is_empty() {
                continue;
            }
            if montant.is_empty() {
                elidees.push(ligne);
                continue;
            }
            let (valeur, devise) = parse_montant(montant).expect("montant validé");
            postings.push(Posting::new(compte, Amount::new(valeur, devise.clone())));
            *residus.entry(devise).or_default() += valeur;
        }
        // Deuxième passe : la ligne d'équilibrage reçoit l'opposé du résidu
        // (écrit explicitement — voir la doc du module).
        for ligne in elidees {
            let compte = ligne.compte.trim();
            let (devise, reste) = residus
                .iter()
                .find(|(_, valeur)| !valeur.is_zero())
                .map_or(("EUR".to_string(), Decimal::ZERO), |(devise, valeur)| {
                    (devise.clone(), *valeur)
                });
            let contrepartie = -reste;
            *residus.entry(devise.clone()).or_default() += contrepartie;
            postings.push(Posting::new(compte, Amount::new(contrepartie, devise)));
        }

        let mut transaction = Transaction::new(date, self.libelle.trim()).with_flag(self.flag);
        if !self.tiers.trim().is_empty() {
            transaction = transaction.with_payee(self.tiers.trim());
        }
        for posting in postings {
            transaction = transaction.with_synthesized_posting(posting);
        }
        transaction.tags = parse_symboles(&self.tags, '#')
            .into_iter()
            .map(rustledger_core::Tag::from)
            .collect();
        transaction.links = parse_symboles(&self.liens, '^')
            .into_iter()
            .map(rustledger_core::Link::from)
            .collect();
        Ok(transaction)
    }
}

/// Pré-remplit le brouillon depuis une ligne du journal (mode édition).
/// Les postings élidés reviennent avec un montant vide (ligne d'équilibrage).
#[must_use]
pub fn depuis_ligne(ligne: &LigneJournal) -> Brouillon {
    let lignes = ligne
        .lignes_posting
        .iter()
        .map(|posting| LigneBrouillon {
            compte: posting.compte.clone(),
            montant: posting
                .montant
                .as_ref()
                .map(|(valeur, _)| valeur.to_string())
                .unwrap_or_default(),
        })
        .collect();
    Brouillon {
        date: ligne.date.to_string(),
        flag: ligne.flag,
        tiers: ligne.tiers.clone().unwrap_or_default(),
        libelle: ligne.libelle.clone(),
        tags: ligne
            .tags
            .iter()
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<_>>()
            .join(" "),
        liens: ligne
            .liens
            .iter()
            .map(|lien| format!("^{lien}"))
            .collect::<Vec<_>>()
            .join(" "),
        lignes,
    }
}

/// Parse un montant saisi : `"38.42"` → (38.42, EUR), `"10 USD"` →
/// (10, USD), virgule décimale acceptée (`"38,42"`).
///
/// # Erreurs
/// Message lisible si le texte n'est ni vide ni un montant.
pub fn parse_montant(texte: &str) -> Result<(Decimal, String), String> {
    let texte = texte.trim();
    if texte.is_empty() {
        return Err("montant vide".to_string());
    }
    // Suffixe devise éventuel (dernier token alphabétique).
    let (nombre, devise) = match texte.rsplit_once(' ') {
        Some((nombre, devise)) => (nombre.trim(), Some(devise.trim())),
        None => (texte, None),
    };
    if let Some(devise) = devise
        && (devise.is_empty() || !devise.chars().all(|c| c.is_ascii_alphabetic()))
    {
        return Err(format!("devise invalide : {devise:?}"));
    }
    let valeur: Decimal = nombre
        .replace(',', ".")
        .parse()
        .map_err(|_| format!("montant invalide : {texte:?}"))?;
    Ok((valeur, devise.unwrap_or("EUR").to_uppercase()))
}

/// Parse une liste de tags/liens saisis : séparateurs = espaces, préfixe
/// `#`/`^` facultatif (tolérance de saisie), doublons dédupliqués.
#[must_use]
pub fn parse_symboles(texte: &str, symbole: char) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in texte.split_whitespace() {
        let nom = token.trim_start_matches(symbole);
        if nom.is_empty() || out.iter().any(|existant| existant == nom) {
            continue;
        }
        out.push(nom.to_string());
    }
    out
}

/// Suggestions de comptes : ouverts à la date, filtrés par la saisie
/// (contient, insensible à la casse), ordre alphabétique, borné.
#[must_use]
pub fn completer_comptes(
    comptes: &[InfoCompte],
    saisi: &str,
    date: NaiveDate,
    limite: usize,
) -> Vec<String> {
    let filtre = saisi.trim().to_lowercase();
    let mut candidats: Vec<&str> = comptes
        .iter()
        .filter(|compte| compte.ouverture <= date)
        .filter(|compte| compte.fermeture.is_none_or(|fermeture| fermeture > date))
        .filter(|compte| filtre.is_empty() || compte.nom.to_lowercase().contains(&filtre))
        .map(|compte| compte.nom.as_str())
        .collect();
    candidats.sort_unstable();
    candidats.truncate(limite);
    candidats.into_iter().map(str::to_string).collect()
}

/// Suggestions de tiers : fréquence décroissante puis alphabétique, filtrés
/// par la saisie (contient, insensible à la casse).
#[must_use]
pub fn completer_payees(payees: &[(String, usize)], saisi: &str, limite: usize) -> Vec<String> {
    let filtre = saisi.trim().to_lowercase();
    let mut candidats: Vec<(String, usize)> = payees
        .iter()
        .filter(|(nom, _)| filtre.is_empty() || nom.to_lowercase().contains(&filtre))
        .cloned()
        .collect();
    candidats.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    candidats.truncate(limite);
    candidats.into_iter().map(|(nom, _)| nom).collect()
}

/// Date ISO optionnelle (chaîne vide = pas encore saisie).
#[must_use]
pub fn date_optionnelle(texte: &str) -> Option<NaiveDate> {
    let texte = texte.trim();
    if texte.is_empty() {
        return None;
    }
    texte.parse().ok()
}

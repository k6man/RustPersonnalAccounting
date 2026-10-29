//! View-models : l'assemblage **pur** des données affichées, fait une fois
//! par chargement dans le thread worker (jamais dans les vues).
//!
//! Aucune dépendance à egui — testable sans GUI. Les vues lisent
//! [`UiData`] (en `Arc`) et ne font que de la présentation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use rust_decimal::Decimal;
use rustledger_core::{AccountTypes, Directive, NaiveDate};

use household_budget::BudgetGrid;
use household_core::Snapshot;

/// Toutes les données dont les vues ont besoin, prêtes à afficher.
#[derive(Debug, Clone)]
pub struct UiData {
    /// Chemin du ledger chargé.
    pub main: PathBuf,
    /// Horodatage du chargement (informatif).
    pub charge_le: String,
    pub tableau_de_bord: DashboardData,
    pub journal: JournalData,
    /// Erreurs de validation du ledger (bannière).
    pub erreurs: Vec<LigneErreur>,
}

/// Une erreur de validation, prête pour la bannière.
#[derive(Debug, Clone)]
pub struct LigneErreur {
    pub code: String,
    pub message: String,
    pub phase: String,
    pub fichier: Option<String>,
    pub ligne: Option<usize>,
}

/// Données du tableau de bord.
#[derive(Debug, Clone)]
pub struct DashboardData {
    /// Soldes actuels des trois comptes clés (fin du ledger).
    pub solde_cc: Option<Decimal>,
    pub solde_livret_a: Option<Decimal>,
    pub solde_ldd: Option<Decimal>,
    /// Jauges de plafond (comptes d'épargne plafonnés).
    pub jauges: Vec<Jauge>,
    /// Patrimoine net (Σ comptes de bilan, EUR).
    pub patrimoine_net: Decimal,
    /// Budget vs réel du mois courant, trié (sur-budget d'abord).
    pub budget_mois: Vec<BudgetVsReel>,
    /// Étiquette du mois (« 2026-03 »).
    pub budget_mois_label: String,
    /// Diagnostics du modèle budgétaire.
    pub budget_avertissements: Vec<String>,
    /// Patrimoine net par fin de mois (courbe d'épargne).
    pub courbe: Vec<(String, Decimal)>,
}

/// Une jauge de plafond d'un compte d'épargne.
#[derive(Debug, Clone)]
pub struct Jauge {
    pub compte: String,
    pub plafond: Decimal,
    pub courant: Decimal,
    pub etat: EtatJauge,
}

/// État d'une jauge : < 90 %, 90–100 % (alerte), > 100 % (dépassement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatJauge {
    Ok,
    Alerte,
    Depassement,
}

impl EtatJauge {
    #[must_use]
    pub fn of(courant: Decimal, plafond: Decimal) -> Self {
        if plafond.is_zero() {
            return if courant > Decimal::ZERO {
                Self::Depassement
            } else {
                Self::Ok
            };
        }
        let ratio = courant / plafond;
        if ratio > Decimal::ONE {
            Self::Depassement
        } else if ratio >= Decimal::from(9) / Decimal::from(10) {
            Self::Alerte
        } else {
            Self::Ok
        }
    }

    /// Fraction remplie (0..1), plafonnée à 1 pour l'affichage.
    #[must_use]
    pub fn fraction(&self, courant: Decimal, plafond: Decimal) -> f64 {
        if plafond.is_zero() {
            return if courant.is_zero() { 0.0 } else { 1.0 };
        }
        (f64::try_from(courant / plafond).unwrap_or(1.0)).clamp(0.0, 1.0)
    }
}

/// Une ligne « budget vs réel » du mois.
#[derive(Debug, Clone)]
pub struct BudgetVsReel {
    pub compte: String,
    pub budgete: Decimal,
    pub reel: Decimal,
}

impl BudgetVsReel {
    /// Écart budget − réel (positif : sous le budget).
    #[must_use]
    pub fn ecart(&self) -> Decimal {
        self.budgete - self.reel
    }

    /// Fraction consommée (0..∞), `None` si budget nul.
    #[must_use]
    pub fn consomme(&self) -> Option<f64> {
        if self.budgete.is_zero() {
            None
        } else {
            Some(f64::try_from(self.reel / self.budgete).unwrap_or(f64::INFINITY))
        }
    }
}

/// Le journal complet (toutes les transactions, ordre du ledger).
#[derive(Debug, Clone)]
pub struct JournalData {
    pub lignes: Vec<LigneJournal>,
}

/// Une transaction du journal, prête à afficher.
#[derive(Debug, Clone)]
pub struct LigneJournal {
    /// Index d'entrée dans le snapshot (pour l'édition au jalon 5).
    pub index: usize,
    pub date: NaiveDate,
    pub flag: char,
    pub tiers: Option<String>,
    pub libelle: String,
    pub lignes_posting: Vec<PostingVue>,
    /// Comptes de catégorisation (hors bilan), joints par des virgules.
    pub categories: String,
    pub tags: Vec<String>,
    pub liens: Vec<String>,
    pub fichier: String,
    pub ligne: usize,
    /// Montant principal = plus grande valeur absolue des postings complets
    /// (pour un tri/filtre « montant » ; `None` si tout est élidé).
    pub montant: Option<Decimal>,
}

/// Un posting (ligne d'écriture) de la vue journal.
#[derive(Debug, Clone)]
pub struct PostingVue {
    pub compte: String,
    /// `None` = montant élidé (équilibrage automatique).
    pub montant: Option<(Decimal, String)>,
}

/// Assemble tout le view-model depuis un snapshot.
///
/// `aujourdhui` est injecté (au lieu de lire l'horloge ici) pour la
/// testabilité ; `main.rs` passe la date du jour.
pub fn build(snapshot: &Snapshot, aujourdhui: NaiveDate) -> Result<UiData, String> {
    let erreurs: Vec<LigneErreur> = snapshot
        .errors()
        .iter()
        .map(|e| LigneErreur {
            code: e.code.clone(),
            message: e.message.clone(),
            phase: e.phase.clone(),
            fichier: e.location.as_ref().map(|l| l.file.display().to_string()),
            ligne: e.location.as_ref().map(|l| l.line),
        })
        .collect();

    let types = AccountTypes {
        assets: snapshot.ledger().options.name_assets.clone(),
        liabilities: snapshot.ledger().options.name_liabilities.clone(),
        equity: snapshot.ledger().options.name_equity.clone(),
        income: snapshot.ledger().options.name_income.clone(),
        expenses: snapshot.ledger().options.name_expenses.clone(),
    };

    let soldes = soldes_par_compte(snapshot)?;
    let jauges = snapshot
        .ceilings()
        .into_iter()
        .map(|(compte, plafond)| {
            let courant = solde_eur(&soldes, &compte);
            Jauge {
                compte,
                plafond: plafond.amount,
                courant,
                etat: EtatJauge::of(courant, plafond.amount),
            }
        })
        .collect();

    let directives: Vec<Directive> = snapshot.entries().iter().map(|e| e.value.clone()).collect();
    let annee = aujourdhui.year();
    let mois = u8::try_from(aujourdhui.month()).unwrap_or(1);
    let grille = BudgetGrid::build(&directives, &types, annee);
    let mut budget_mois: Vec<BudgetVsReel> = grille
        .rows
        .iter()
        .map(|ligne| BudgetVsReel {
            compte: ligne.account.clone(),
            budgete: ligne.cells[usize::from(mois) - 1].budgeted,
            reel: ligne.cells[usize::from(mois) - 1].actual,
        })
        .collect();
    // Sur-budget d'abord, puis écart croissant (les plus tendus en tête).
    budget_mois.sort_unstable_by(|a, b| {
        let depasse_a = a.reel > a.budgete;
        let depasse_b = b.reel > b.budgete;
        depasse_b
            .cmp(&depasse_a)
            .then_with(|| a.ecart().cmp(&b.ecart()))
    });

    let courbe = courbe_patrimoine(snapshot, &types);

    Ok(UiData {
        main: snapshot.main.clone(),
        charge_le: horodatage(),
        tableau_de_bord: DashboardData {
            solde_cc: solde_opt(&soldes, "Assets:Banque:CompteCourant"),
            solde_livret_a: solde_opt(&soldes, "Assets:Epargne:LivretA"),
            solde_ldd: solde_opt(&soldes, "Assets:Epargne:LDD"),
            jauges,
            patrimoine_net: patrimoine_net(&soldes, &types),
            budget_mois,
            budget_mois_label: format!("{annee}-{mois:02}"),
            budget_avertissements: grille.warnings,
            courbe,
        },
        journal: JournalData {
            lignes: lignes_journal(snapshot)?,
        },
        erreurs,
    })
}

// ------------------------------------------------------------------ helpers ---

fn soldes_par_compte(
    snapshot: &Snapshot,
) -> Result<BTreeMap<String, BTreeMap<String, Decimal>>, String> {
    let mut out: BTreeMap<String, BTreeMap<String, Decimal>> = BTreeMap::new();
    for solde in snapshot.balances_at_end().map_err(|e| e.to_string())? {
        out.insert(solde.account, solde.amounts);
    }
    Ok(out)
}

fn solde_opt(
    soldes: &BTreeMap<String, BTreeMap<String, Decimal>>,
    compte: &str,
) -> Option<Decimal> {
    soldes
        .get(compte)
        .and_then(|devises| devises.get("EUR").copied())
}

fn solde_eur(soldes: &BTreeMap<String, BTreeMap<String, Decimal>>, compte: &str) -> Decimal {
    solde_opt(soldes, compte).unwrap_or_default()
}

/// Patrimoine net : Σ Assets − Σ Liabilities (EUR uniquement, devise unique
/// du foyer — documenté). **Equity exclu** : le compte d'ouverture
/// (`Equity:Soldes-Ouverture`) porte le capital historique et s'annulerait
/// avec les assets d'ouverture — le « net » du foyer est ce qu'il possède
/// moins ce qu'il doit, pas l'équation comptable.
fn patrimoine_net(
    soldes: &BTreeMap<String, BTreeMap<String, Decimal>>,
    types: &AccountTypes,
) -> Decimal {
    soldes
        .iter()
        .filter(|(compte, _)| est_actif_ou_passif(compte, types))
        .filter_map(|(_, devises)| devises.get("EUR"))
        .copied()
        .sum()
}

/// Assets ou Liabilities (pas Equity) — pour le patrimoine net.
fn est_actif_ou_passif(compte: &str, types: &AccountTypes) -> bool {
    matches!(
        types.kind(compte),
        Some(
            rustledger_core::AccountTypeKind::Assets
                | rustledger_core::AccountTypeKind::Liabilities
        )
    )
}

/// Courbe du patrimoine net, par fin de mois, du premier au dernier mois
/// transactionné (24 derniers points max).
fn courbe_patrimoine(snapshot: &Snapshot, types: &AccountTypes) -> Vec<(String, Decimal)> {
    let Some(dernier) = snapshot.last_transaction_date() else {
        return Vec::new();
    };
    let Some(premier) = snapshot
        .entries()
        .iter()
        .filter_map(|e| match &e.value {
            Directive::Transaction(t) => Some(t.date),
            _ => None,
        })
        .min()
    else {
        return Vec::new();
    };

    let mut points = Vec::new();
    let mut annee = premier.year();
    let mut mois = u8::try_from(premier.month()).unwrap_or(1);
    let dernier_mois = u8::try_from(dernier.month()).unwrap_or(1);
    loop {
        let fin = fin_de_mois(annee, mois);
        let valeur = snapshot
            .balances_at(fin, household_core::Through::EndOfDay)
            .ok()
            .map(|soldes| {
                soldes
                    .iter()
                    .filter(|s| est_actif_ou_passif(&s.account, types))
                    .filter_map(|s| s.amounts.get("EUR").copied())
                    .sum::<Decimal>()
            })
            .unwrap_or_default();
        points.push((format!("{annee}-{mois:02}"), valeur));
        if (annee, mois) == (dernier.year(), dernier_mois) {
            break;
        }
        // Mois suivant.
        if mois == 12 {
            annee += 1;
            mois = 1;
        } else {
            mois += 1;
        }
        if points.len() > 48 {
            break; // garde-fou (4 ans de mois)
        }
    }
    if points.len() > 24 {
        points.drain(..points.len() - 24);
    }
    points
}

/// Dernier jour du mois (gère février bissextile).
#[must_use]
pub fn fin_de_mois(annee: i16, mois: u8) -> NaiveDate {
    let jour = match mois {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if est_bissextile(i32::from(annee)) {
                29
            } else {
                28
            }
        }
    };
    rustledger_core::naive_date(i32::from(annee), u32::from(mois), jour)
        .expect("fin de mois valide")
}

/// Année bissextile grégorienne.
#[must_use]
pub fn est_bissextile(annee: i32) -> bool {
    (annee % 4 == 0 && annee % 100 != 0) || annee % 400 == 0
}

fn horodatage() -> String {
    jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string()
}

/// Les transactions du ledger, prêtes pour le journal.
fn lignes_journal(snapshot: &Snapshot) -> Result<Vec<LigneJournal>, String> {
    let types = AccountTypes {
        assets: snapshot.ledger().options.name_assets.clone(),
        liabilities: snapshot.ledger().options.name_liabilities.clone(),
        equity: snapshot.ledger().options.name_equity.clone(),
        income: snapshot.ledger().options.name_income.clone(),
        expenses: snapshot.ledger().options.name_expenses.clone(),
    };

    let mut lignes = Vec::new();
    for (index, entree) in snapshot.entries().iter().enumerate() {
        let Directive::Transaction(t) = &entree.value else {
            continue;
        };
        let mut postings = Vec::new();
        let mut montant = None::<Decimal>;
        for posting in &t.postings {
            let nombre = match &posting.value.units {
                Some(rustledger_core::IncompleteAmount::Complete(a)) => {
                    Some((a.number, a.currency.as_str().to_string()))
                }
                _ => None,
            };
            if let Some((absolu, _)) = nombre.as_ref() {
                let absolu = absolu.abs();
                if montant.is_none_or(|m| absolu > m) {
                    montant = Some(absolu);
                }
            }
            postings.push(PostingVue {
                compte: posting.value.account.as_str().to_string(),
                montant: nombre,
            });
        }
        let source = snapshot
            .source_of(index)
            .map(|s| (s.file.display().to_string(), s.line))
            .unwrap_or_else(|_| ("?".to_string(), 0));
        lignes.push(LigneJournal {
            index,
            date: t.date,
            flag: t.flag,
            tiers: t.payee.as_ref().map(|p| p.as_str().to_string()),
            libelle: t.narration.as_str().to_string(),
            lignes_posting: postings,
            categories: postings_categoriees(&t.postings, &types),
            tags: t.tags.iter().map(|tag| tag.as_str().to_string()).collect(),
            liens: t
                .links
                .iter()
                .map(|lien| lien.as_str().to_string())
                .collect(),
            fichier: source.0,
            ligne: source.1,
            montant,
        });
    }
    Ok(lignes)
}

/// Les comptes de catégorisation (hors bilan), joints par des virgules.
fn postings_categoriees(
    postings: &[rustledger_core::Spanned<rustledger_core::Posting>],
    types: &AccountTypes,
) -> String {
    postings
        .iter()
        .map(|p| p.value.account.as_str())
        .filter(|compte| !types.is_balance_sheet(compte))
        .collect::<Vec<_>>()
        .join(", ")
}

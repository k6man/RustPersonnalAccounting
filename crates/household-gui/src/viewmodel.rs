//! View-models : l'assemblage **pur** des données affichées, fait une fois
//! par chargement dans le thread worker (jamais dans les vues).
//!
//! Aucune dépendance à egui — testable sans GUI. Les vues lisent
//! [`UiData`] (en `Arc`) et ne font que de la présentation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use rust_decimal::Decimal;
use rustledger_core::{AccountTypes, Directive, IncompleteAmount, NaiveDate};

use household_budget::BudgetGrid;
use household_core::Snapshot;
use rustledger_budget::Budgets;

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
    /// Comptes (ouverts/fermés) pour l'autocomplétion de saisie.
    pub comptes: Vec<InfoCompte>,
    /// Tiers par fréquence pour l'autocomplétion de saisie.
    pub payees: Vec<(String, usize)>,
    /// Grilles budgétaires par année budgétée (éditeur de budget).
    pub grilles: BTreeMap<i16, household_budget::BudgetGrid>,
    /// Directives `custom "budget"` du ledger, triées — base du diff et du
    /// sélecteur d'années.
    pub directives_budget: Vec<DirectiveBudget>,
    /// Données de l'écran Rapports (année courante + N−1 pour comparaison).
    pub rapports: RapportsData,
    /// Données de l'écran Graphiques (répartition, cascade annuelle).
    pub graphiques: GraphesData,
}

/// Données de l'écran **Rapports**, calculées une fois par chargement :
/// agrégats par mois dans l'année, en `Decimal` (conversion `f64` réservée
/// au tracé egui_plot, jamais stockée).
#[derive(Debug, Clone, Default)]
pub struct RapportsData {
    /// Année affichée (l'année courante).
    pub annee: i16,
    /// Dépenses par catégorie (2 premiers segments) × 12 mois (0-based).
    pub depenses_categories: BTreeMap<String, [Decimal; 12]>,
    /// Dépenses totales par mois.
    pub depenses_mois: [Decimal; 12],
    /// Dépenses totales de l'année précédente (comparaison N vs N−1),
    /// `None` si aucune transaction cette année-là.
    pub depenses_annee_precedente: Option<[Decimal; 12]>,
    /// Revenus totaux par mois.
    pub revenus_mois: [Decimal; 12],
    /// Variation nette de l'épargne par mois (transferts + intérêts, signés).
    pub epargne_mois: [Decimal; 12],
}

impl RapportsData {
    /// Dépenses totales de l'année affichée.
    #[must_use]
    pub fn depenses_annee(&self) -> Decimal {
        self.depenses_mois.iter().copied().sum()
    }

    /// Revenus totaux de l'année affichée.
    #[must_use]
    pub fn revenus_annee(&self) -> Decimal {
        self.revenus_mois.iter().copied().sum()
    }

    /// Variation nette de l'épargne sur l'année.
    #[must_use]
    pub fn epargne_annee(&self) -> Decimal {
        self.epargne_mois.iter().copied().sum()
    }

    /// Taux d'épargne de l'année (variation nette / revenus), `None` si
    /// aucun revenu n'est enregistré.
    #[must_use]
    pub fn taux_epargne(&self) -> Option<Decimal> {
        let revenus = self.revenus_annee();
        (!revenus.is_zero()).then(|| self.epargne_annee() / revenus)
    }
}

/// Une directive `custom "budget"` telle qu'écrite dans le ledger.
#[derive(Debug, Clone)]
pub struct DirectiveBudget {
    pub date: NaiveDate,
    pub compte: String,
    pub intervalle: String,
    pub montant: Decimal,
}

/// Un compte du plan, avec ses bornes d'existence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoCompte {
    pub nom: String,
    pub ouverture: NaiveDate,
    /// Date de fermeture, s'il est fermé.
    pub fermeture: Option<NaiveDate>,
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
    /// Total budgété du mois courant (postes EUR).
    pub total_budgete: Decimal,
    /// Total réel dépensé du mois courant (postes EUR).
    pub total_reel: Decimal,
    /// Reste à dépenser : total budgété − total réel (positif : sous le
    /// budget, négatif : le réel a déjà dépassé le budget global).
    pub reste_a_depenser: Decimal,
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

/// Données de l'écran « Graphiques » : répartition des dépenses (année /
/// mois courant) et cascade annuelle. La partie budget vs réel est relue
/// depuis [`DashboardData::budget_mois`] (même tri « plus tendus en tête »
/// que la carte du tableau de bord — cohérence des deux écrans).
#[derive(Debug, Clone)]
pub struct GraphesData {
    pub annee: i16,
    /// Mois courant (1..=12), base des cellules de répartition mensuelle.
    pub mois: u8,
    /// Top catégories décroissant + agrégat « Autres (N postes) » — année.
    pub repartition_annee: Vec<(String, Decimal)>,
    /// Pareil pour le mois courant.
    pub repartition_mois: Vec<(String, Decimal)>,
    /// Cascade annuelle de l'exercice.
    pub cascade: CascadeAnnee,
}

/// Cascade de l'année : **identité comptable** Revenus − Dépenses = Δ
/// patrimoine (patrimoine fin N−1 contre fin de ledger ; l'Équité est stable
/// en cours d'exercice, cf. ADR-003). L'épargne nette n'est pas additive —
/// les intérêts sont déjà comptés dans les revenus et les transferts
/// Assets→Assets s'annulent — elle est stockée ici comme barre
/// d'**information** séparée (décision du 2026-11-01, pas de double-compte).
#[derive(Debug, Clone)]
pub struct CascadeAnnee {
    /// Revenus de l'année (totaux du view-model Rapports).
    pub revenus: Decimal,
    /// Dépenses de l'année.
    pub depenses: Decimal,
    /// Δ patrimoine net : fin de ledger contre fin N−1 (Σ Assets − Σ
    /// Liabilities, EUR, Equity exclu).
    pub variation_patrimoine: Decimal,
    /// Variation nette de l'épargne (née Rapports) — hors identité.
    pub epargne: Decimal,
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

    let directives: Vec<Directive> = snapshot.entries().iter().map(|e| e.value.clone()).collect();

    // Grilles budgétaires par année budgétée (éditeur + carte du mois).
    let budgets = Budgets::from_directives(&directives);
    let mut annees: Vec<i16> = budgets.entries().map(|entree| entree.from.year()).collect();
    annees.sort_unstable();
    annees.dedup();
    let mut grilles = BTreeMap::new();
    for annee_budget in &annees {
        grilles.insert(
            *annee_budget,
            BudgetGrid::build(&directives, &types, *annee_budget),
        );
    }

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

    let annee = aujourdhui.year();
    let mois = u8::try_from(aujourdhui.month()).unwrap_or(1);
    let mut budget_mois: Vec<BudgetVsReel> = grilles
        .get(&annee)
        .map(|grille| {
            grille
                .rows
                .iter()
                .map(|ligne| BudgetVsReel {
                    compte: ligne.account.clone(),
                    budgete: ligne.cells[usize::from(mois) - 1].budgeted,
                    reel: ligne.cells[usize::from(mois) - 1].actual,
                })
                .collect()
        })
        .unwrap_or_default();
    // Sur-budget d'abord, puis écart croissant (les plus tendus en tête).
    budget_mois.sort_unstable_by(|a, b| {
        let depasse_a = a.reel > a.budgete;
        let depasse_b = b.reel > b.budgete;
        depasse_b
            .cmp(&depasse_a)
            .then_with(|| a.ecart().cmp(&b.ecart()))
    });

    // Totaux du mois courant. Les budgets du foyer sont en EUR ; les
    // postes d'une autre devise (s'il y en a un jour) sont exclus du
    // solde prévisionnel : ils ne pèsent pas sur le compte courant.
    let (total_budgete, total_reel) = grilles
        .get(&annee)
        .map(|grille| {
            let cell = usize::from(mois) - 1;
            grille
                .rows
                .iter()
                .filter(|ligne| ligne.currency == "EUR")
                .fold((Decimal::ZERO, Decimal::ZERO), |(b, r), ligne| {
                    (b + ligne.cells[cell].budgeted, r + ligne.cells[cell].actual)
                })
        })
        .unwrap_or((Decimal::ZERO, Decimal::ZERO));
    let reste_a_depenser = total_budgete - total_reel;

    let courbe = courbe_patrimoine(snapshot, &types);

    // Rapports : année courante + comparaison N−1.
    let annee_courante = aujourdhui.year();
    let rapports = rapports(&directives, &types, annee_courante);
    let waterfall = cascade(snapshot, &types, &soldes, &rapports, annee);
    let graphiques = GraphesData {
        annee,
        mois,
        repartition_annee: repartition(&rapports.depenses_categories, None, TOP_CATEGORIES),
        repartition_mois: repartition(
            &rapports.depenses_categories,
            Some(usize::from(mois) - 1),
            TOP_CATEGORIES,
        ),
        cascade: waterfall,
    };
    let budget_avertissements = grilles
        .get(&annee)
        .map(|grille| grille.warnings.clone())
        .unwrap_or_default();

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
            total_budgete,
            total_reel,
            reste_a_depenser,
            budget_avertissements,
            courbe,
        },
        journal: JournalData {
            lignes: lignes_journal(snapshot)?,
        },
        erreurs,
        comptes: comptes_du_plan(snapshot),
        payees: snapshot.payees(),
        grilles,
        directives_budget: directives_budget(&directives),
        rapports,
        graphiques,
    })
}

/// Nombre de catégories affichées dans la répartition (le reste est
/// agrégé en « Autres »).
const TOP_CATEGORIES: usize = 8;

/// Catégorie d'agrégation d'un compte : ses 2 premiers segments
/// (`Expenses:Enfants`), ou le compte entier s'il en a moins.
fn categorie_de(compte: &str) -> String {
    let segments: Vec<&str> = compte.split(':').collect();
    if segments.len() >= 3 {
        segments[..2].join(":")
    } else {
        compte.to_string()
    }
}

/// Agrégats de l'écran Rapports pour une année : parcours des transactions,
/// classifications par préfixe (dépenses / revenus / variation d'épargne),
/// EUR seulement.
///
/// Conventions du projet : épargne = comptes `Assets:Epargne:*` (variations
/// signées — transferts ET intérêts) ; dépense/revenu = valeur absolue des
/// postings `Expenses:*` / `Income:*`.
#[must_use]
pub fn rapports(directives: &[Directive], types: &AccountTypes, annee: i16) -> RapportsData {
    let mut out = RapportsData {
        annee,
        ..RapportsData::default()
    };
    let epargne_prefixe = format!("{}:Epargne", types.assets);

    for directive in directives {
        let Directive::Transaction(t) = directive else {
            continue;
        };
        if t.date.year() != annee {
            continue;
        }
        let mois_zero = usize::try_from(i16::from(t.date.month()) - 1).unwrap_or(0);
        for posting in &t.postings {
            let Some(rustledger_core::IncompleteAmount::Complete(a)) = &posting.value.units else {
                continue;
            };
            if a.currency.as_str() != "EUR" {
                continue; // foyer euro-centrique
            }
            let compte = posting.value.account.as_str();
            if compte.starts_with(&types.expenses) {
                let montant = a.number.abs();
                out.depenses_mois[mois_zero] += montant;
                let cat = categorie_de(compte);
                out.depenses_categories.entry(cat).or_default()[mois_zero] += montant;
            } else if compte.starts_with(&types.income) {
                out.revenus_mois[mois_zero] += a.number.abs();
            } else if compte.starts_with(epargne_prefixe.as_str()) {
                // Variation nette (signée) : + transfert/intérêts, − retrait.
                out.epargne_mois[mois_zero] += a.number;
            }
        }
    }

    // Comparaison N vs N−1 : présent seulement si l'année précédente a des
    // dépenses.
    let precedente = rapports_annees_traversees(directives, types, annee - 1);
    if precedente.iter().any(|d| !d.is_zero()) {
        out.depenses_annee_precedente = Some(precedente);
    }
    out
}

/// Dépenses totales par mois d'une année (utilisé pour N−1).
fn rapports_annees_traversees(
    directives: &[Directive],
    types: &AccountTypes,
    annee: i16,
) -> [Decimal; 12] {
    let mut mois_all = [Decimal::ZERO; 12];
    for directive in directives {
        let Directive::Transaction(t) = directive else {
            continue;
        };
        if t.date.year() != annee {
            continue;
        }
        let mois_zero = usize::try_from(i16::from(t.date.month()) - 1).unwrap_or(0);
        for posting in &t.postings {
            let Some(rustledger_core::IncompleteAmount::Complete(a)) = &posting.value.units else {
                continue;
            };
            if a.currency.as_str() != "EUR"
                || !posting.value.account.as_str().starts_with(&types.expenses)
            {
                continue;
            }
            mois_all[mois_zero] += a.number.abs();
        }
    }
    mois_all
}

/// Les directives `custom "budget"`, triées (date, compte) — base du diff
/// de l'éditeur.
fn directives_budget(directives: &[Directive]) -> Vec<DirectiveBudget> {
    let mut out: Vec<DirectiveBudget> = directives
        .iter()
        .filter_map(|d| match d {
            Directive::Custom(personnalisee) if personnalisee.custom_type == "budget" => {
                let compte = personnalisee.values.first().and_then(|v| match v {
                    rustledger_core::MetaValue::Account(compte) => {
                        Some(compte.as_str().to_string())
                    }
                    _ => None,
                })?;
                let intervalle = personnalisee.values.get(1).and_then(|v| match v {
                    rustledger_core::MetaValue::String(intervalle) => Some(intervalle.clone()),
                    _ => None,
                })?;
                let montant = personnalisee.values.iter().find_map(|v| match v {
                    rustledger_core::MetaValue::Amount(montant) => Some(montant.number),
                    _ => None,
                })?;
                Some(DirectiveBudget {
                    date: personnalisee.date,
                    compte,
                    intervalle,
                    montant,
                })
            }
            _ => None,
        })
        .collect();
    out.sort_unstable_by(|a, b| a.date.cmp(&b.date).then_with(|| a.compte.cmp(&b.compte)));
    out
}

/// Cascade de l'exercice : agrégats du view-model Rapports + Δ patrimoine
/// calculé sur les soldes (fin de ledger contre fin N−1, cf.
/// [`CascadeAnnee`]).
fn cascade(
    snapshot: &Snapshot,
    types: &AccountTypes,
    soldes_fin: &BTreeMap<String, BTreeMap<String, Decimal>>,
    rapports: &RapportsData,
    annee: i16,
) -> CascadeAnnee {
    let debut_exercice =
        fin_de_mois(annee.checked_sub(1).unwrap_or(annee), 12).expect("fin de decembre valide");
    let soldes_debut = snapshot
        .balances_at(debut_exercice, household_core::Through::EndOfDay)
        .map(soldes_en_map)
        .unwrap_or_default();
    CascadeAnnee {
        revenus: rapports.revenus_annee(),
        depenses: rapports.depenses_annee(),
        variation_patrimoine: patrimoine_net(soldes_fin, types)
            - patrimoine_net(&soldes_debut, types),
        epargne: rapports.epargne_annee(),
    }
}

/// Soldes par compte/devise depuis une liste de balances (un compte
/// apparaît une fois).
fn soldes_en_map(
    balances: Vec<household_core::AccountBalance>,
) -> BTreeMap<String, BTreeMap<String, Decimal>> {
    balances
        .into_iter()
        .map(|solde| (solde.account, solde.amounts))
        .collect()
}

/// Top `top` catégories décroissant ; les suivantes sont agrégées en
/// « Autres (N postes) » pour que les parts affichées somment à 100 %.
/// `colonne` : un seul mois (`Some(index 0..=11)`) ou l'année (`None`).
#[must_use]
fn repartition(
    categories: &BTreeMap<String, [Decimal; 12]>,
    colonne: Option<usize>,
    top: usize,
) -> Vec<(String, Decimal)> {
    let mut lignes: Vec<(String, Decimal)> = categories
        .iter()
        .map(|(categorie, mois)| {
            let total = match colonne {
                Some(index) => mois.get(index).copied().unwrap_or_default(),
                None => mois.iter().copied().sum(),
            };
            (categorie.clone(), total)
        })
        .filter(|(_, total)| !total.is_zero())
        .collect();
    lignes.sort_unstable_by_key(|ligne| std::cmp::Reverse(ligne.1));
    if lignes.len() <= top {
        return lignes;
    }
    let reste = lignes.len() - top;
    let autres: Decimal = lignes[top..].iter().map(|(_, total)| *total).sum();
    let mut limited: Vec<_> = lignes.into_iter().take(top).collect();
    limited.push((format!("Autres ({reste} postes)"), autres));
    limited
}

/// Le plan de comptes : opens + closes joints (bornes d'existence).
fn comptes_du_plan(snapshot: &Snapshot) -> Vec<InfoCompte> {
    let mut comptes: BTreeMap<String, (NaiveDate, Option<NaiveDate>)> = BTreeMap::new();
    for entree in snapshot.entries() {
        match &entree.value {
            Directive::Open(open) => {
                comptes
                    .entry(open.account.as_str().to_string())
                    .or_insert((open.date, None));
            }
            Directive::Close(close) => {
                comptes
                    .entry(close.account.as_str().to_string())
                    .and_modify(|(_, fermeture)| *fermeture = Some(close.date))
                    .or_insert((close.date, Some(close.date)));
            }
            _ => {}
        }
    }
    comptes
        .into_iter()
        .map(|(nom, (ouverture, fermeture))| InfoCompte {
            nom,
            ouverture,
            fermeture,
        })
        .collect()
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
///
/// Passe incrémentale unique sur les chronologie : les soldes sont accumulés
/// transaction après transaction, et le patrimoine est consigné à chaque fin
/// de mois. Complexite O(transactions) au lieu de O(mois x transactions)
/// avec un `balances_at` relance par point.
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

    // Dates de fin de mois entre premier et dernier (max 48 pour le garde-fou).
    let mut dates_fin_mois: Vec<NaiveDate> = Vec::new();
    let mut annee = premier.year();
    let mut mois = u8::try_from(premier.month()).unwrap_or(1);
    let dernier_mois = u8::try_from(dernier.month()).unwrap_or(1);
    while let Some(fin) = fin_de_mois(annee, mois) {
        dates_fin_mois.push(fin);
        if (annee, mois) == (dernier.year(), dernier_mois) {
            break;
        }
        if mois == 12 {
            annee += 1;
            mois = 1;
        } else {
            mois += 1;
        }
        if dates_fin_mois.len() > 48 {
            break;
        }
    }

    // Passe incrémentale unique : soldes accumulés transaction après transaction.
    let mut soldes: BTreeMap<String, BTreeMap<String, Decimal>> = BTreeMap::new();
    let mut points: Vec<(String, Decimal)> = Vec::with_capacity(dates_fin_mois.len());
    let entries = snapshot.entries();
    let mut entree_idx = 0;

    for &date_fin in &dates_fin_mois {
        // Appliquer toutes les transactions <= date_fin (cumulatif, entree_idx avance).
        while entree_idx < entries.len() {
            match &entries[entree_idx].value {
                Directive::Transaction(txn) if txn.date <= date_fin => {
                    for posting in &txn.postings {
                        if let Some(IncompleteAmount::Complete(a)) = &posting.value.units {
                            *soldes
                                .entry(posting.value.account.as_str().to_string())
                                .or_default()
                                .entry(a.currency.as_str().to_string())
                                .or_default() += a.number;
                        }
                    }
                    entree_idx += 1;
                }
                Directive::Transaction(_) => break, // après date_fin : arret pour cette date
                _ => entree_idx += 1,               // directive non-transactionnee : sauter
            }
        }
        let valeur = patrimoine_net(&soldes, types);
        points.push((
            format!("{:04}-{:02}", date_fin.year(), date_fin.month()),
            valeur,
        ));
    }

    if points.len() > 24 {
        points.drain(..points.len() - 24);
    }
    points
}

/// Dernier jour du mois (gère février bissextile via jiff).
/// Retourne `None` si la date est invalide (mois hors 1..=12, année hors limites).
#[must_use]
pub fn fin_de_mois(annee: i16, mois: u8) -> Option<NaiveDate> {
    let mois = i8::try_from(mois).ok()?;
    NaiveDate::new(annee, mois, 1)
        .ok()
        .map(|d| d.last_of_month())
}

/// 1er jour du mois suivant (borne exclusive de la fenêtre).
/// Retourne `None` si la date est invalide.
#[must_use]
pub fn mois_suivant_1er(annee: i16, mois: u8) -> Option<NaiveDate> {
    if mois == 12 {
        NaiveDate::new(annee.checked_add(1)?, 1, 1).ok()
    } else {
        NaiveDate::new(annee, i8::try_from(mois).ok()? + 1, 1).ok()
    }
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

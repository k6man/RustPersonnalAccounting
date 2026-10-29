# Mission

Tu es un ingénieur Rust senior. Mets en place **de bout en bout** un système de comptabilité personnelle de foyer basé sur **Beancount**, en utilisant **rustledger** (implémentation Rust de Beancount, https://github.com/rustledger/rustledger) comme moteur de parsing, validation et requêtes. Le tout est versionné avec **git**, et accompagné d'une **application GUI en Rust avec egui/eframe**.

Je suis expert en architecture et développement logiciel : pas besoin d'explications basiques, privilégie des choix techniques argumentés et un code propre et testé.

## Étape 0 : Reconnaissance (avant d'écrire du code)

1. Consulte le dépôt et la doc de rustledger (README, crates publiées, API, version courante) et **vérifie ce qui est réellement disponible** : parsing, loader/include, validation, booking, requêtes BQL, plugins, et surtout **le support de la directive `custom`** (parsing, exposition dans l'API, préservation à l'écriture). Ne suppose aucune API : lis le code ou la doc.
2. Vérifie les versions courantes d'`eframe`/`egui`, et d'éventuelles crates utiles (`egui_plot`, `rust_decimal`, `chrono`/`time`, `git2`, `notify`, `serde`, `toml`, `anyhow`/`thiserror`).
3. Présente-moi un **plan court** (architecture, structure du dépôt, jalons) ainsi que les écarts éventuels entre mes besoins et les capacités de rustledger.
4. **Crée le fichier `TODO.md`** à la racine (voir section « Suivi ») à partir de ce plan. Puis enchaîne sur l'implémentation sans attendre, sauf blocage réel.

## Contexte fonctionnel

- Foyer : **un couple + 2 enfants**, devise **EUR**, contexte **français**.
- **Un seul compte courant, commun au couple.** Pas de comptes individuels.
- Langue : noms de comptes en français, sans accents dans les identifiants (contrainte Beancount), libellés et UI en français.

### Comptes à modéliser

- `Assets:Banque:CompteCourant` (compte courant unique du foyer)
- `Assets:Epargne:LivretA` (plafond 22 950 €)
- `Assets:Epargne:LDD` (Livret de Développement Durable et Solidaire, plafond 12 000 €)
- `Income:*` : salaire de chaque conjoint (deux comptes distincts), allocations familiales, intérêts Livret A / LDD (non imposables), remboursements santé, autres revenus
- `Equity:Soldes-Ouverture`

### Plan de comptes de dépenses (postes classiques)

À fournir comme base (hiérarchie `Expenses:`), ajustable :
- **Logement** : Loyer/Credit, Charges-Copro, Assurance-Habitation, Taxe-Fonciere, Electricite, Gaz, Eau, Internet, Mobiles, Entretien-Travaux
- **Alimentation** : Courses, Restaurants, Cantine
- **Transport** : Carburant, Assurance-Auto, Entretien-Auto, Transports-En-Commun, Parking-Peages, Taxe-Auto
- **Enfants** : Creche-Garde, Periscolaire, Cantine-Scolaire, Vetements, Activites-Sport-Culture, Fournitures-Scolaires, Argent-De-Poche, Sante-Enfants
- **Sante** : Mutuelle, Medecin, Pharmacie, Dentiste-Optique
- **Loisirs** : Sorties, Abonnements (streaming, etc.), Sport, Culture
- **Vacances**
- **Impots** : Impot-Revenu (ou prélèvement à la source), Taxe-Habitation/autres
- **Assurances** diverses, **Frais-Bancaires**, **Cadeaux**, **Habillement**, **Soins-Personnels**, **Animaux** (optionnel), **Divers/Imprevus**
- Les virements vers l'épargne sont des **transferts** entre actifs, pas des dépenses.

### Épargne

- Livret A et LDD : transactions de dépôt/retrait, et **intérêts annuels** (crédités en fin d'année) modélisés comme `Income:Interets:...`.
- Plafonds paramétrables ; l'app avertit à l'approche/dépassement.
- Objectifs d'épargne mensuels budgétables (virement planifié CC → Livret A/LDD).

## Budgétisation (exigence centrale)

Beancount n'a pas de budget natif. Conçois un mécanisme qui satisfait :

1. **Budget défini mois par mois** sur une année, pour chaque poste de dépense (et d'épargne).
2. **Différent chaque année** : un fichier par année, sans héritage implicite obligatoire (mais avec un outil de copie/duplication d'une année vers la suivante, avec option d'ajustement en %).
3. **Source de vérité dans les fichiers Beancount** (lisibles, diffables en git), via des directives **`custom "budget"`** datées, compatibles avec la convention Fava :
   ```beancount
   2026-01-01 custom "budget" Expenses:Alimentation:Courses  "monthly"  850.00 EUR
   2026-07-01 custom "budget" Expenses:Alimentation:Courses  "monthly"  900.00 EUR
   ```
   - Une directive s'applique **à partir de sa date** jusqu'à la directive suivante pour le même compte.
   - Pour un budget qui varie chaque mois (ex. vacances), on pose **une directive par mois** concerné (y compris `0.00 EUR` pour « remettre à zéro »).
   - La grille « postes × 12 mois » de la GUI lit et écrit ces directives ; le fichier `budgets/AAAA.beancount` reste la seule source de vérité.
   - Si rustledger ne gère pas correctement `custom`, propose et justifie une alternative (ex. TOML/CSV à côté du ledger) et dis-moi pourquoi.
4. Calcul **budget vs réel** par mois, par poste et par groupe (agrégation hiérarchique), avec écart, % consommé, cumul annuel, et report éventuel (rollover) en option.
5. Ce calcul doit vivre dans une **bibliothèque Rust indépendante de l'UI** (testée unitairement) : l'UI ne fait qu'afficher.

## Structure du dépôt (proposition à affiner)

```
TODO.md                     # suivi du projet (voir section « Suivi »)
README.md
/ledger
  main.beancount            # options, includes
  accounts.beancount        # open/close, plan de comptes
  commodities.beancount
  /budgets/2026.beancount   # directives custom "budget" par année
  /transactions/2026/01.beancount ...   # un fichier par mois
/crates (workspace Cargo)
  /household-core           # wrapper rustledger, modèle de domaine, accès ledger
  /household-budget         # logique budgétaire (pur, testé)
  /household-git            # opérations git (statut, commit, historique)
  /household-gui            # application eframe/egui
/docs
```

Fournis aussi : `.gitignore`, `.gitattributes` si utile, un `justfile` ou `Makefile` (build, test, check, fmt, clippy), CI GitHub Actions minimale (fmt, clippy, tests, validation du ledger via rustledger).

## Git

- Le ledger est un dépôt git ; une transaction/saisie = modification de fichiers texte, jamais de format opaque.
- Écriture **déterministe et stable** (tri, indentation, alignement des montants) pour des diffs propres.
- Hook pre-commit (ou équivalent) qui **valide le ledger** (rustledger check) et refuse un commit invalide.
- Dans la GUI : affichage du statut git, bouton « Commit » avec message proposé, historique simple. Utilise `git2` ou le binaire `git` : justifie ton choix.
- Écritures atomiques (fichier temporaire + rename) pour ne jamais corrompre un fichier.

## Application GUI (egui/eframe)

Écrans minimaux :
1. **Tableau de bord** : soldes des comptes (CC, Livret A, LDD) avec jauge de plafond, patrimoine total, budget vs réel du mois courant, évolution de l'épargne (graphique).
2. **Saisie de transaction** : date, bénéficiaire, narration, postings multiples, auto-complétion des comptes/bénéficiaires, validation en direct, tags/liens, écriture dans le bon fichier mensuel.
3. **Journal** : liste filtrable/recherchable (date, compte, montant, tag), édition/suppression sûres.
4. **Budget** : grille éditable **postes × 12 mois** pour l'année choisie, totaux par ligne/colonne/groupe, duplication d'année, indicateurs d'écart.
5. **Rapports** : dépenses par catégorie/mois, comparaison année N vs N-1, graphiques (`egui_plot`).
6. **Git** : statut, diff résumé, commit.
7. Rechargement automatique si les fichiers changent sur disque (watcher).

Exigences techniques : architecture claire (état applicatif / commandes / vues), pas de logique métier dans les vues, calculs lourds hors du thread UI, gestion d'erreurs explicite (`thiserror`), montants en `Decimal` (jamais de `f64` pour l'argent).

## Suivi (TODO.md)

Crée et maintiens un fichier **`TODO.md`** à la racine du dépôt, en Markdown, sous git :

- **Structure** :
  - `## En cours` : la ou les tâches actives (une seule idéalement).
  - `## À faire` : backlog organisé par jalon (1 à 7), en cases à cocher `- [ ]`, tâches granulaires (une tâche = quelques heures de travail au plus).
  - `## Fait` : tâches terminées `- [x]`, avec la date et si possible le hash du commit.
  - `## Questions ouvertes / limites` : points bloquants ou ambigus, notamment les écarts constatés avec rustledger (ex. support de `custom`), avec la décision prise et sa justification.
  - `## Idées / hors périmètre v1` : améliorations futures (import CSV bancaire, multi-devises, comptes individuels, etc.).
- **Initialisation** : généré à la fin de l'étape 0 à partir du plan, avec tous les jalons et leurs tâches détaillées.
- **Maintenance** : mets-le à jour **à chaque jalon et à chaque fin de tâche significative**, dans le même commit que le code concerné. Ajoute les tâches découvertes en cours de route plutôt que de les perdre.
- **Reprise** : le fichier doit permettre à quelqu'un (ou à un autre agent) de reprendre le projet à froid : l'état actuel et la prochaine action doivent être évidents en le lisant.
- Mentionne son rôle et ses conventions dans le README.

## Livrables et méthode

- Procède **par jalons commités** : (1) squelette workspace + ledger d'exemple valide, (2) core + lecture ledger, (3) budget + tests, (4) GUI lecture seule (dashboard/journal), (5) saisie + écriture, (6) éditeur de budget, (7) git + polish.
- À la fin de chaque jalon : tests au vert, `TODO.md` à jour, commit avec message clair.
- Fournis un **ledger d'exemple réaliste** (au moins 2-3 mois de 2026, budget complet 2026, soldes d'ouverture, intérêts) qui passe la validation.
- Tests : unitaires sur le budget, tests d'intégration sur lecture/écriture de ledger (golden files).
- Documente dans le README : installation, lancement, conventions de nommage, format des budgets, workflow git, rôle du `TODO.md`.
- Si un point est ambigu ou si rustledger présente une limite bloquante, signale-le brièvement, consigne-le dans `TODO.md`, choisis l'option la plus raisonnable et continue.

Commence par l'étape 0.

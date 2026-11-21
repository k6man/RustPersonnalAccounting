# piloter-budget Specification

## Purpose
Construire et suivre le budget annuel : lire la grille postes × 12 mois, ajuster une cellule, commenter un poste, reporter le non-dépensé et préparer l'année suivante.

## Requirements

### Requirement: Grille postes × 12 mois
Le système SHALL construire, pour une année, une grille où chaque cellule contient le budget effectif
du mois, le réel, l'écart restant et la fraction consommée, via une comparaison mensuelle sur la
fenêtre `[1er du mois, 1er du mois suivant)` avec `children = true`.

#### Scenario: Poste sans directive en vigueur
- **WHEN** aucun budget n'est en force sur un mois pour un poste
- **THEN** la cellule vaut zéro

#### Scenario: Directive datée en milieu de mois
- **WHEN** une directive prend effet en milieu de mois
- **THEN** le budget du mois est mensualisé au prorata par jour, sur la durée calendaire du mois

#### Scenario: Réel avant le début d'effet
- **WHEN** des dépenses précèdent le début d'effet du budget
- **THEN** elles ne sont pas comptées dans le réel de la cellule

### Requirement: Agrégation hiérarchique
Le système SHALL agréger les sous-postes dans un poste parent (addition) sans absorber les lignes
enfants ; un budget intermédiaire MUST NOT être re-sommé par-dessus son agrégat.

#### Scenario: Parent avec sous-postes
- **WHEN** un compte parent et ses enfants sont budgétés
- **THEN** chaque poste garde sa propre ligne et le parent reflète l'agrégat de sa sous-arborescence

### Requirement: Rollover « cumul annuel »
Le système SHALL calculer `disponible(m) = report_initial + Σ budget(1..m) − Σ réel(1..m)` ; le report
peut être négatif et ne franchit pas les années.

#### Scenario: Mois sous le budget
- **WHEN** un mois est sous son budget
- **THEN** le disponible des mois suivants augmente de l'écart

#### Scenario: Dépassement
- **WHEN** un mois dépasse son budget
- **THEN** le disponible des mois suivants diminue, jusqu'à devenir négatif si nécessaire

### Requirement: Budget d'épargne
Le système SHALL permettre un budget sur les comptes `Assets:Epargne:*` destinataires du transfert
(seconde patte) ; les intérêts crédités sur le même compte gonflent son réel, ce que l'UI signale.

#### Scenario: Compte d'épargne budgété
- **WHEN** une ligne de grille porte sur `Assets:Epargne:LivretA`
- **THEN** une note d'avertissement sur les intérêts est associée à la ligne

### Requirement: Calendrier sûr
Le système MUST NOT paniquer sur un mois ou une année hors bornes ; les fonctions de calendrier
retournent `Option<NaiveDate>`.

#### Scenario: Mois invalide
- **WHEN** un mois hors 1..=12 est fourni
- **THEN** la fonction retourne `None`

### Requirement: Modification d'une cellule
Le système SHALL, pour modifier la cellule (1er du mois, compte), réécrire sur place le token montant
d'une directive du même mois si elle existe, sinon insérer une directive de supersession datée du 1er du
mois cible, triée par (date, compte). Une remise à zéro s'écrit `0.00 EUR` explicitement.

#### Scenario: Directive existante le même mois
- **WHEN** une directive du même mois et du même compte existe
- **THEN** seul son montant change, l'intervalle est conservé, et un montant identique laisse le fichier inchangé

#### Scenario: Pas de directive ce mois
- **WHEN** aucune directive du mois n'existe
- **THEN** une nouvelle directive `monthly` est insérée, la précédente restant valide avant sa date

#### Scenario: Confirmation du diff
- **WHEN** l'utilisateur valide une cellule dans la TUI
- **THEN** un diff de confirmation lui est présenté avant l'envoi de la commande d'écriture

### Requirement: Commentaires de lignes budgétaires
Le système SHALL stocker le commentaire d'un poste comme une ligne `; note` immédiatement au-dessus de
la première directive du compte dans le fichier de l'année (versionné avec le ledger). Un texte vide
supprime le commentaire.

#### Scenario: Commentaire non adjacent
- **WHEN** une ligne vide ou un `#` sépare le commentaire de la directive
- **THEN** il n'est rattaché à aucune directive

#### Scenario: Commentaire multi-lignes édité à la main
- **WHEN** plusieurs lignes `;` contiguës précèdent la directive
- **THEN** la lecture les joint, et l'écriture n'en produit qu'une

### Requirement: Duplication d'année
Le système SHALL dupliquer les directives de l'année N vers N+1 avec un pourcentage d'ajustement
(positif, négatif ou nul) et un arrondi au multiple supérieur d'un pas strictement positif, sortie triée
par (date, compte).

#### Scenario: Pas invalide
- **WHEN** le pas d'arrondi est inférieur ou égal à zéro
- **THEN** la duplication échoue avec `InvalidStep`

#### Scenario: Année source vide
- **WHEN** l'année source n'a aucune directive
- **THEN** la duplication échoue avec `EmptySourceYear`

#### Scenario: Année cible déjà budgétée
- **WHEN** l'année cible contient déjà des directives
- **THEN** la duplication est refusée (`Conflict`), sans écrasement silencieux

#### Scenario: Copie exacte
- **WHEN** le pourcentage vaut 0
- **THEN** les montants sont recopiés à l'identique (opération idempotente)

### Requirement: Écriture d'une nouvelle année
Le système SHALL créer `budgets/AAAA.beancount` et ajouter l'include correspondant dans `main.beancount`
en une seule mutation pré-vérifiée ; il MUST refuser d'écraser un fichier existant ou d'écrire zéro directive.

#### Scenario: Fichier cible existant
- **WHEN** `budgets/AAAA.beancount` existe déjà
- **THEN** l'écriture est refusée avec un message demandant de le supprimer ou de l'éditer

### Requirement: Compatibilité Fava
Le système SHALL conserver le format `custom "budget" Compte "interval" montant devise`, une directive par ligne.

#### Scenario: Round-trip
- **WHEN** une cellule est modifiée à valeur égale
- **THEN** le fichier est identique octet pour octet

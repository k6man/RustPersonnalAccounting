# naviguer-application Specification

## Purpose
Se déplacer dans l'application plein écran au clavier et à la souris, et recevoir un retour clair après chaque action.

## Requirements

### Requirement: Écran unique à onglets
Le système SHALL proposer sept écrans — Tableau de bord, Journal, Saisie, Budget, Rapports, Graphiques,
Git — accessibles par Ctrl+1..7 ou F1..F7 et par clic sur la barre du haut.

#### Scenario: Changement d'écran
- **WHEN** l'utilisateur presse F4
- **THEN** l'écran Budget devient actif

### Requirement: Esc ne quitte jamais
Le système SHALL ramener au Tableau de bord sur Esc ; seuls `q` et Ctrl+C quittent l'application.

#### Scenario: Esc depuis le Journal
- **WHEN** Esc est pressé sur le Journal
- **THEN** le Tableau de bord s'affiche et l'application reste ouverte

### Requirement: Routage clavier par priorité
Le système SHALL router les touches dans l'ordre : modale de confirmation, Ctrl+1..7/F1..F7, écran actif,
commandes globales (q/r/l/Esc). Les écrans à champ texte (Saisie, Git) et les éditeurs consomment toutes
les touches, lettres comprises.

#### Scenario: Lettre tapée dans un champ
- **WHEN** l'utilisateur tape `q` dans un champ texte de la Saisie
- **THEN** le caractère est inséré et l'application ne quitte pas

### Requirement: Filtrage des événements clavier
Le système SHALL ne traiter que les événements `KeyEventKind::Press`, pour ne pas doubler les touches
sur les terminaux émettant Press et Release.

#### Scenario: Press puis Release
- **WHEN** un terminal émet Press et Release pour une touche
- **THEN** la touche n'est traitée qu'une fois

### Requirement: Souris par zones enregistrées
Le système SHALL gérer la souris via des zones cliquables enregistrées au rendu, sans coordonnées codées en dur.

#### Scenario: Clic sur un onglet
- **WHEN** l'utilisateur clique sur un onglet de la barre du haut
- **THEN** l'écran correspondant devient actif

### Requirement: Affordance de test
Le système SHALL accepter `COMPTA_ECRAN=<ecran>` pour ouvrir uniquement l'écran demandé.

#### Scenario: Capture d'un écran
- **WHEN** `COMPTA_ECRAN` est défini
- **THEN** l'écran indiqué s'ouvre au démarrage

### Requirement: Flash de retour utilisateur
Le système SHALL afficher en bas d'écran un flash : un succès disparaît après 4 s, une erreur reste
affichée jusqu'à ce qu'un autre message la remplace.

#### Scenario: Succès
- **WHEN** une écriture réussit
- **THEN** le flash de succès disparaît après 4 s

#### Scenario: Erreur
- **WHEN** une écriture est refusée
- **THEN** le flash d'erreur reste affiché jusqu'au message suivant

# Spec Delta

## ADDED Requirements

### Requirement: Profil d'import par défaut configurable
Le système SHALL permettre à l'utilisateur de définir un profil d'import par défaut dans les préférences applicatives, qui sera pré-sélectionné à l'ouverture de l'écran Import s'il existe parmi les profils chargés.

#### Scenario: Profil par défaut présent dans la liste
- **WHEN** l'utilisateur ouvre l'écran Import et un `profil_import_defaut` est configuré et correspond à un profil chargé
- **THEN** ce profil est sélectionné automatiquement (index correspondant)

#### Scenario: Profil par défaut absent de la liste
- **WHEN** l'utilisateur ouvre l'écran Import et un `profil_import_defaut` est configuré mais ne correspond à aucun profil chargé
- **THEN** le premier profil de la liste est sélectionné (comportement actuel conservé)

#### Scenario: Aucun profil par défaut configuré
- **WHEN** l'utilisateur ouvre l'écran Import et aucun `profil_import_defaut` n'est configuré
- **THEN** le premier profil de la liste est sélectionné (comportement actuel conservé)
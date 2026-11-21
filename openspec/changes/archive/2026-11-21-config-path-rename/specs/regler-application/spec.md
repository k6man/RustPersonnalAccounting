## MODIFIED Requirements

### Requirement: Contenu limité aux réglages
Le système SHALL stocker uniquement des réglages applicatifs (chemin du vrai ledger, rollover par
famille) et MUST NOT écrire de préférences dans le ledger.

#### Scenario: Bascule du rollover
- **WHEN** l'utilisateur active le rollover pour une famille
- **THEN** le réglage est persisté dans le TOML local

### Requirement: Écriture atomique et démarrage tolérant
Le système SHALL écrire via temporaire caché, `sync_all` puis rename ; un TOML corrompu produit un
diagnostic et les valeurs par défaut, et l'application démarre toujours.

#### Scenario: TOML corrompu
- **WHEN** le fichier de préférences est illisible
- **THEN** un diagnostic est émis et les valeurs par défaut s'appliquent
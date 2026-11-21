## MODIFIED Requirements

### Requirement: Chaîne de résolution par priorité
Le système SHALL résoudre le ledger actif selon l'ordre : argument explicite, variable
d'environnement `RPA_LEDGER`, champ `chemin_ledger` de `~/.config/RustPersonnalAccounting/config.toml`,
puis repli sur `examples/ledger/main.beancount`. Le premier chemin *configuré* gagne, qu'il existe ou non.

#### Scenario: L'argument prime sur tout
- **WHEN** un chemin est passé en argument et `RPA_LEDGER` est aussi défini
- **THEN** le chemin de l'argument est retenu et sa source est « argument »

#### Scenario: Variable d'environnement avant la config
- **WHEN** `RPA_LEDGER` et `chemin_ledger` sont définis, sans argument
- **THEN** `RPA_LEDGER` est retenu

#### Scenario: Variable d'environnement vide
- **WHEN** `RPA_LEDGER` est vide ou ne contient que des espaces
- **THEN** elle est ignorée et l'échelon suivant s'applique

#### Scenario: Aucune configuration
- **WHEN** aucun échelon n'est configuré
- **THEN** le ledger d'exemple est retenu en mode « exemple »
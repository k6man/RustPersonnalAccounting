# ouvrir-ledger Specification

## Purpose
Choisir quel ledger ouvrir, le charger, savoir s'il s'agit des données fictives d'exemple, voir ses erreurs de validation et suivre ses modifications externes.

## Requirements

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

### Requirement: Pas de repli silencieux
Le système MUST NOT remplacer un chemin configuré mais introuvable par le ledger d'exemple.

#### Scenario: Chemin configuré absent
- **WHEN** le chemin configuré n'existe pas
- **THEN** ce chemin est renvoyé tel quel, le chargement échouera avec une bannière, et un avertissement de résolution est fourni

### Requirement: Détection du mode exemple
Le système SHALL signaler le mode « exemple » dès que le chemin résolu (canonisé) désigne le ledger
d'exemple, quel que soit l'échelon qui l'a sélectionné, afin d'afficher le badge « données fictives ».

#### Scenario: Argument pointant vers l'exemple
- **WHEN** l'argument ou `RPA_LEDGER` désigne `examples/ledger/main.beancount`
- **THEN** le mode est « exemple » et le badge est affiché

### Requirement: Surcharge de la configuration
Le système SHALL indiquer que la configuration persistée est surchargée lorsque la source gagnante est
l'argument ou `RPA_LEDGER`.

#### Scenario: Réglages désactivés
- **WHEN** la source est l'argument ou la variable d'environnement
- **THEN** `surcharge_config` est vrai et la TUI ne modifie pas `chemin_ledger`

### Requirement: Chargement complet et immuable
Le système SHALL charger includes, plugins, booking STRICT et validation, et produire un `Snapshot`
immuable ; toute écriture produit un nouveau `Snapshot`.

#### Scenario: Ledger avec erreur de validation
- **WHEN** le ledger contient une erreur de solde mais reste parsable
- **THEN** le chargement réussit, les erreurs restent consultables et `is_valid()` est faux

#### Scenario: Include manquant
- **WHEN** un fichier inclus est introuvable
- **THEN** le chargement échoue avec une erreur fatale `CoreError`

### Requirement: Directives sourcées
Le système SHALL conserver pour chaque directive son fichier, sa ligne (1-based) et sa span d'octets,
pour permettre l'édition chirurgicale.

#### Scenario: Directive synthétisée
- **WHEN** une directive n'a pas de source (ex. transaction de pad)
- **THEN** la demande de localisation retourne une erreur explicite

### Requirement: Soldes à une échelle de date
Le système SHALL calculer les soldes par compte et par devise selon la sémantique beancount v3 : une
assertion `balance` à la date D est vérifiée à 00:00 de D. Les soldes acceptent `StartOfDay` ou `EndOfDay`.

#### Scenario: Solde en début de journée
- **WHEN** les soldes sont demandés à 00:00 de D
- **THEN** seules les transactions strictement antérieures à D sont comptées

#### Scenario: Solde de fin de ledger
- **WHEN** les soldes finaux sont demandés
- **THEN** ils incluent la dernière journée transactionnée

### Requirement: Métadonnées de comptes
Le système SHALL exposer les comptes ouverts triés par nom, avec devises autorisées et plafond lu de la
métadonnée `ceiling:` de la directive `open`, ainsi que les tiers triés par fréquence puis alphabétique.

#### Scenario: Plafond déclaré
- **WHEN** un `open` porte `ceiling: 22950.00 EUR`
- **THEN** le plafond est exposé pour ce compte

### Requirement: Badge ledger d'exemple
Le système SHALL afficher le badge « LEDGER D'EXEMPLE — données fictives », le chemin et la source de
résolution dans l'en-tête lorsque le mode est « exemple ».

#### Scenario: Repli sur l'exemple
- **WHEN** aucun ledger n'est configuré
- **THEN** le badge est visible

### Requirement: Worker unique et rechargement automatique
Le système SHALL exécuter tout chargement et toute écriture dans un thread worker dédié via des
commandes sérialisées ; la boucle principale ne fait que rendu et événements. Un watcher de fichiers
déclenche un rechargement avec un debounce de 250 ms.

#### Scenario: Modification externe du ledger
- **WHEN** un éditeur externe modifie un fichier du ledger
- **THEN** un rechargement est lancé après 250 ms de calme

#### Scenario: Répertoire du ledger absent
- **WHEN** le répertoire du ledger n'existe pas
- **THEN** la surveillance est désactivée sans panique et l'application démarre avec une bannière

### Requirement: Bannière d'erreurs et échec de chargement
Le système SHALL afficher les erreurs de validation du ledger en bannière. Un échec de chargement
conserve l'état précédent affiché avec l'erreur et MUST NOT bloquer sur un spinner.

#### Scenario: Échec au démarrage
- **WHEN** le chargement initial échoue
- **THEN** l'écran d'accueil indique « Chargement impossible » avec renvoi vers la bannière

#### Scenario: Échec au rechargement
- **WHEN** un rechargement échoue après un premier chargement réussi
- **THEN** l'état précédent reste affiché avec l'erreur

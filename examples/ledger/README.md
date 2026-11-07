# Ledger d'exemple — données fictives

Ce répertoire contient le **ledger de recette** du logiciel
RustPersonalAccounting : des données **purement fictives** (personnes
anonymisées « Conjoint-1/2 », montants inventés) validées en CI par
`rledger check`. Il sert de `golden` à la suite de tests et de bac à sable
pour la recette (`just run` sans configuration).

## Ce que ce ledger n'est pas

Ce n'est **pas** votre comptabilité réelle. Ne saisissez jamais de données
vraies ici : le répertoire est versionné et vissé au dépôt de code.

## Le vrai ledger

Le vrai ledger du foyer vit **hors de ce dépôt** (dans sa propre arborescence,
idéalement son propre dépôt git). Il est désigné par la chaîne de résolution
documentée dans le README racine (§ « Utiliser avec votre vrai ledger ») :

1. argument de la ligne de commande (`compta-foyer /chemin/main.beancount`) ;
2. variable d'environnement `RPA_LEDGER` ;
3. champ `chemin_ledger` de `~/.config/rust-personal-accounting.toml` ;
4. à défaut : cet exemple (avec un avertissement « données fictives » dans
   l'interface).

Pour la recette des writers nous-mêmes (`just check`, CI, hook pre-commit),
cet exemple reste la source de vérification.

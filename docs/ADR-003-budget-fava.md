# ADR-003 — Budgets : directives `custom "budget"` fava-compatibles, source de vérité du ledger

**Statut** : acceptée (2026-10-29, jalons 1 et 3).

## Contexte

Budgétiser dans beancount revient à choisir entre :
- de la métadonnée propriété sur les `open` ;
- des `pad`/`balance` artificiels ;
- la **convention Fava** : `custom "budget" Compte "interval" montant devise`.

## Décision

1. **Directive `custom "budget"`** (convention Fava), une par ligne, un
   fichier par année (`ledger/budgets/AAAA.beancount`). Compatibilité outil
   externe (Fava) conservée.
2. Le budget d'un compte de tête agrège sa sous-arborescence
   (`children = true` côté moteur) : l'éditeur synthétise l'arbre des
   préfixes, un budget intermédiaire n'est jamais re-sommé par-dessus son
   agrégat.
3. **Supersession datée** : le changement de montant se pose au 1ᵉʳ du mois
   cible (nouvelle directive datée, la précédente reste valide avant) ;
   remise à zéro = montant `0.00 EUR` explicite. Le writer chirurgical
   (`household_budget::writer::set_cell`) modifie le token montant in place
   quand une directive du même mois existe — round-trip identique à valeur
   égale — sinon insère une supersession triée.
4. **Sémantique rollover = « cumul annuel »** : disponible(mois) = budget
   cumulé − réel cumulé depuis le 1ᵉʳ janvier (report de janvier à décembre
   ne franchit PAS les années) ; report négatif autorisé (pas de
   re-budgétisation magique). Réglage du rollover (families) = préférence
   GUI en TOML local — le ledger ne contient que des données.
5. **Budget d'épargne** sur les comptes `Assets:Epargne:*` **destinataires**
   (seconde patte du transfert) ; caveat assumé : les intérêts annuels
   crédités sur le même compte gonflent son « réel » (signalé dans l'UI).
6. **Patrimoine net = Σ bilan − Σ dettes, Equity exclu** : la directive
   d'ouverture s'annulerait sinon (Equity n'est pas un appauvrissement
   réel). Un net violerait les comptes d'actifs d'ouverture.

## Conséquences

- Les montants restent `Decimal` de bout en bout (`rust_decimal`).
- Le format est déterministe/diffable : écritures atomiques, blocs
  canoniques, diffs git propres.
- Les plafonds (`LivretA` 22 950 €, `LDD` 12 000 €) sont des métadonnées
  `ceiling:` sur les `open` — jamais de budget croisé avec le plafond :
  les jauges du tableau de bord utilisent le solde réel.

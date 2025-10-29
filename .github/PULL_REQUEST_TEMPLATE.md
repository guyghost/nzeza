## Description

<!-- Décrivez vos changements en détail -->

## Type de changement

<!-- Cochez toutes les cases qui s'appliquent -->

- [ ] 🐛 Bug fix (changement non-breaking qui corrige un problème)
- [ ] ✨ Nouvelle fonctionnalité (changement non-breaking qui ajoute une fonctionnalité)
- [ ] 💥 Breaking change (fix ou feature qui causerait un comportement existant à changer)
- [ ] 📝 Documentation (changements uniquement dans la documentation)
- [ ] 🎨 Style (formatage, points-virgules manquants, etc.)
- [ ] ♻️ Refactoring (ni fix ni feature, amélioration du code)
- [ ] ⚡ Performance (amélioration des performances)
- [ ] ✅ Tests (ajout ou correction de tests)
- [ ] 🔧 Configuration / CI (changements dans la config ou CI/CD)

## Motivation et contexte

<!-- Pourquoi ce changement est-il nécessaire? Quel problème résout-il? -->
<!-- Si cela corrige une issue ouverte, veuillez la lier ici -->

Fixes #(issue)

## Comment cela a été testé?

<!-- Décrivez comment vous avez testé vos changements -->
<!-- Incluez les détails de votre environnement de test, et les tests que vous avez exécutés -->

- [ ] Tests unitaires (`cargo test`)
- [ ] Tests d'intégration
- [ ] Tests manuels (décrire les scénarios)
- [ ] Benchmarks (si pertinent)

## Captures d'écran (si applicable)

<!-- Ajoutez des captures d'écran pour illustrer les changements UI/UX -->

## Checklist

<!-- Cochez toutes les cases qui s'appliquent. Utilisez [x] pour cocher -->

### Qualité du code

- [ ] Mon code suit le style de ce projet (vérifié avec `cargo fmt`)
- [ ] Mon code passe tous les lints (vérifié avec `cargo clippy`)
- [ ] J'ai effectué une auto-review de mon code
- [ ] J'ai commenté mon code, particulièrement dans les zones difficiles à comprendre
- [ ] Mes changements ne génèrent pas de nouveaux warnings

### Tests (TDD requis)

- [ ] J'ai écrit les tests AVANT d'implémenter la fonctionnalité (cycle TDD: Red → Green → Refactor)
- [ ] J'ai ajouté des tests qui prouvent que mon fix est efficace ou que ma fonctionnalité fonctionne
- [ ] Les tests unitaires nouveaux et existants passent localement avec mes changements
- [ ] La couverture de code reste > 80% (vérifier avec `cargo tarpaulin`)

### Documentation

- [ ] J'ai mis à jour la documentation si nécessaire
- [ ] J'ai ajouté/mis à jour les commentaires de documentation (`///` ou `//!`)
- [ ] J'ai mis à jour le fichier AGENTS.md si les processus changent

### Sécurité et dépendances

- [ ] Mon code n'introduit pas de vulnérabilités de sécurité
- [ ] J'ai vérifié les advisories de sécurité (`cargo audit`)
- [ ] Les nouvelles dépendances (si ajoutées) sont nécessaires et sûres
- [ ] J'ai vérifié les licences des nouvelles dépendances (`cargo deny check`)

### Git et workflow

- [ ] J'ai suivi le format Conventional Commits pour mes messages de commit
- [ ] Ma branche est à jour avec la branche principale
- [ ] J'ai résolu tous les conflits de merge
- [ ] Ma PR ne contient que les changements nécessaires (pas de fichiers temporaires, node_modules, etc.)

## Impact

<!-- Décrivez l'impact de ce changement sur le projet -->

- **Breaking changes:** Oui / Non
  <!-- Si oui, décrivez la migration path -->
- **Performance:** Amélioration / Neutre / Dégradation
  <!-- Si dégradation, justifiez pourquoi c'est acceptable -->
- **Dépendances ajoutées:** Oui / Non
  <!-- Si oui, listez-les et justifiez -->

## Notes pour les reviewers

<!-- Informations supplémentaires pour faciliter la review -->
<!-- Points d'attention particuliers, zones à examiner en priorité, etc. -->

## Checklist post-merge

<!-- Actions à effectuer après le merge (si applicable) -->

- [ ] Déployer en staging
- [ ] Mettre à jour la documentation externe
- [ ] Notifier les utilisateurs du changement
- [ ] Créer un release tag (si applicable)

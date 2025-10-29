# Guide de contribution

Merci de votre intérêt pour contribuer à nzeza! Ce document fournit les lignes directrices pour contribuer au projet.

## 🎯 Table des matières

- [Code de conduite](#code-de-conduite)
- [Comment contribuer](#comment-contribuer)
- [Workflow de développement](#workflow-de-développement)
- [Standards de code](#standards-de-code)
- [Tests](#tests)
- [Commits et pull requests](#commits-et-pull-requests)
- [Communication](#communication)

## 📜 Code de conduite

- Soyez respectueux et professionnel
- Accueillez les nouveaux contributeurs
- Soyez ouvert aux critiques constructives
- Concentrez-vous sur ce qui est meilleur pour la communauté
- Faites preuve d'empathie envers les autres membres

## 🤝 Comment contribuer

### Signaler des bugs

1. Vérifiez que le bug n'a pas déjà été signalé dans les [issues](https://github.com/guyghost/nzeza/issues)
2. Créez une nouvelle issue en utilisant le [template de bug report](.github/ISSUE_TEMPLATE/bug_report.yml)
3. Fournissez un maximum de détails:
   - Description claire du problème
   - Étapes de reproduction
   - Comportement attendu vs actuel
   - Logs avec `RUST_LOG=debug`
   - Environnement (OS, version Rust, etc.)

### Proposer des fonctionnalités

1. Vérifiez que la fonctionnalité n'existe pas déjà ou n'est pas en cours de développement
2. Créez une issue en utilisant le [template de feature request](.github/ISSUE_TEMPLATE/feature_request.yml)
3. Décrivez clairement:
   - Le problème à résoudre
   - La solution proposée
   - Les alternatives considérées
   - L'impact potentiel

### Contribuer du code

1. **Fork** le repository
2. **Clone** votre fork localement
3. **Créez une branche** depuis `main`:
   ```bash
   git checkout -b feat/ma-super-feature
   ```
4. **Développez** en suivant le cycle TDD
5. **Committez** régulièrement avec des messages conventionnels
6. **Poussez** vers votre fork
7. **Créez une Pull Request** vers la branche `main`

## 🔄 Workflow de développement

### Trunk-Based Development

Nous utilisons le **Trunk-Based Development**:
- Branche principale: `main` (toujours stable)
- Branches de fonctionnalités: courte durée (1-2 jours max)
- Commits fréquents (3-5 fois par jour minimum)
- Intégration rapide vers `main`

### Naming des branches

Utilisez ces préfixes:
- `feat/description` - Nouvelle fonctionnalité
- `fix/description` - Correction de bug
- `refactor/description` - Refactoring
- `docs/description` - Documentation
- `test/description` - Tests

**Exemples:**
```bash
git checkout -b feat/add-macd-indicator
git checkout -b fix/websocket-reconnection
git checkout -b refactor/simplify-rsi-calculation
```

## 📝 Standards de code

### Formatage

Utilisez `rustfmt` avec la configuration du projet:

```bash
# Vérifier le formatage
cargo fmt --all -- --check

# Formater le code
cargo fmt --all
```

### Linting

Utilisez `clippy` avec les règles strictes du projet:

```bash
# Linter
cargo clippy --all-targets --all-features -- -D warnings

# Appliquer les suggestions automatiques
cargo clippy --all-targets --all-features --fix
```

### Style

- **Largeur de ligne**: Maximum 100 caractères
- **Imports**: Groupés et triés automatiquement
- **Commentaires**: Clairs et concis, en français ou anglais
- **Noms**: Descriptifs et suivant les conventions Rust
  - `snake_case` pour variables et fonctions
  - `PascalCase` pour types et traits
  - `SCREAMING_SNAKE_CASE` pour constantes

### Documentation

Documentez le code public:

```rust
/// Calcule la moyenne mobile exponentielle (EMA).
///
/// # Arguments
///
/// * `prices` - Liste des prix historiques
/// * `period` - Période de l'EMA
///
/// # Returns
///
/// Valeur de l'EMA calculée
///
/// # Exemples
///
/// ```
/// let prices = vec![100.0, 101.0, 102.0];
/// let ema = calculate_ema(&prices, 3);
/// ```
pub fn calculate_ema(prices: &[f64], period: usize) -> f64 {
    // Implementation
}
```

## ✅ Tests

### Test-Driven Development (TDD)

**OBLIGATOIRE**: Suivez le cycle TDD pour tout nouveau code:

1. **Red**: Écrire un test qui échoue
2. **Green**: Écrire le code minimal pour passer le test
3. **Refactor**: Améliorer le code sans casser les tests

### Types de tests

#### Tests unitaires

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_calculation() {
        let prices = vec![100.0, 101.0, 102.0];
        let ema = calculate_ema(&prices, 3);
        assert!((ema - 101.0).abs() < 0.01);
    }
}
```

#### Tests d'intégration

Placez dans `tests/`:

```rust
// tests/websocket_integration.rs
use nzeza::infrastructure::websocket::*;

#[tokio::test]
async fn test_websocket_connection() {
    // Test logic
}
```

### Exécuter les tests

```bash
# Tous les tests
cargo test

# Tests spécifiques
cargo test test_ema_calculation

# Tests avec output
cargo test -- --nocapture

# Tests d'intégration seulement
cargo test --test integration_tests
```

### Couverture de code

La couverture doit rester **> 80%**:

```bash
# Générer le rapport
cargo tarpaulin --out Html

# Ouvrir le rapport
open tarpaulin-report.html
```

## 📦 Commits et Pull Requests

### Format des commits: Conventional Commits

Tous les commits doivent suivre le format [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[corps optionnel]

[footer optionnel]
```

**Types:**
- `feat`: Nouvelle fonctionnalité
- `fix`: Correction de bug
- `refactor`: Refactoring (sans changement de comportement)
- `test`: Ajout/modification de tests
- `docs`: Documentation
- `style`: Formatage, syntaxe
- `perf`: Amélioration de performance
- `chore`: Maintenance (dépendances, config)

**Exemples:**

```bash
git commit -m "feat(mpc): add price aggregation across exchanges"
git commit -m "fix(websocket): handle reconnection on connection loss"
git commit -m "test(indicators): add tests for MACD calculation"
git commit -m "docs(readme): update installation instructions"
```

### Pull Requests

#### Checklist avant de soumettre

- [ ] Code formaté (`cargo fmt`)
- [ ] Aucun warning clippy (`cargo clippy`)
- [ ] Tous les tests passent (`cargo test`)
- [ ] Couverture > 80% (`cargo tarpaulin`)
- [ ] Audit de sécurité OK (`cargo audit`)
- [ ] Documentation à jour
- [ ] Commits conventionnels
- [ ] Branche à jour avec `main`

#### Template de PR

Utilisez le [template de PR](.github/PULL_REQUEST_TEMPLATE.md) qui inclut:
- Description des changements
- Type de changement
- Motivation et contexte
- Tests effectués
- Checklist complète

#### Process de review

1. La CI doit être verte (tous les jobs passent)
2. Au moins 1 review approuvé requis
3. Pas de merge conflicts
4. Code respectant tous les standards

### Synchronisation

Synchronisez régulièrement avec `main`:

```bash
# Récupérer les dernières modifications
git fetch origin main

# Rebaser votre branche
git rebase origin/main

# En cas de conflit
git rebase --continue  # après résolution
```

## 🔒 Sécurité

### Bonnes pratiques

- **Ne jamais commiter** de secrets, clés API, ou informations sensibles
- Utiliser `.env` pour les configurations locales
- Utiliser `zeroize` pour les données sensibles en mémoire
- Valider toutes les entrées utilisateur
- Auditer régulièrement les dépendances

### Signaler une vulnérabilité

Pour signaler une vulnérabilité de sécurité:
1. **NE PAS** créer une issue publique
2. Envoyer un email à [adresse email du mainteneur]
3. Ou créer une issue avec le label `security` en mode privé

## 🛠️ Outils recommandés

### Éditeur

- **VS Code** avec extensions:
  - rust-analyzer
  - CodeLLDB
  - Better TOML
  - Error Lens

### Outils en ligne de commande

```bash
# Installer les outils de développement
cargo install cargo-watch    # Auto-recompilation
cargo install cargo-audit     # Audit de sécurité
cargo install cargo-deny      # Vérification dépendances
cargo install cargo-tarpaulin # Couverture de code
cargo install cargo-outdated  # Dépendances obsolètes
```

### Hooks Git

Créez `.git/hooks/pre-commit`:

```bash
#!/bin/sh
echo "Running pre-commit checks..."

cargo fmt --all -- --check || {
    echo "❌ Formatting check failed. Run: cargo fmt --all"
    exit 1
}

cargo clippy --all-targets --all-features -- -D warnings || {
    echo "❌ Clippy check failed. Fix warnings and try again."
    exit 1
}

cargo test --all-features || {
    echo "❌ Tests failed. Fix tests and try again."
    exit 1
}

echo "✅ All pre-commit checks passed!"
```

```bash
chmod +x .git/hooks/pre-commit
```

## 📚 Ressources

- [AGENTS.md](AGENTS.md) - Guide complet pour développeurs
- [docs/CI_CD.md](docs/CI_CD.md) - Documentation CI/CD
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Conventional Commits](https://www.conventionalcommits.org/)

## 🎓 Premiers pas

### Pour les nouveaux contributeurs

1. **Lisez** la documentation:
   - [README.md](README.md)
   - [AGENTS.md](AGENTS.md)
   - Ce guide de contribution

2. **Configurez** votre environnement:
   ```bash
   git clone https://github.com/guyghost/nzeza.git
   cd nzeza
   cargo build
   cargo test
   ```

3. **Cherchez** une issue avec le label `good-first-issue`

4. **Demandez** de l'aide si nécessaire dans les commentaires de l'issue

### Issues pour débutants

Les issues avec ces labels sont parfaites pour commencer:
- `good-first-issue` - Parfait pour débuter
- `help-wanted` - Besoin d'aide de la communauté
- `documentation` - Améliorations de documentation

## 💬 Communication

- **Issues GitHub**: Questions, bugs, features
- **Pull Requests**: Discussions sur le code
- **Commits**: Commentaires clairs et descriptifs

## ⏱️ Temps de réponse

- **Issues**: Réponse dans les 48h
- **Pull Requests**: Review dans les 72h
- **Questions**: Réponse dans les 24h

## 🙏 Reconnaissance

Tous les contributeurs seront reconnus dans:
- Le fichier AUTHORS ou CONTRIBUTORS
- Les release notes
- Les messages de commit (co-authored-by)

## 📞 Besoin d'aide?

Si vous avez des questions:
1. Consultez la [documentation](docs/)
2. Cherchez dans les [issues existantes](https://github.com/guyghost/nzeza/issues)
3. Créez une nouvelle issue avec le label `question`

---

Merci de contribuer à nzeza! 🚀

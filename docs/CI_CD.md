# CI/CD Documentation

## Vue d'ensemble

Ce projet utilise GitHub Actions pour l'intégration continue et le déploiement continu (CI/CD), suivant les meilleures pratiques pour les projets Rust.

**Note importante:** Le projet utilise des sous-modules Git (git submodules) pour certaines dépendances comme `dydx-clients`. Le fichier `.gitmodules` configure le sous-module pour pointer vers `https://github.com/dydxprotocol/v4-clients.git`. Tous les workflows incluent automatiquement `submodules: recursive` dans l'étape de checkout pour garantir que les dépendances sont correctement récupérées.

## Workflows

### 1. CI Principal (`ci.yml`)

**Déclencheurs:**
- Push sur `main` et `develop`
- Pull requests vers `main` et `develop`
- Déclenchement manuel

**Jobs:**

#### Format Check
- Vérifie le formatage du code avec `rustfmt`
- Échoue si le code n'est pas formaté selon `rustfmt.toml`

#### Clippy Lint
- Analyse statique du code avec Clippy
- Configuration dans `clippy.toml`
- Traite tous les warnings comme des erreurs

#### Test Suite
- **Multi-plateforme:** Linux, macOS, Windows
- **Multi-version:** Rust stable et beta
- Exécute tous les tests unitaires et d'intégration
- Teste également la documentation

#### Security Audit
- Analyse les vulnérabilités avec `cargo-audit`
- Vérifie la base de données RustSec

#### Dependency Check
- Vérifie les licences et sources avec `cargo-deny`
- Configuration dans `deny.toml`

#### MSRV Check
- Vérifie la compatibilité avec la version minimale supportée (Rust 1.70.0)
- Assure la rétrocompatibilité

#### Documentation
- Compile la documentation
- Échoue sur les warnings de documentation

#### CI Success
- Job de synthèse qui échoue si n'importe quel job précédent échoue
- Utilisé comme requirement pour les branch protections

### 2. Couverture de Code (`coverage.yml`)

**Déclencheurs:**
- Push sur `main`
- Pull requests vers `main`
- Déclenchement manuel

**Fonctionnalités:**
- Génère un rapport de couverture avec `cargo-tarpaulin`
- Upload vers Codecov
- Archive les rapports comme artifacts

**Objectif de couverture:** >80% (selon AGENTS.md)

### 3. Release (`release.yml`)

**Déclencheurs:**
- Tags au format `v*.*.*` (ex: v1.0.0)
- Déclenchement manuel

**Fonctionnalités:**
- Builds multi-plateformes (Linux, macOS, Windows)
- Création d'artifacts binaires optimisés
- Upload vers GitHub Releases
- Publication optionnelle sur crates.io (nécessite `CARGO_REGISTRY_TOKEN`)

### 4. Benchmarks (`benchmark.yml`)

**Déclencheurs:**
- Push sur `main`
- Pull requests vers `main`
- Déclenchement manuel

**Fonctionnalités:**
- Exécute les benchmarks de performance
- Archive les résultats dans Criterion

## Configuration

### rustfmt.toml

Configuration du formatage:
- Édition 2021
- Largeur maximale: 100 caractères
- Réorganisation automatique des imports
- Commentaires formatés et wrappés

**Commandes:**
```bash
# Vérifier le formatage
cargo fmt --all -- --check

# Formater le code
cargo fmt --all
```

### clippy.toml

Configuration du linting:
- Deny: all, correctness, suspicious, complexity, perf, style
- Warn: pedantic, nursery, cargo
- Permet certains lints trop restrictifs

**Commandes:**
```bash
# Linter le code
cargo clippy --all-targets --all-features

# Appliquer les suggestions automatiques
cargo clippy --all-targets --all-features --fix
```

### deny.toml

Configuration de cargo-deny:
- **Advisories:** Deny les vulnérabilités et yanked crates
- **Licenses:** Whitelist de licences permissives (MIT, Apache-2.0, BSD, etc.)
- **Bans:** Warn sur les versions multiples
- **Sources:** Deny les sources inconnues

**Commandes:**
```bash
# Vérifier toutes les règles
cargo deny check

# Vérifier seulement les advisories
cargo deny check advisories

# Vérifier seulement les licences
cargo deny check licenses
```

## Dependabot

Configuration automatique des mises à jour de dépendances:
- **Cargo:** Vérification hebdomadaire le lundi à 9h
- **GitHub Actions:** Vérification hebdomadaire le lundi à 9h
- Groupement des dépendances de production et de développement
- Maximum 10 PRs ouvertes pour Cargo, 5 pour Actions

## Commandes de développement

### Tests
```bash
# Tous les tests
cargo test --all-features

# Tests avec output détaillé
cargo test --all-features -- --nocapture

# Tests spécifiques
cargo test test_name

# Tests d'intégration seulement
cargo test --test integration_tests

# Tests de documentation
cargo test --doc
```

### Qualité de code
```bash
# Formatage
cargo fmt --all

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Documentation
cargo doc --no-deps --all-features --open
```

### Sécurité et dépendances
```bash
# Audit de sécurité
cargo audit

# Vérification des dépendances
cargo deny check

# Mise à jour des dépendances
cargo update
```

### Couverture
```bash
# Installer tarpaulin
cargo install cargo-tarpaulin

# Générer un rapport de couverture
cargo tarpaulin --out Html --all-features

# Rapport en XML pour CI
cargo tarpaulin --out Xml --all-features
```

### Benchmarks
```bash
# Exécuter tous les benchmarks
cargo bench

# Benchmark spécifique
cargo bench benchmark_name
```

## Protection des branches

Pour `main` et `develop`, il est recommandé de configurer:

1. **Required status checks:**
   - CI Success
   - Format Check
   - Clippy Lint
   - Test Suite (stable/ubuntu-latest)

2. **Required reviews:**
   - Au moins 1 review approuvé

3. **Autres protections:**
   - Require branches to be up to date
   - Include administrators

## Badges recommandés

Ajouter au README.md:

```markdown
[![CI](https://github.com/guyghost/nzeza/actions/workflows/ci.yml/badge.svg)](https://github.com/guyghost/nzeza/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/guyghost/nzeza/branch/main/graph/badge.svg)](https://codecov.io/gh/guyghost/nzeza)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
```

## Secrets GitHub requis

Pour un fonctionnement complet, configurer ces secrets:

1. **CARGO_REGISTRY_TOKEN:** Pour publier sur crates.io (optionnel)
2. **CODECOV_TOKEN:** Pour uploader sur Codecov (recommandé)

## Troubleshooting

### Les tests échouent en CI mais passent localement
- Vérifier que toutes les dépendances sont dans Cargo.toml
- Vérifier les variables d'environnement requises
- S'assurer que les tests ne dépendent pas de fichiers locaux

### Le build est lent
- Le cache Rust est activé via `Swatinem/rust-cache`
- Les builds incrémentaux sont automatiques
- Considérer limiter les tests sur beta aux plateformes principales

### Échec de l'audit de sécurité
- Consulter https://rustsec.org/
- Mettre à jour les dépendances: `cargo update`
- Ajouter des ignores temporaires dans `deny.toml` si nécessaire

## Intégration avec TDD

Selon les principes du projet (voir AGENTS.md):

1. **Red:** Écrire un test qui échoue
2. **Green:** Implémenter le code minimal
3. **Refactor:** Améliorer sans casser les tests

La CI garantit que:
- Tous les tests passent avant merge
- Le formatage est cohérent
- Aucune régression n'est introduite
- La couverture reste >80%

## Workflows locaux recommandés

### Avant chaque commit
```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --all-features
```

### Avant chaque push
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo audit
cargo deny check
```

### Hook pre-commit (optionnel)

Créer `.git/hooks/pre-commit`:
```bash
#!/bin/sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

```bash
chmod +x .git/hooks/pre-commit
```

## Performance

Les workflows CI sont optimisés pour:
- **Caching:** Dépendances Cargo mises en cache
- **Parallélisation:** Jobs exécutés en parallèle
- **Sélectivité:** Beta tests limités à Linux
- **Efficacité:** Builds incrémentaux activés

Temps de build typiques:
- Format check: ~30 secondes
- Clippy: ~2-3 minutes
- Tests (avec cache): ~3-5 minutes
- Couverture complète: ~5-10 minutes

## Évolutions futures

- Intégration de fuzzing avec cargo-fuzz
- Profiling automatique de performance
- Déploiement automatique vers des environnements de staging
- Tests de charge automatisés
- Notifications Slack/Discord pour les échecs

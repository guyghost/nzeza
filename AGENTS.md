# AGENTS.md - Guide pour les Agents Développeurs

## Vue d'ensemble du projet
Ce projet implémente un serveur MPC (Multi-Party Computation) connecté à plusieurs échanges de crypto-monnaies : dydx (v4), hyperliquid, coinbase, binance, et kraken. L'architecture suit les principes DDD (Domain-Driven Design) avec des acteurs asynchrones utilisant des WebSockets pour des mises à jour de prix en temps réel.

## Fonctionnalités MPC supportées
Le serveur supporte diverses computations multi-parties :

- **Price Aggregation** : Agrège les prix à travers les échanges pour obtenir une moyenne fiable.
- **Order Matching** : Fait correspondre les ordres à travers les participants de manière sécurisée.
- **Portfolio Optimization** : Optimise les portefeuilles en utilisant des computations MPC pour la confidentialité.

## Fonctionnalités de Trading
Le serveur inclut des outils avancés pour le trading algorithmique :

- **Available Indicators** : EMA, RSI, Bollinger Bands, MACD, Stochastic Oscillator, Volume (VWAP)
- **Strategies** : Fast Scalping, Momentum Scalping, Conservative Scalping
- **Signal Combination** : Weighted combination with confidence scoring

## Configuration des Logs

Le système utilise `tracing` pour la journalisation avec différents niveaux de verbosité. La configuration se fait via la variable d'environnement `RUST_LOG`.

**Note:** Les variables d'environnement peuvent être configurées dans un fichier `.env` à la racine du projet. Voir `ENV-README.md` pour les instructions détaillées.

### Niveaux de Log Disponibles

- **`trace`** : Le plus verbeux - tous les détails internes
- **`debug`** : Informations de débogage (prix individuels, détails techniques)
- **`info`** : Informations générales importantes (prix agrégés, connexions)
- **`warn`** : Avertissements (problèmes non critiques)
- **`error`** : Erreurs (problèmes critiques)

### Configuration par Défaut

Par défaut, le système affiche uniquement les logs de niveau `info` et supérieur pour le module `nzeza` :

```bash
# Configuration par défaut (équivalent à RUST_LOG=nzeza=info)
cargo run
```

### Afficher Plus de Détails

Pour voir les logs de débogage (prix individuels par échange) :

```bash
# Afficher debug et supérieur pour nzeza
RUST_LOG=nzeza=debug cargo run

# Afficher tous les détails (très verbeux)
RUST_LOG=nzeza=trace cargo run
```

### Configuration Avancée

```bash
# Différents niveaux par module
RUST_LOG=nzeza=info,tokio=warn cargo run

# Tout en debug sauf certains modules
RUST_LOG=debug,tokio=warn,hyper=warn cargo run

# Uniquement les erreurs
RUST_LOG=error cargo run
```

### Exemples Pratiques

```bash
# Développement : voir les prix individuels et détails de débogage
RUST_LOG=nzeza=debug cargo run

# Production : uniquement les informations importantes
RUST_LOG=nzeza=info cargo run

# Diagnostic : voir tous les détails internes
RUST_LOG=nzeza=trace cargo run

# Silencieux : uniquement les erreurs
RUST_LOG=error cargo run
```

### Logs Actuellement Configurés

- **Prix individuels par échange** : `debug` (masqué par défaut)
- **Prix agrégés** : `info` (visible par défaut)
- **Connexions WebSocket** : `info` (visible par défaut)
- **Erreurs** : `error` (toujours visible)
- **Vérifications de santé** : `info` (visible par défaut)

## Configuration dYdX

Pour activer le trading sur dYdX v4, vous devez configurer la variable d'environnement `DYDX_MNEMONIC` avec votre phrase mnémonique.

### Configuration de la Clé Mnémonique

```bash
# Définir la variable d'environnement pour la session actuelle
export DYDX_MNEMONIC="your twelve word mnemonic phrase here"

# Ou l'ajouter à votre ~/.bashrc ou ~/.zshrc pour la persistance
echo 'export DYDX_MNEMONIC="your twelve word mnemonic phrase here"' >> ~/.bashrc

# Ou utiliser le fichier .env (recommandé)
echo 'DYDX_MNEMONIC=your twelve word mnemonic phrase here' >> .env
```

### Sécurité Importante

⚠️ **NEVER commit your mnemonic to version control or share it with anyone.**

- Utilisez toujours des variables d'environnement pour les clés sensibles
- Considérez l'utilisation d'un fichier de configuration séparé pour les environnements de production
- Assurez-vous que votre phrase mnémonique est sauvegardée de manière sécurisée

### Vérification de la Configuration

Le système vérifiera automatiquement la présence de `DYDX_MNEMONIC` au démarrage :

- ✅ **Présente** : Trading dYdX activé avec message de confirmation
- ❌ **Absente** : Trading dYdX désactivé avec avertissement

### Fonctionnalités dYdX Supportées

- **Placement d'ordres** : Market et Limit orders
- **Annulation d'ordres** : Par order ID
- **Statut d'ordres** : Vérification du statut des ordres
- **Signature EIP-712** : Authentification sécurisée
- **Gestion des séquences** : Numéros de séquence automatiques

### Test de l'Intégration

Pour tester l'intégration dYdX sans risquer de vrais fonds :

1. Utilisez un portefeuille de test avec des fonds de testnet
2. Configurez l'environnement pour pointer vers testnet (si supporté)
3. Vérifiez les logs pour confirmer l'initialisation réussie du client dYdX

## Configuration Coinbase

Pour activer le trading sur Coinbase Pro, vous devez configurer trois variables d'environnement avec vos identifiants API.

### Configuration des Clés API Coinbase

```bash
# Définir les variables d'environnement pour la session actuelle
export COINBASE_API_KEY="your_api_key_here"
export COINBASE_API_SECRET="your_api_secret_here"
export COINBASE_PASSPHRASE="your_passphrase_here"  # Optionnel - certains API keys n'en nécessitent pas

# Ou les ajouter à votre ~/.bashrc ou ~/.zshrc pour la persistance
echo 'export COINBASE_API_KEY="your_api_key_here"' >> ~/.bashrc
echo 'export COINBASE_API_SECRET="your_api_secret_here"' >> ~/.bashrc
echo 'export COINBASE_PASSPHRASE="your_passphrase_here"' >> ~/.bashrc  # Optionnel

# Ou utiliser le fichier .env (recommandé)
echo 'COINBASE_API_KEY=your_api_key_here' >> .env
echo 'COINBASE_API_SECRET=your_api_secret_here' >> .env
echo 'COINBASE_PASSPHRASE=your_passphrase_here' >> .env  # Optionnel
```

### Création des Clés API Coinbase

1. **Connectez-vous** à votre compte Coinbase Pro
2. **Allez dans** Settings → API
3. **Cliquez sur** "Create New API Key"
4. **Configurez les permissions** :
   - ✅ View (requis pour les prix et soldes)
   - ✅ Trade (requis pour placer/annuler des ordres)
5. **Notez** votre Passphrase (nécessaire pour l'authentification)
6. **Sauvegardez** l'API Key et le Secret

### Sécurité Importante

⚠️ **NEVER commit your Coinbase API credentials to version control or share them with anyone.**

- Utilisez toujours des variables d'environnement pour les clés sensibles
- Activez uniquement les permissions nécessaires (View + Trade)
- Utilisez des clés API séparées pour les environnements de développement/production
- Coinbase permet de restreindre les adresses IP pour plus de sécurité

### Fonctionnalités Coinbase Supportées

- **Placement d'ordres** : Market et Limit orders
- **Annulation d'ordres** : Par order ID
- **Statut d'ordres** : Vérification du statut des ordres
- **Authentification HMAC-SHA256** : Sécurisée avec timestamp + passphrase (optionnel)
- **WebSocket temps réel** : Flux de prix pour BTC-USD, ETH-USD, SOL-USD

### Vérification de la Configuration

Le système vérifiera automatiquement la présence des variables Coinbase au démarrage :

- ✅ **Présentes** : Trading Coinbase activé avec message de confirmation
- ❌ **Absentes** : Trading Coinbase désactivé avec avertissement

### Test de l'Intégration

Pour tester l'intégration Coinbase sans risquer de vrais fonds :

1. Utilisez un compte de test Coinbase Pro (sandbox)
2. Configurez les variables d'environnement pour pointer vers sandbox
3. Vérifiez les logs pour confirmer l'initialisation réussie du client Coinbase

### Configuration Sandbox (Test)

Pour utiliser l'environnement de test Coinbase :

```bash
# Variables pour le sandbox
export COINBASE_API_KEY="your_sandbox_api_key"
export COINBASE_API_SECRET="your_sandbox_api_secret" 
export COINBASE_PASSPHRASE="your_sandbox_passphrase"
```

Le système détecte automatiquement l'environnement sandbox via les clés API.

## Approche de développement : Test-Driven Development (TDD)

### Principe fondamental
**Tous les développements doivent suivre le cycle TDD :**
1. **Red** : Écrire un test qui échoue (car la fonctionnalité n'existe pas encore).
2. **Green** : Implémenter le code minimal pour que le test passe.
3. **Refactor** : Améliorer le code sans changer son comportement, en gardant les tests verts.

### Pourquoi TDD ?
- **Qualité** : Code plus fiable, moins de bugs.
- **Design** : Force à penser aux interfaces et à la modularité.
- **Maintenance** : Tests comme documentation vivante et filet de sécurité lors des refactorings.
- **Confiance** : Permet de modifier le code en toute sécurité.

### Outils en Rust
- Utiliser `cargo test` pour exécuter les tests.
- Tests unitaires dans `#[cfg(test)]` modules.
- Tests d'intégration dans `tests/` directory.
- Couverture avec `cargo tarpaulin` (si installé).

### Structure des tests
- **Tests unitaires** : Pour les fonctions pures, value objects, entités.
- **Tests d'acteurs** : Pour les interactions asynchrones (utiliser `tokio::test`).
- **Tests d'intégration** : Pour les connexions WebSocket et APIs externes (avec mocks si nécessaire).

### Exemple de cycle TDD
```rust
// 1. Écrire le test (Red)
#[test]
fn test_price_creation() {
    let price = Price::new(100.0);
    assert!(price.is_ok());
}

// 2. Implémenter le code (Green)
impl Price {
    pub fn new(value: f64) -> Result<Self, String> {
        if value >= 0.0 {
            Ok(Price(value))
        } else {
            Err("Price must be non-negative".to_string())
        }
    }
}

// 3. Refactor si nécessaire
```

### Règles strictes
- **Pas de code sans test** : Toute nouvelle fonctionnalité doit être couverte par des tests.
- **Tests avant le code** : Écrire le test AVANT d'implémenter la fonctionnalité.
- **Tests verts en permanence** : Ne jamais commiter avec des tests rouges.
- **Couverture minimale** : Viser >80% de couverture de code.
- **Tests rapides** : Les tests unitaires doivent être <1ms chacun.

### Commandes essentielles
```bash
# Exécuter tous les tests
cargo test

# Exécuter avec couverture (si tarpaulin installé)
cargo tarpaulin --out Html

# Tests spécifiques
cargo test test_price_creation

# Tests d'intégration seulement
cargo test --test integration_tests
```

### Bonnes pratiques
- **Noms descriptifs** : `test_should_return_error_when_price_negative`
- **Un test, un comportement** : Chaque test vérifie une seule chose.
- **Mocks/Stubs** : Pour les dépendances externes (APIs, WebSockets).
- **Assertions claires** : Utiliser `assert_eq!`, `assert!`, etc. avec messages.
- **Tests indépendants** : Pas de dépendances entre tests.

### Intégration continue
- Les tests doivent passer sur toutes les branches.
- Utiliser GitHub Actions ou similaire pour CI.
- Bloquer les merges si tests échouent.

### Rappel
Le TDD n'est pas optionnel ; c'est la méthode imposée pour garantir la robustesse du serveur MPC. Toute violation sera rejetée lors des reviews.

## Workflow Git : Trunk-Based Development

### Principe fondamental
Ce projet utilise le **Trunk-Based Development**, une approche de gestion de version centrée sur une branche principale (`main`) toujours déployable.

### Règles du Trunk-Based Development

#### 1. Branche principale (`main`)
- **Toujours stable** : La branche `main` doit toujours compiler et passer tous les tests.
- **Source unique de vérité** : Toutes les fonctionnalités partent de `main` et y retournent rapidement.
- **Déployable à tout moment** : Chaque commit sur `main` devrait être potentiellement déployable en production.

#### 2. Branches de fonctionnalités (Feature Branches)
- **Courte durée de vie** : Maximum 1-2 jours avant merge.
- **Petites et focalisées** : Une branche = une fonctionnalité ou un fix spécifique.
- **Naming convention** :
  - `feat/description` : Nouvelles fonctionnalités
  - `fix/description` : Corrections de bugs
  - `refactor/description` : Refactoring sans changement de comportement
  - `docs/description` : Documentation uniquement
  - `test/description` : Ajout ou modification de tests

#### 3. Commits fréquents
- **Commit souvent** : Au moins plusieurs fois par jour.
- **Conventional Commits** : Utiliser le format standard pour les messages de commit.
- **Atomicité** : Chaque commit doit représenter une unité logique de changement.

#### 4. Intégration continue
- **Pull/Push réguliers** : Synchroniser avec `main` plusieurs fois par jour.
- **Rebase** : Préférer `git rebase` à `git merge` pour garder un historique linéaire.
- **CI/CD** : Tous les tests doivent passer avant le merge.

### Format des commits : Conventional Commits

Tous les commits doivent suivre le format **Conventional Commits** :

```
<type>(<scope>): <description>

[corps optionnel]

[footer optionnel]
```

#### Types de commits
- `feat` : Nouvelle fonctionnalité
- `fix` : Correction de bug
- `refactor` : Refactoring sans changement de comportement
- `test` : Ajout ou modification de tests
- `docs` : Documentation uniquement
- `style` : Formatage, points-virgules manquants, etc.
- `perf` : Amélioration de performance
- `chore` : Tâches de maintenance (dépendances, config, etc.)

#### Exemples
```bash
feat(mpc): add price aggregation across exchanges
fix(websocket): handle reconnection on connection loss
refactor(indicators): simplify RSI calculation logic
test(strategies): add tests for momentum scalping
docs(readme): update installation instructions
```

### Workflow quotidien

#### 1. Démarrer une nouvelle tâche
```bash
# Synchroniser avec main
git checkout main
git pull origin main

# Créer une branche de fonctionnalité
git checkout -b feat/add-new-indicator
```

#### 2. Développer avec TDD
```bash
# Cycle TDD : Red -> Green -> Refactor
# Faire des commits atomiques fréquents
git add .
git commit -m "test(indicators): add failing test for MACD"
git commit -m "feat(indicators): implement MACD calculation"
git commit -m "refactor(indicators): extract common EMA logic"
```

#### 3. Synchroniser régulièrement
```bash
# Plusieurs fois par jour
git fetch origin
git rebase origin/main
```

#### 4. Merger vers main
```bash
# S'assurer que tout est à jour
git checkout main
git pull origin main
git checkout feat/add-new-indicator
git rebase main

# Tous les tests doivent passer
cargo test

# Merger (fast-forward si possible)
git checkout main
git merge feat/add-new-indicator
git push origin main

# Supprimer la branche locale
git branch -d feat/add-new-indicator
```

### Bonnes pratiques

#### DO ✅
- Commiter au moins 3-5 fois par jour sur votre branche
- Synchroniser avec `main` au moins 2 fois par jour
- Garder les branches de fonctionnalités < 2 jours
- Utiliser des messages de commit descriptifs et conventionnels
- Faire tourner tous les tests avant de pusher
- Merger rapidement pour éviter les divergences

#### DON'T ❌
- Ne jamais commiter directement sur `main` sans tests
- Ne pas garder une branche de fonctionnalité > 3 jours
- Ne pas merger avec des tests qui échouent
- Ne pas utiliser `git merge` (préférer `rebase` pour garder un historique linéaire)
- Ne pas accumuler trop de commits non pushés
- Ne pas ignorer les conflits de merge

### Gestion des conflits
```bash
# En cas de conflit lors du rebase
git rebase origin/main

# Résoudre les conflits dans les fichiers
# Puis continuer le rebase
git add .
git rebase --continue
```

### Vérification avant merge
```bash
# Checklist avant de merger vers main
cargo test                    # Tous les tests passent
cargo clippy                  # Pas d'avertissements Clippy
cargo fmt -- --check          # Code correctement formaté
git log --oneline            # Vérifier l'historique des commits
```

### Intégration avec CI/CD
- **GitHub Actions** : Automatiser les tests sur chaque push
- **Pre-commit hooks** : Vérifier le formatage et les tests avant chaque commit
- **Branch protection** : Protéger `main` contre les pushs directs non vérifiés

### Résumé
Le Trunk-Based Development favorise :
- **Intégration rapide** : Réduire les risques de conflits
- **Feedback continu** : Détecter les problèmes tôt
- **Simplicité** : Moins de branches, historique plus clair
- **Qualité** : Tests continus, code toujours déployable
# nzeza

[![CI](https://github.com/guyghost/nzeza/actions/workflows/ci.yml/badge.svg)](https://github.com/guyghost/nzeza/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/guyghost/nzeza/branch/main/graph/badge.svg)](https://codecov.io/gh/guyghost/nzeza)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

Serveur MPC (Multi-Party Computation) connecté à plusieurs échanges de crypto-monnaies pour le trading algorithmique sécurisé.

## 🚀 Fonctionnalités

### Multi-Party Computation
- **Price Aggregation**: Agrégation de prix à travers plusieurs échanges
- **Order Matching**: Appariement d'ordres sécurisé entre participants
- **Portfolio Optimization**: Optimisation de portefeuille avec confidentialité

### Trading Algorithmique
- **Indicateurs**: EMA, RSI, Bollinger Bands, MACD, Stochastic Oscillator, VWAP
- **Stratégies**: Fast Scalping, Momentum Scalping, Conservative Scalping
- **Signal Combination**: Combinaison pondérée avec scoring de confiance

### Échanges supportés
- dYdX v4
- Hyperliquid
- Coinbase Advanced Trade
- Binance
- Kraken

## 📋 Prérequis

- Rust 1.70.0 ou supérieur
- Cargo
- OpenSSL (pour les connexions TLS)

## 🔧 Installation

```bash
# Cloner le repository
git clone https://github.com/guyghost/nzeza.git
cd nzeza

# Installer les dépendances
cargo build

# Exécuter les tests
cargo test
```

## ⚙️ Configuration

Créer un fichier `.env` à la racine du projet avec les variables d'environnement nécessaires :

```bash
# Configuration des logs
RUST_LOG=nzeza=info

# dYdX v4
DYDX_MNEMONIC="your twelve word mnemonic phrase here"

# Coinbase Advanced Trade
COINBASE_API_KEY="your_api_key_here"
COINBASE_API_SECRET="your_api_secret_here"
COINBASE_PASSPHRASE="your_passphrase_here"  # Optionnel
```

Pour plus de détails, voir [docs/ENV-README.md](docs/ENV-README.md) et [docs/TRADING_SETUP.md](docs/TRADING_SETUP.md).

## 🏃 Utilisation

```bash
# Mode développement avec logs détaillés
RUST_LOG=nzeza=debug cargo run

# Mode production
RUST_LOG=nzeza=info cargo run --release

# Voir tous les détails internes
RUST_LOG=nzeza=trace cargo run
```

## 🧪 Tests

Le projet suit strictement l'approche **Test-Driven Development (TDD)**.

```bash
# Tous les tests
cargo test

# Tests avec output détaillé
cargo test -- --nocapture

# Tests d'intégration seulement
cargo test --test integration_tests

# Tests avec couverture
cargo tarpaulin --out Html
```

**Objectif de couverture**: >80%

## 🔍 Qualité de code

```bash
# Vérifier le formatage
cargo fmt --all -- --check

# Formater le code
cargo fmt --all

# Linting avec Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Audit de sécurité
cargo audit

# Vérifier les dépendances
cargo deny check
```

## 📚 Documentation

- [CI/CD Setup](docs/CI_CD.md) - Configuration et utilisation de la CI/CD
- [Agent Guidelines](AGENTS.md) - Guide pour les développeurs et agents IA
- [Trading Setup](docs/TRADING_SETUP.md) - Configuration du trading
- [Environment Variables](docs/ENV-README.md) - Variables d'environnement

### Générer la documentation

```bash
cargo doc --no-deps --all-features --open
```

## 🤝 Contribution

Nous suivons le **Trunk-Based Development** avec des branches de courte durée et des commits fréquents.

### Workflow

1. Créer une branche depuis `main`:
   ```bash
   git checkout -b feat/ma-fonctionnalite
   ```

2. Développer avec TDD (Red → Green → Refactor)

3. Commits fréquents avec [Conventional Commits](https://www.conventionalcommits.org/):
   ```bash
   git commit -m "feat(mpc): add price aggregation"
   git commit -m "test(mpc): add tests for price aggregation"
   ```

4. Pousser et créer une Pull Request

### Types de commits

- `feat`: Nouvelle fonctionnalité
- `fix`: Correction de bug
- `refactor`: Refactoring
- `test`: Ajout/modification de tests
- `docs`: Documentation
- `chore`: Maintenance (dépendances, config, etc.)
- `perf`: Amélioration de performance

### Avant de soumettre

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --all-features
cargo audit
```

Voir le [template de PR](.github/PULL_REQUEST_TEMPLATE.md) pour plus de détails.

## 🔐 Sécurité

- Ne jamais commiter de secrets ou clés API
- Utiliser `.env` pour les informations sensibles
- Les audits de sécurité sont exécutés automatiquement en CI
- Signaler les vulnérabilités en créant une issue avec le label `security`

## 📊 CI/CD

Le projet utilise GitHub Actions pour:
- ✅ Tests multi-plateformes (Linux, macOS, Windows)
- ✅ Vérification du formatage et linting
- ✅ Audits de sécurité automatiques
- ✅ Couverture de code
- ✅ Builds de release
- ✅ Mises à jour automatiques des dépendances (Dependabot)

Voir [docs/CI_CD.md](docs/CI_CD.md) pour plus de détails.

## 🏗️ Architecture

Le projet suit les principes **Domain-Driven Design (DDD)**:

```
src/
├── domain/          # Logique métier et entités
├── application/     # Cas d'usage et orchestration
├── infrastructure/  # WebSockets, APIs externes
└── persistence/     # Stockage de données
```

## 📝 License

MIT License - voir le fichier [LICENSE](LICENSE) pour plus de détails.

## 🙏 Remerciements

- [dYdX](https://dydx.exchange/) pour l'API v4
- [Hyperliquid](https://hyperliquid.xyz/)
- [Coinbase](https://www.coinbase.com/)
- [Binance](https://www.binance.com/)
- [Kraken](https://www.kraken.com/)

## 📧 Contact

Pour toute question ou suggestion, ouvrir une [issue](https://github.com/guyghost/nzeza/issues/new/choose).

---

**Note**: Ce projet est en développement actif. Les fonctionnalités et l'API peuvent changer.

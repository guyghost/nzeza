pou# Guide de Configuration du Trading Automatique

## Vue d'Ensemble

NZEZA supporte le trading automatique sur plusieurs exchanges :
- ✅ **Binance** - Fonctionnel
- ✅ **Coinbase Pro** - Fonctionnel
- ⚠️ **dYdX v4** - Activé mais avec problèmes connus (voir avertissements)
- ✅ **Hyperliquid** - Fonctionnel
- ✅ **Kraken** - Fonctionnel

## Configuration Rapide

### 1. Générer une Clé API Sécurisée

```bash
# Génération d'une clé API de 32+ caractères (requis)
openssl rand -base64 32

# Exemple de sortie : 4f8jK2mN9pL3qR5tU7vW8xY0zA1bC3dE4fG6hI8jK==
```

### 2. Configurer les Variables d'Environnement

Copiez `.env.example` vers `.env` :

```bash
cp .env.example .env
```

Éditez `.env` avec vos paramètres :

```bash
# API Authentication (REQUIS - minimum 32 caractères)
API_KEYS=votre_cle_api_securisee_de_32_caracteres_ou_plus

# Coinbase Pro (Recommandé pour commencer)
COINBASE_API_KEY=votre_cle_coinbase
COINBASE_API_SECRET=votre_secret_coinbase
COINBASE_PASSPHRASE=votre_passphrase  # Optionnel

# dYdX v4 (⚠️ Utilisez avec précaution - voir section ci-dessous)
# DYDX_MNEMONIC=vos douze mots de phrase mnémonique ici

# Trading Configuration
ENABLE_AUTOMATED_TRADING=true
MIN_CONFIDENCE_THRESHOLD=0.7
MAX_POSITIONS_PER_SYMBOL=1
MAX_TOTAL_POSITIONS=5
DEFAULT_POSITION_SIZE=0.001
STOP_LOSS_PERCENTAGE=0.05      # 5% stop loss
TAKE_PROFIT_PERCENTAGE=0.10    # 10% take profit
MAX_TRADES_PER_HOUR=10
MAX_TRADES_PER_DAY=50
```

## Configuration par Exchange

### Coinbase Pro (Recommandé)

1. **Créer des Clés API Coinbase** :
   - Connectez-vous à [Coinbase Pro](https://pro.coinbase.com)
   - Allez dans Settings > API
   - Créez une nouvelle clé API

2. **Permissions Recommandées** :
   - ✅ View (lecture seule)
   - ✅ Trade (si trading activé)
   - ❌ Transfer (NE JAMAIS activer)
   - ❌ Withdraw (NE JAMAIS activer)

3. **Configuration** :
   ```bash
   COINBASE_API_KEY=abc123...
   COINBASE_API_SECRET=xyz789...
   COINBASE_PASSPHRASE=votre_passphrase  # Peut être omis si non requis
   ```

4. **Symboles Supportés** :
   - BTC-USD
   - ETH-USD
   - SOL-USD
   - (Configuration dans `src/config.rs`)

### dYdX v4 (⚠️ Avertissements Importants)

**ATTENTION** : L'intégration dYdX v4 actuelle a des problèmes connus :

#### ⚠️ Problèmes Connus
- Utilise la signature Ethereum (EIP-712) au lieu de Cosmos SDK
- Les ordres **PEUVENT ÊTRE REJETÉS** par l'exchange
- **NON RECOMMANDÉ pour la production**
- Fonctionne uniquement pour les tests

#### Configuration (à vos risques)
```bash
# ⚠️ Utilisez uniquement pour les tests
DYDX_MNEMONIC=vos douze mots de phrase mnémonique ici
```

#### Solution pour Production
Pour utiliser dYdX en production, il faut :
1. Implémenter l'intégration Cosmos SDK appropriée
2. Utiliser l'encodage protobuf au lieu d'EIP-712
3. Référence : https://github.com/dydxprotocol/v4-clients

#### Alternative Recommandée
Utilisez **Coinbase Pro** ou **Binance** à la place jusqu'à ce que l'intégration Cosmos soit implémentée.

### Binance

```bash
# Binance API
BINANCE_API_KEY=votre_cle_binance
BINANCE_API_SECRET=votre_secret_binance
```

Symboles : BTCUSDT, ETHUSDT, SOLUSDT

### Hyperliquid

Configuration spécifique à Hyperliquid (voir documentation Hyperliquid).

Symboles : BTC, ETH, SOL

### Kraken

```bash
# Kraken API
KRAKEN_API_KEY=votre_cle_kraken
KRAKEN_API_SECRET=votre_secret_kraken
```

Symboles : BTC/USD, ETH/USD, SOL/USD

## Paramètres de Trading

### Seuil de Confiance
```bash
MIN_CONFIDENCE_THRESHOLD=0.7  # Entre 0.0 et 1.0
```
- Confiance minimale requise pour exécuter un trade
- Valeur par défaut : 0.7 (70%)
- Plus élevé = moins de trades mais plus de qualité

### Gestion des Positions
```bash
MAX_POSITIONS_PER_SYMBOL=1  # Max positions par symbole
MAX_TOTAL_POSITIONS=5       # Max positions totales
```

### Taille des Positions
```bash
DEFAULT_POSITION_SIZE=0.001              # Taille par défaut (BTC)
PORTFOLIO_PERCENTAGE_PER_POSITION=0.02   # 2% du portfolio par position
```

### Gestion du Risque
```bash
STOP_LOSS_PERCENTAGE=0.05      # 5% stop loss
TAKE_PROFIT_PERCENTAGE=0.10    # 10% take profit
```

### Limites de Trading
```bash
MAX_TRADES_PER_HOUR=10   # Maximum 10 trades par heure
MAX_TRADES_PER_DAY=50    # Maximum 50 trades par jour
```

## Stratégies de Trading

NZEZA utilise plusieurs stratégies de scalping :

1. **Fast Scalping** - Trades rapides avec petits profits
2. **Conservative Scalping** - Approche plus conservatrice
3. **Momentum Scalping** - Suit les mouvements de momentum
4. **Signal Combiner** - Combine les signaux de toutes les stratégies

Les stratégies sont configurées automatiquement. Les poids sont ajustés dynamiquement en fonction de la performance.

## Démarrage

### Mode Test (Recommandé pour débuter)

```bash
# 1. Désactiver le trading automatique
export ENABLE_AUTOMATED_TRADING=false

# 2. Lancer l'application
cargo run --release

# 3. Observer les signaux sans exécuter de trades
# L'application générera des signaux mais ne passera pas d'ordres
```

### Mode Production

```bash
# 1. Vérifier la configuration
cat .env

# 2. Vérifier que les clés API sont valides
# 3. Activer le trading automatique
export ENABLE_AUTOMATED_TRADING=true

# 4. Lancer l'application
cargo run --release
```

## Monitoring

### Endpoints API

L'application expose plusieurs endpoints pour le monitoring :

```bash
# Health check
curl http://localhost:3000/health

# Métriques
curl -H "Authorization: Bearer VOTRE_CLE_API" \
  http://localhost:3000/metrics

# Positions ouvertes
curl -H "Authorization: Bearer VOTRE_CLE_API" \
  http://localhost:3000/positions

# Historique des trades
curl -H "Authorization: Bearer VOTRE_CLE_API" \
  http://localhost:3000/trades?limit=100
```

### Logs

Les logs sont disponibles dans la console et contiennent :
- Signaux de trading générés
- Ordres passés
- Confirmations d'exécution
- Erreurs et avertissements

Niveau de log configurable :
```bash
RUST_LOG=nzeza=info    # info, debug, trace
```

## Sécurité

### ✅ Bonnes Pratiques

1. **Clés API** :
   - Minimum 32 caractères (256 bits)
   - Générez avec `openssl rand -base64 32`
   - Ne jamais commiter dans Git

2. **Permissions Exchange** :
   - View + Trade uniquement
   - **JAMAIS** Transfer ou Withdraw
   - Whitelist IP si disponible

3. **Hardware Wallet** (Production) :
   - Utilisez un Ledger ou Trezor pour les mnémoniques
   - Ne stockez jamais les mnémoniques en clair
   - Framework hardware wallet disponible dans `src/hardware_wallet.rs`

4. **1Password CLI** (Recommandé) :
   ```bash
   # Installer 1Password CLI
   brew install --cask 1password-cli

   # Stocker les secrets
   op item create --category=password \
     --title="NZEZA Coinbase" \
     "api_key[password]=$COINBASE_API_KEY"

   # Charger depuis 1Password
   export COINBASE_API_KEY=$(op read "op://Private/NZEZA Coinbase/api_key")
   ```

### ⚠️ Avertissements

1. **dYdX** : Intégration avec problèmes - ordres peuvent échouer
2. **Variables d'environnement** : Visibles dans les processus
3. **Fichier .env** : Ne jamais commiter (déjà dans .gitignore)
4. **Tests en production** : Commencez avec de petites positions
5. **Monitoring** : Surveillez activement les premiers jours

## Base de Données (Persistence)

Le système utilise SQLite pour la persistence :

```bash
# Configuration
DATABASE_URL=sqlite://data/nzeza.db
DATABASE_MAX_CONNECTIONS=5
DATABASE_LOG_QUERIES=false
```

### Tables

- **positions** : Positions ouvertes et fermées
- **trades** : Historique complet des trades
- **audit_log** : Journal d'audit de tous les événements

### Récupération après Crash

Les positions sont automatiquement rechargées au redémarrage :
```bash
# Le système récupère automatiquement l'état depuis la DB
cargo run --release
```

## Dépannage

### L'application ne démarre pas

```bash
# Vérifier que les clés API sont ≥ 32 caractères
echo $API_KEYS | wc -c

# Régénérer si nécessaire
export API_KEYS=$(openssl rand -base64 32)
```

### Les ordres sont rejetés (dYdX)

C'est un problème connu avec dYdX v4. Solutions :
1. Utilisez Coinbase ou Binance à la place
2. Attendez l'implémentation Cosmos SDK
3. Implémentez vous-même l'intégration Cosmos

### Pas de signaux générés

```bash
# Vérifier le seuil de confiance
echo $MIN_CONFIDENCE_THRESHOLD

# Réduire le seuil pour plus de signaux
export MIN_CONFIDENCE_THRESHOLD=0.5
```

### Trop de trades

```bash
# Réduire les limites
export MAX_TRADES_PER_HOUR=5
export MAX_TRADES_PER_DAY=20

# Augmenter le seuil de confiance
export MIN_CONFIDENCE_THRESHOLD=0.8
```

## Support

- **Documentation de sécurité** : `SECURITY.md`
- **Correctifs prioritaires** : `PRIORITY_FIXES.md`
- **Configuration exemple** : `.env.example`
- **Issues GitHub** : Ouvrir un issue avec le tag `trading`

## Changelog

### Version Actuelle

- ✅ Trading automatique activé par défaut
- ✅ Support multi-exchange (Binance, Coinbase, dYdX, Hyperliquid, Kraken)
- ⚠️ dYdX activé avec warnings (problèmes connus)
- ✅ Persistence SQLite pour positions et trades
- ✅ Gestion sécurisée des clés API (32+ caractères requis)
- ✅ Support 1Password CLI
- ✅ Framework hardware wallet

### Prochaines Améliorations

- [ ] Intégration Cosmos SDK pour dYdX v4
- [ ] Backtesting framework
- [ ] Machine learning pour optimisation des stratégies
- [ ] Dashboard web de monitoring
- [ ] Notifications (email, Telegram, Discord)

---

**Date de mise à jour** : 2025-10-07
**Statut** : Trading automatique activé et prêt pour Coinbase/Binance
**dYdX Status** : ⚠️ Activé avec warnings - utiliser avec précaution

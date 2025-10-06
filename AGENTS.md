# AGENTS.md - Guide pour les Agents Développeurs

## Vue d'ensemble du projet
Ce projet implémente un serveur MPC (Multi-Party Computation) connecté à plusieurs échanges de crypto-monnaies : dydx (v4), hyperliquid, coinbase, binance, et kraken. L'architecture suit les principes DDD (Domain-Driven Design) avec des acteurs asynchrones utilisant des WebSockets pour des mises à jour de prix en temps réel.

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
# Design: dYdX Order Cancellation Implementation

## Overview
This design implements full order cancellation support for dYdX v4 by adding order metadata persistence and updating the cancellation logic to use stored metadata.

## Architecture

### Current Architecture
```
Order Placement Flow:
1. place_order() -> generates OrderId and good_until_block
2. Order placed on dYdX chain
3. Returns order_id string (for informational purposes only)

Order Cancellation Flow (Current - BROKEN):
1. cancel_order(order_id) -> ERROR: metadata not stored
2. Cannot reconstruct OrderId and good_until_block
3. User must cancel manually via web interface
```

### Proposed Architecture
```
Order Placement Flow:
1. place_order() -> generates OrderId and good_until_block
2. Store metadata in database (order_id -> {OrderId, good_until_block, client_id})
3. Order placed on dYdX chain
4. Return order_id string

Order Cancellation Flow (NEW - WORKING):
1. cancel_order(order_id) -> retrieve metadata from database
2. Reconstruct OrderId and good_until_block from stored data
3. Call dYdX cancellation API with proper parameters
4. Update database with cancellation status
```

## Database Schema Changes

### New Table: dydx_order_metadata
```sql
CREATE TABLE dydx_order_metadata (
    order_id TEXT PRIMARY KEY,           -- Our internal order ID string
    dydx_order_id TEXT NOT NULL,         -- Full OrderId protobuf encoded (base64)
    good_until_block INTEGER NOT NULL,   -- Block height when order expires
    client_id INTEGER NOT NULL,          -- Client ID used for order
    subaccount_number INTEGER NOT NULL,  -- Subaccount that placed the order
    symbol TEXT NOT NULL,                -- Trading pair (e.g., "BTC-USD")
    side TEXT NOT NULL,                  -- "buy" or "sell"
    quantity TEXT NOT NULL,              -- Order quantity as string
    price TEXT,                          -- Order price as string (null for market)
    order_type TEXT NOT NULL,            -- "market" or "limit"
    placed_at DATETIME NOT NULL,         -- When order was placed
    cancelled_at DATETIME,               -- When order was cancelled (null if active)
    tx_hash TEXT,                        -- dYdX transaction hash
    status TEXT NOT NULL DEFAULT 'active' -- 'active', 'cancelled', 'expired', 'filled'
);
```

### Integration with Existing Persistence Layer
- Extend `src/persistence/models.rs` with `DydxOrderMetadata` struct
- Add `DydxOrderMetadataRepository` in `src/persistence/repository.rs`
- Implement CRUD operations for metadata storage and retrieval

## Component Changes

### DydxV4Client Changes
```rust
impl DydxV4Client {
    // Existing method - enhanced to store metadata
    pub async fn place_order(&self, order: &Order, metadata_repo: &DydxOrderMetadataRepository) -> Result<String, String> {
        // ... existing order building logic ...

        // After successful placement, store metadata
        let metadata = DydxOrderMetadata {
            order_id: generated_order_id.clone(),
            dydx_order_id: base64_encoded_order_id,
            good_until_block: good_until_block.height(),
            // ... other fields ...
        };
        metadata_repo.create(&metadata).await?;

        Ok(generated_order_id)
    }

    // Existing method - updated to use stored metadata
    pub async fn cancel_order(&self, order_id: &str, metadata_repo: &DydxOrderMetadataRepository) -> Result<(), String> {
        // Retrieve metadata
        let metadata = metadata_repo.get_by_order_id(order_id).await?
            .ok_or_else(|| format!("Order metadata not found for: {}", order_id))?;

        // Check if already cancelled/expired
        if metadata.status != "active" {
            return Err(format!("Order {} is already {}", order_id, metadata.status));
        }

        // Reconstruct OrderId from stored data
        let order_id_struct = reconstruct_order_id(&metadata.dydx_order_id)?;

        // Check if order has expired
        let current_block = self.node_client.latest_block_height().await?;
        if current_block >= metadata.good_until_block {
            metadata_repo.update_status(order_id, "expired").await?;
            return Err(format!("Order {} has expired at block {}", order_id, metadata.good_until_block));
        }

        // Perform cancellation
        let cancel_tx = self.node_client.cancel_order(&mut *self.account.lock().await, order_id_struct).await?;

        // Update metadata
        metadata_repo.update_cancelled(order_id, &cancel_tx).await?;

        Ok(())
    }
}
```

### ExchangeActor Integration
```rust
impl ExchangeActor {
    async fn place_order_dydx(order: &Order, client: &DydxV4Client, metadata_repo: &DydxOrderMetadataRepository) -> Result<String, String> {
        client.place_order(order, metadata_repo).await
    }

    async fn cancel_order_dydx(order_id: &str, client: &DydxV4Client, metadata_repo: &DydxOrderMetadataRepository) -> Result<(), String> {
        client.cancel_order(order_id, metadata_repo).await
    }
}
```

## Error Handling Strategy

### Cancellation Errors
- **MetadataNotFound**: Order was placed before this feature or metadata corrupted
- **OrderExpired**: Order reached good_until_block and can no longer be cancelled
- **AlreadyCancelled**: Order was previously cancelled
- **AlreadyFilled**: Order was filled and cannot be cancelled
- **NetworkError**: dYdX API/network issues during cancellation
- **InvalidMetadata**: Stored metadata is corrupted or invalid

### Recovery Mechanisms
- Graceful degradation: If metadata missing, provide clear error with manual cancellation instructions
- Automatic cleanup: Background task to mark expired orders
- Audit logging: All cancellation attempts logged for debugging

## Security Considerations

### Data Protection
- Order metadata contains sensitive trading information
- Store in encrypted database with proper access controls
- Implement data retention policies for old order metadata

### API Security
- Cancellation requests validated against stored metadata
- Prevent cancellation of orders not owned by the client
- Rate limiting on cancellation requests

## Performance Considerations

### Database Impact
- Additional write on every order placement
- Additional read on every cancellation attempt
- Index on order_id for fast lookups
- Consider partitioning for high-volume trading

### Memory Usage
- Metadata structs are small and temporary
- No significant increase in memory footprint
- Database connection pooling for metadata operations

## Testing Strategy

### Unit Tests
- Test metadata storage and retrieval
- Test OrderId reconstruction from stored data
- Test cancellation validation logic
- Test error handling for various failure scenarios

### Integration Tests
- End-to-end order placement and cancellation
- Test with actual dYdX testnet
- Verify metadata persistence across restarts
- Test concurrent cancellation attempts

### Edge Cases
- Cancellation of expired orders
- Cancellation of already-filled orders
- Network failures during cancellation
- Database corruption scenarios

## Migration Strategy

### Backward Compatibility
- Existing orders without metadata cannot be cancelled programmatically
- Clear error messages guide users to manual cancellation
- No breaking changes to existing APIs

### Data Migration
- No migration needed for existing data
- New orders automatically get metadata stored
- Optional: Background job to populate metadata for recent orders (if feasible)

## Rollback Plan

### Feature Flags
- Implement behind feature flag for gradual rollout
- Easy rollback by disabling metadata storage
- Existing cancellation behavior preserved when disabled

### Monitoring
- Track cancellation success/failure rates
- Alert on metadata storage failures
- Monitor database performance impact</content>
<parameter name="filePath">openspec/changes/implement-dydx-order-cancellation/design.md
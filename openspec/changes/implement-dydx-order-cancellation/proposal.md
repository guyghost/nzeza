# Implement dYdX Order Cancellation

## Why
The dYdX v4 integration currently has limited order cancellation support. While orders can be placed successfully, cancellation requires storing order metadata (OrderId and good_until_block) that is not currently persisted. This prevents users from canceling orders programmatically, forcing them to use the dYdX web interface or CLI tools for manual cancellation.

## What Changes
This change implements full order cancellation support for dYdX v4 by:

- Adding order metadata persistence to track OrderId and good_until_block for each placed order
- Updating the DydxV4Client to support proper order cancellation using stored metadata
- Integrating with the existing persistence layer for reliable metadata storage
- Adding comprehensive error handling and validation for cancellation operations

## Problem
The current dYdX v4 client implementation places orders successfully but cannot cancel them because:

1. **Missing Metadata Storage**: Order cancellation in dYdX v4 requires the original OrderId and good_until_block height from order placement
2. **No Persistence**: Order metadata is not stored in the database after placement
3. **Limited API**: The current cancel_order method returns an error explaining the limitation
4. **User Experience**: Users cannot programmatically cancel dYdX orders, reducing system usability

## Root Cause Analysis
After code review, the cancellation limitation stems from dYdX v4's protocol requirements:

1. **Protocol Requirements**: dYdX v4 requires exact OrderId and good_until_block for cancellation
2. **OrderId Complexity**: OrderId is a complex structure containing client_id, order_flags, and other fields
3. **Block Height Dependency**: Orders expire at good_until_block, requiring height tracking
4. **No Metadata Persistence**: Current implementation doesn't store required cancellation data

## Proposed Solution
Implement comprehensive order metadata persistence and cancellation support:

- Store order metadata (OrderId, good_until_block, client_id) in database after placement
- Update DydxV4Client::cancel_order to retrieve metadata and perform actual cancellation
- Add database schema extensions for order metadata storage
- Implement proper error handling for cancellation failures
- Add validation to ensure orders can only be cancelled before expiration

## Requirements
### Order Metadata Persistence
The system SHALL store order metadata including OrderId, good_until_block, and client_id for all dYdX orders.

### Cancellation Support
The system SHALL support programmatic order cancellation for dYdX v4 using stored metadata.

### Metadata Retrieval
The system SHALL retrieve stored order metadata when cancel_order is called.

### Error Handling
The system SHALL provide clear error messages for cancellation failures and expired orders.

### Data Integrity
The system SHALL ensure order metadata is stored atomically with order placement.

## Impact
- Users can programmatically cancel dYdX orders
- Improved system completeness and usability
- Better integration with trading strategies requiring order management
- Enhanced error handling and user feedback

## Risk Assessment
- Low risk: Changes are additive and don't affect existing order placement
- Database schema changes are backward compatible
- Cancellation failures are handled gracefully with clear error messages
- No impact on other exchange integrations</content>
<parameter name="filePath">openspec/changes/implement-dydx-order-cancellation/proposal.md
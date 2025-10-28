# Order Cancellation

## ADDED Requirements

### Store Order Metadata on Placement
#### Scenario: Store Order Metadata on Placement
**Given** a dYdX v4 order is being placed
**When** the order is successfully submitted to the dYdX chain
**Then** the system SHALL store order metadata including OrderId, good_until_block, client_id, and subaccount information in the database

### Cancel Active Order
#### Scenario: Cancel Active Order
**Given** a dYdX v4 order is active (not filled, cancelled, or expired)
**When** cancel_order is called with the order ID
**Then** the system SHALL retrieve stored metadata and submit a cancellation transaction to dYdX

### Prevent Cancellation of Expired Orders
#### Scenario: Prevent Cancellation of Expired Orders
**Given** a dYdX v4 order has reached its good_until_block
**When** cancel_order is called
**Then** the system SHALL return an error indicating the order has expired and update the order status to 'expired'

### Handle Missing Metadata
#### Scenario: Handle Missing Metadata
**Given** an order was placed before metadata storage was implemented
**When** cancel_order is called
**Then** the system SHALL return a clear error message explaining manual cancellation is required via dYdX interface

### Update Status on Successful Cancellation
#### Scenario: Update Status on Successful Cancellation
**Given** a cancellation transaction is successfully submitted to dYdX
**When** the transaction is confirmed
**Then** the system SHALL update the order metadata status to 'cancelled' and record the cancellation timestamp

### Validate Cancellation Permissions
#### Scenario: Validate Cancellation Permissions
**Given** a cancellation request for a dYdX order
**When** the request is processed
**Then** the system SHALL verify the order belongs to the authenticated subaccount before proceeding

## MODIFIED Requirements

### Enhanced Error Messages
#### Scenario: Enhanced Error Messages
**Given** any cancellation operation fails
**When** the error is returned to the client
**Then** the system SHALL provide detailed error messages including the order ID, failure reason, and suggested remediation steps</content>
<parameter name="filePath">openspec/changes/implement-dydx-order-cancellation/specs/order-cancellation/spec.md
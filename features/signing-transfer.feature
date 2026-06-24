Feature: Step-by-step FROST signing and Solana transfer
  The reviewer can create a transfer request, trigger each signing round manually,
  aggregate the signature shares, broadcast the transaction, and inspect confirmation status.

  Background:
    Given the DKG session is "COMPLETED"
    And wallet index 0 exists
    And wallet index 0 has Devnet SOL for fees and transfer amount

  Scenario: Create a pending signing request
    When I create a transfer request from wallet index 0
    Then a signing request should be created
    And the signing request status should be "PENDING"
    And the pending request list should show the sender, recipient, amount, and created time

  Scenario: Complete signing rounds with independent node triggers
    Given a signing request is "PENDING"
    When I trigger Signing Round 1 for Node A
    Then Node A Signing Round 1 should be "COMPLETED"
    And Node B Signing Round 1 should be "NOT_STARTED"
    When I trigger Signing Round 1 for Node B
    Then the signing request status should be "COMMITMENTS_READY"
    When I trigger Signing Round 2 for Node A
    And I trigger Signing Round 2 for Node B
    Then the signing request status should be "READY_TO_AGGREGATE"

  Scenario: Reject Signing Round 2 before both commitments exist
    Given a signing request is "PENDING"
    And Node B Signing Round 1 is "NOT_STARTED"
    When I trigger Signing Round 2 for Node A
    Then the coordinator should reject the request
    And no nonce should be consumed for Round 2

  Scenario: Reject a signing request for an unknown wallet
    When I create a transfer request from wallet index 99
    Then the coordinator should reject the request
    And no signing request should be created

  Scenario: Re-trigger Signing Round 1 without creating a second nonce
    Given a signing request is "PENDING"
    And Node A Signing Round 1 is "COMPLETED"
    When I trigger Signing Round 1 for Node A again
    Then Node A Signing Round 1 should still be "COMPLETED"
    And the original public nonce commitment should be returned

  Scenario: Reject Signing Round 2 nonce reuse
    Given Node A Signing Round 2 is "COMPLETED"
    When I trigger Signing Round 2 for Node A again
    Then the coordinator should reject the request
    And Node A should not reuse the consumed nonce

  Scenario: Distinguish multiple pending signing requests
    Given wallet index 0 exists
    When I create two transfer requests from wallet index 0
    Then both signing requests should appear in the request list
    And each request should have its own request id and node step statuses

  Scenario: Broadcast and confirm a transfer
    Given a signing request is "READY_TO_AGGREGATE"
    When I aggregate and broadcast the signing request
    Then the signing request status should become "BROADCASTED"
    And the frontend should display a Solana Explorer transaction link
    When Solana reports the transaction as confirmed
    Then the signing request status should become "CONFIRMED"

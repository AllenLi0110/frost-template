Feature: Step-by-step DKG flow
  The reviewer can manually trigger each FROST DKG round for Node A and Node B
  so the protocol is observable instead of hidden behind a single action.

  Background:
    Given the coordinator is running
    And TSS Node A is running
    And TSS Node B is running
    And PostgreSQL is available

  Scenario: Complete DKG with independent node round triggers
    Given no completed DKG session exists
    When I create a DKG session
    Then the DKG session status should be "NOT_STARTED"
    When I trigger DKG Round 1 for Node A
    Then Node A DKG Round 1 should be "COMPLETED"
    And Node B DKG Round 1 should be "NOT_STARTED"
    When I trigger DKG Round 1 for Node B
    Then the DKG session status should be "ROUND_1_COMPLETE"
    When I trigger DKG Round 2 for Node A
    And I trigger DKG Round 2 for Node B
    Then the DKG session status should be "ROUND_2_COMPLETE"
    When I trigger DKG Round 3 for Node A
    And I trigger DKG Round 3 for Node B
    Then the DKG session status should be "COMPLETED"
    And the coordinator should display a master public key in Base58 format

  Scenario: View the DKG control surface
    Given the frontend is running
    When I open the DKG control surface
    Then I should see the active DKG session status
    And I should see independent controls for Node A Round 1 through Round 3
    And I should see independent controls for Node B Round 1 through Round 3
    And I should not see a "Run All" shortcut

  Scenario: Reject out-of-order DKG rounds
    Given a DKG session exists
    And Node B DKG Round 1 is "NOT_STARTED"
    When I trigger DKG Round 2 for Node A
    Then the coordinator should reject the request
    And the DKG session should not advance to "ROUND_2_IN_PROGRESS"

  Scenario: Re-trigger a completed DKG round
    Given Node A DKG Round 1 is "COMPLETED"
    When I trigger DKG Round 1 for Node A again
    Then Node A DKG Round 1 should still be "COMPLETED"
    And the coordinator should return the original public round result

  Scenario: DKG state survives restart
    Given a DKG session is "COMPLETED"
    When the coordinator restarts
    Then the active DKG session should still be "COMPLETED"
    And the master public key should still be available

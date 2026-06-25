Feature: Reviewer experience and handoff
  The reviewer can start the full stack, fund a derived Devnet wallet,
  run the manual acceptance flow, and inspect the AI collaboration evidence.

  Scenario: Start the full stack with one Docker Compose command
    When the reviewer runs "docker compose up -d"
    Then PostgreSQL should be healthy
    And the coordinator should expose a health endpoint
    And TSS Node A should expose a health endpoint
    And TSS Node B should expose a health endpoint
    And the frontend should be available at "http://localhost:3000"

  Scenario: Locate and fund a derived wallet
    Given DKG has completed
    When the reviewer creates wallet index 0
    Then the frontend should display the wallet index and Solana address
    And the README should explain how to fund that address with Devnet SOL

  Scenario: Complete the manual acceptance checklist
    Given wallet index 0 has Devnet SOL
    And a recipient Devnet wallet address is available
    When the reviewer creates a signing request
    And independently runs both signing rounds for Node A and Node B
    And broadcasts and refreshes confirmation
    Then the signing request should become "CONFIRMED"
    And the frontend should link to the Solana Explorer transaction

  Scenario: Inspect AI collaboration evidence
    When the reviewer opens the AI-native documentation
    Then they should find the implementation roadmap
    And they should find prompt files for each phase
    And they should find collaboration logs, decision logs, and verification evidence

Feature: Wallet derivation
  The reviewer can derive multiple Solana Devnet wallet addresses from one completed DKG
  without requiring additional node-to-node interaction.

  Background:
    Given the DKG session is "COMPLETED"
    And the coordinator has a master public key

  Scenario: Create wallets with sequential indexes
    Given no derived wallets exist
    When I create a wallet
    Then wallet index 0 should be created
    And the wallet should have a Solana address in Base58 format
    When I create another wallet
    Then wallet index 1 should be created
    And wallet index 0 should still be listed

  Scenario: Derived wallets persist after restart
    Given wallet index 0 exists
    And wallet index 1 exists
    When the coordinator restarts
    Then wallet index 0 should still be listed
    And wallet index 1 should still be listed
    When I create another wallet
    Then wallet index 2 should be created

  Scenario: Display wallet balance from Solana Devnet
    Given wallet index 0 exists
    When I refresh the balance for wallet index 0
    Then the frontend should display the balance in lamports or SOL

  Scenario: Select a derived wallet as transfer sender
    Given wallet index 0 exists
    When I select wallet index 0 as sender
    Then the transfer form should use wallet index 0 as the sender wallet


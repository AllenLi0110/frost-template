Feature: Wallet derivation
  The reviewer can derive multiple Solana Devnet wallet addresses from one completed DKG
  without requiring additional node-to-node interaction.

  Scenario: Reject wallet creation before DKG completes
    Given the DKG session is "ROUND_2_COMPLETE"
    When I create a wallet
    Then the request should be rejected because wallet derivation requires a completed DKG

  Scenario: Create wallets with sequential indexes
    Given the DKG session is "COMPLETED"
    And the coordinator has a master public key
    And no derived wallets exist
    When I create a wallet
    Then wallet index 0 should be created
    And the wallet should have a Solana address in Base58 format
    When I create another wallet
    Then wallet index 1 should be created
    And wallet index 0 should still be listed

  Scenario: Derived wallets persist after restart
    Given the DKG session is "COMPLETED"
    And the coordinator has a master public key
    And wallet index 0 exists
    And wallet index 1 exists
    When the coordinator restarts
    Then wallet index 0 should still be listed
    And wallet index 1 should still be listed
    When I create another wallet
    Then wallet index 2 should be created

  Scenario: Display wallet balance from Solana Devnet
    Given the DKG session is "COMPLETED"
    And the coordinator has a master public key
    And wallet index 0 exists
    When I refresh the balance for wallet index 0
    Then the frontend should display the balance in lamports or SOL

  Scenario: Refresh all visible vault balances
    Given the DKG session is "COMPLETED"
    And wallet index 0 exists
    And wallet index 1 exists
    When I refresh the Vault Watch balances
    Then the frontend should refresh the balance for wallet index 0 from Solana Devnet
    And the frontend should refresh the balance for wallet index 1 from Solana Devnet

  Scenario: Refresh balances after a transfer changes vault funds
    Given a signing request is "BROADCASTED"
    And wallet index 0 exists as the sender vault
    And wallet index 1 exists as the recipient vault
    When I refresh the transfer confirmation
    Then Vault Watch should show the latest sender balance from Solana Devnet
    And Vault Watch should show the latest recipient balance from Solana Devnet

  Scenario: Balance lookup failure is visible without breaking wallet listing
    Given the DKG session is "COMPLETED"
    And the coordinator has a master public key
    And wallet index 0 exists
    And the Solana RPC endpoint is unavailable
    When I refresh the balance for wallet index 0
    Then wallet index 0 should still be listed
    And the balance status should be "UNAVAILABLE"

  Scenario: Select a derived wallet as transfer sender
    Given the DKG session is "COMPLETED"
    And the coordinator has a master public key
    And wallet index 0 exists
    When I select wallet index 0 as sender
    Then the transfer form should use wallet index 0 as the sender wallet

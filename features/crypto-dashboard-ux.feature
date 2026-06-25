Feature: Crypto dashboard UX
  The reviewer can understand the project as an MPC Solana wallet workflow
  without losing access to the manual protocol controls.

  Scenario: See the wallet workflow first
    When the reviewer opens the frontend
    Then the page should present "MPC Wallet Dashboard"
    And the page should show the Solana Devnet network
    And the page should show a five-step workflow from key ceremony to broadcast
    And the current workflow step should be visually active for a mobile demo

  Scenario: Run the key ceremony manually
    Given no product protocol behavior has changed
    When the reviewer starts the key ceremony
    Then Node A and Node B DKG rounds 1, 2, and 3 should remain independently clickable
    And the page should explain that private shares stay node-local

  Scenario: Review derived vaults
    Given key ceremony has completed
    When the reviewer creates derived wallets
    Then the wallet section should present them as derived vaults
    And each vault should show address, derivation path, balance status, and sender selection

  Scenario: Complete a transfer ticket
    Given a derived vault is funded on Devnet
    When the reviewer creates a transfer ticket
    Then the signing section should show signer commitment and signature-share collection
    And broadcast plus confirmation should still expose a Solana Explorer receipt

  Scenario: Review on a mobile viewport
    When the reviewer opens the frontend on a phone-sized screen
    Then the page should keep the workflow, vaults, transfer ticket, and receipt in one vertical flow
    And the active step animation should not block manual protocol controls

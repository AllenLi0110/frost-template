Feature: Project foundation services
  The reviewer can start the full local stack and verify each service is reachable
  before any DKG, wallet derivation, or signing behavior is implemented.

  Background:
    Given Docker Compose is available
    And PostgreSQL configuration is present

  Scenario: Start the foundation stack
    When I start the Docker Compose stack
    Then PostgreSQL should be healthy
    And the coordinator should expose a health endpoint
    And TSS Node A should expose a health endpoint
    And TSS Node B should expose a health endpoint
    And the frontend should be reachable

  Scenario: Coordinator checks node reachability
    Given the coordinator is running
    And TSS Node A is running
    And TSS Node B is running
    When I request coordinator node health
    Then Node A should be reported as reachable
    And Node B should be reported as reachable

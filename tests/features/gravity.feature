Feature: DBSP gravity integration
  Scenario: Unsupported entity falls
    Given a headless app with a single unsupported entity
    When the simulation ticks once
    Then the entity's z position should be 0.0

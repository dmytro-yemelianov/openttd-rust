use pretty_assertions::assert_eq;
use transport_cli::app::GameApp;
use transport_cli::render_terminal_viewport;
use transport_scenario::ScenarioDefinition;

#[test]
fn test_cli_game_app_lifecycle_and_rendering() {
    let scenario = ScenarioDefinition::default();
    let mut app = GameApp::new(scenario);

    // Initial tick
    assert_eq!(app.world.tick.0, 0);

    // Step 50 ticks
    app.run_ticks(50);
    assert_eq!(app.world.tick.0, 50);

    // Render full screen
    let screen = app.render_screen();
    assert!(screen.contains("NETWORK TELEMETRY DASHBOARD"));
    assert!(screen.contains("SCENARIO: Coastal Commuter Connection"));
    assert!(screen.contains("Viewport"));

    // Render custom terminal viewport
    let view = render_terminal_viewport(&app.world, &app.camera, 20, 10, Some(app.cursor));
    assert!(view.contains("Legend"));
}

use pretty_assertions::assert_eq;
use transport_wasm::WasmGameSession;

#[test]
fn test_wasm_game_session_initialization_and_render() {
    let mut session = WasmGameSession::new(320, 240, 42);

    assert_eq!(session.framebuffer.width, 320);
    assert_eq!(session.framebuffer.height, 240);
    assert_eq!(session.framebuffer.pixels.len(), 320 * 240);

    // Initial tick
    session.tick();
    assert_eq!(session.world.tick.0, 1);

    // Render frame
    let pixels = session.render(0.5);
    assert_eq!(pixels.len(), 320 * 240);
    assert_eq!(session.pixel_buffer_len(), 320 * 240 * 4);

    // Camera pan and zoom
    session.pan(10.0, -5.0);
    session.zoom(1.2);

    // Export telemetry JSON
    let json = session.telemetry_json();
    assert!(json.contains("latest_telemetry"));
}

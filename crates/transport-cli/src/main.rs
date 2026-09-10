use std::env;
use std::thread::sleep;
use std::time::Duration;
use transport_cli::app::GameApp;
use transport_dashboard::{export_json, export_prometheus};
use transport_scenario::ScenarioDefinition;

fn print_help() {
    println!("openttd-rust CLI Simulation Runner");
    println!("Usage:");
    println!("  transport-cli [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --ticks <N>      Run simulation headlessly for N ticks and display report");
    println!("  --demo           Run live simulation demo (30 ticks with animated screen refresh)");
    println!("  --json           Run for 100 ticks and export state as JSON");
    println!("  --prometheus     Run for 100 ticks and export metrics in Prometheus format");
    println!("  --help, -h       Show this help message");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut ticks = 100;
    let mut is_demo = false;
    let mut export_mode_json = false;
    let mut export_mode_prom = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--demo" => {
                is_demo = true;
                i += 1;
            }
            "--json" => {
                export_mode_json = true;
                i += 1;
            }
            "--prometheus" => {
                export_mode_prom = true;
                i += 1;
            }
            "--ticks" => {
                if i + 1 < args.len() {
                    ticks = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    eprintln!("Error: --ticks requires a positive integer argument");
                    return;
                }
            }
            other => {
                eprintln!("Unknown option: {other}");
                print_help();
                return;
            }
        }
    }

    let scenario = ScenarioDefinition::default();
    let mut app = GameApp::new(scenario);

    if export_mode_json {
        app.run_ticks(ticks);
        match export_json(&app.telemetry, &app.ledger) {
            Ok(json_str) => println!("{json_str}"),
            Err(err) => eprintln!("JSON export error: {err}"),
        }
        return;
    }

    if export_mode_prom {
        app.run_ticks(ticks);
        let prom_str = export_prometheus(&app.telemetry, &app.ledger);
        println!("{prom_str}");
        return;
    }

    if is_demo {
        println!("\x1b[2J\x1b[H"); // Clear screen
        println!("Starting openttd-rust Live Simulation Demo...");
        for _ in 0..30 {
            app.step_tick();
            print!("\x1b[H"); // Move cursor to home
            println!("{}", app.render_screen());
            sleep(Duration::from_millis(50));
        }
        println!("\nDemo finished successfully.");
        return;
    }

    // Default: run ticks and render final frame
    app.run_ticks(ticks);
    println!("{}", app.render_screen());
}

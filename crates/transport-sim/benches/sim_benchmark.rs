use std::time::Instant;
use transport_sim::{Command, Simulator};
use transport_types::enum_::{OrderType, VehicleKind};
use transport_types::unit::Money;
use transport_types::{CompanyID, EngineID, OrderListID, StationID, TileIndex, VehicleID};
use transport_world::map::MapSize;

fn setup_benchmark_world(map_dim: u16, fleet_size: usize) -> Simulator {
    let mut sim = Simulator::new(MapSize::new(map_dim, map_dim));

    // Create benchmark company
    let _ = sim.process_command(Command::CreateCompany {
        name: "Benchmark Logistics".to_string(),
        money: Money(100_000_000),
        color: 0x0000FF,
    });
    let company_id = CompanyID(0);

    // Build two stations across the map
    let dock_a_pos = TileIndex::new(2, 2);
    let dock_b_pos = TileIndex::new(map_dim - 3, 2);

    let _ = sim.process_command(Command::BuildStation {
        company_id,
        name: "Origin".to_string(),
        tiles: vec![dock_a_pos],
    });
    let _ = sim.process_command(Command::BuildStation {
        company_id,
        name: "Destination".to_string(),
        tiles: vec![dock_b_pos],
    });

    // Create schedule
    let _ = sim.process_command(Command::CreateOrderList {});
    let order_list_id = OrderListID(0);
    let _ = sim.process_command(Command::AddOrderToList {
        order_list_id,
        order: OrderType::GoToStation {
            station_id: StationID(0),
            conditions: None,
        },
    });
    let _ = sim.process_command(Command::AddOrderToList {
        order_list_id,
        order: OrderType::GoToStation {
            station_id: StationID(1),
            conditions: None,
        },
    });

    // Populate fleet of ships
    for i in 0..fleet_size {
        let x = 2 + ((i as u16) % (map_dim - 6));
        let y = 2 + ((i as u16 / (map_dim - 6)) % (map_dim - 6));
        let vehicle_id = VehicleID(i as u32);
        let vehicle = transport_world::entities::Vehicle::new(
            vehicle_id,
            company_id,
            EngineID(1),
            VehicleKind::Ship,
            TileIndex::new(x, y),
            order_list_id,
        );
        sim.world.vehicles.insert(vehicle_id, vehicle);
    }

    sim
}

fn benchmark_scenario(name: &str, map_dim: u16, fleet_size: usize, ticks: usize) {
    let mut sim = setup_benchmark_world(map_dim, fleet_size);

    // Warm-up (10 ticks to stabilize buffers and steady-state)
    for _ in 0..10 {
        sim.tick();
    }

    let start = Instant::now();
    for _ in 0..ticks {
        sim.tick();
    }
    let duration = start.elapsed();

    let total_micros = duration.as_micros() as f64;
    let micros_per_tick = total_micros / (ticks as f64);
    let ticks_per_sec = (ticks as f64) / duration.as_secs_f64();

    println!(
        "{name:<20} | Map: {map_dim:>3}x{map_dim:<3} | Fleet: {fleet_size:>4} | Ticks: {ticks:>4} | {micros_per_tick:>8.2} µs/tick | {ticks_per_sec:>10.1} ticks/s"
    );
}

fn main() {
    println!("=== OpenTTD Rust Simulation Performance Benchmark ===");
    println!(
        "Tile Memory Footprint: {} bytes (Base: {} bytes, Ext: {} bytes)",
        std::mem::size_of::<transport_world::map::Tile>(),
        std::mem::size_of::<transport_world::map::TileBase>(),
        std::mem::size_of::<transport_world::map::TileExtension>()
    );
    println!("---------------------------------------------------------------------------------------------");

    let scenarios = [
        // Small fleet on small map (64x64)
        ("Small Fleet (64)", 64, 10, 500),
        // Medium fleet on small map
        ("Medium Fleet (64)", 64, 100, 500),
        // Large fleet on small map
        ("Large Fleet (64)", 64, 1000, 200),
        // Small fleet on medium map (256x256)
        ("Small Fleet (256)", 256, 10, 200),
        // Medium fleet on medium map
        ("Medium Fleet (256)", 256, 100, 200),
        // Large fleet on medium map (1,000 ships)
        ("Large Fleet (256)", 256, 1000, 100),
    ];

    for (name, map_dim, fleet_size, ticks) in scenarios {
        benchmark_scenario(name, map_dim, fleet_size, ticks);
    }
    println!("=============================================================================================");
}

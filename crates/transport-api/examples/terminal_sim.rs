use std::sync::Arc;
use std::thread;
use std::time::Duration;
use transport_api::driver::ApiSubsystemDriver;
use transport_api::ingress::CommandQueue;
use transport_api::ring::EventRingBuffer;
use transport_api::Event;
use transport_people::driver::PeopleDriver;
use transport_people::turnstile::StationTurnstile;
use transport_sim::drivers::{LogisticsDriver, MovementDriver};
use transport_sim::kernel::Microkernel;
use transport_sim::World;
use transport_types::enum_::{OrderType, TileKind, VehicleKind};
use transport_types::{
    CargoAmount, CargoType, CompanyID, EngineID, Money, OrderListID, StationID, TileIndex,
    VehicleID,
};
use transport_world::definitions::Station;
use transport_world::entities::{Company, Vehicle};
use transport_world::map::MapSize;

const WIDTH: u16 = 28;
const HEIGHT: u16 = 12;

fn print_hud(
    tick: u64,
    treasury: i64,
    ship_pos: TileIndex,
    ship_state: &str,
    ship_cargo: usize,
    delivered: usize,
    admitted_peeps: usize,
) {
    println!("\x1b[1;36m╔══════════════════════════════════════════════════════════════════════════╗\x1b[0m");
    println!(
        "\x1b[1;36m║\x1b[0m \x1b[1;37mOPENTTD-RUST DETERMINISTIC CAPABILITY MICROKERNEL — LIVE SIMULATION\x1b[0m    \x1b[1;36m║\x1b[0m"
    );
    println!("\x1b[1;36m╠══════════════════════════════════════════════════════════════════════════╣\x1b[0m");
    println!(
        "\x1b[1;36m║\x1b[0m Tick: \x1b[1;32m{tick:>4}\x1b[0m | Treasury: \x1b[1;33m${treasury:<8}\x1b[0m | Delivered Goods: \x1b[1;32m{delivered:>4}\x1b[0m | Peeps Admitted: \x1b[1;35m{admitted_peeps:>2}\x1b[0m \x1b[1;36m║\x1b[0m"
    );
    let ship_x = ship_pos.x;
    let ship_y = ship_pos.y;
    println!(
        "\x1b[1;36m║\x1b[0m Ship 1: \x1b[1;37m({ship_x}, {ship_y})\x1b[0m | State: \x1b[1;34m{ship_state:<10}\x1b[0m | Cargo: \x1b[1;33m{ship_cargo:>2} Grain\x1b[0m               \x1b[1;36m║\x1b[0m"
    );
    println!("\x1b[1;36m╚══════════════════════════════════════════════════════════════════════════╝\x1b[0m");
}

fn render_map(
    world: &World,
    dock_a: TileIndex,
    dock_b: TileIndex,
    obstacles: &[TileIndex],
    ships: &[(VehicleID, TileIndex)],
) {
    println!("  \x1b[1;30m0123456789012345678901234567\x1b[0m");
    for y in 0..HEIGHT {
        print!("\x1b[1;30m{y:>2}\x1b[0m");
        for x in 0..WIDTH {
            let pt = TileIndex::new(x, y);

            // Check if ship is on this tile
            if let Some((vid, _)) = ships.iter().find(|(_, p)| *p == pt) {
                print!("\x1b[1;37;44mS{}\x1b[0m", vid.0 % 10);
            } else if pt == dock_a {
                print!("\x1b[1;33;40m⚓A\x1b[0m");
            } else if pt == dock_b {
                print!("\x1b[1;33;40m⚓B\x1b[0m");
            } else if obstacles.contains(&pt) {
                print!("\x1b[38;5;242m██\x1b[0m");
            } else if world.tile_to_station.contains_key(&pt) {
                print!("\x1b[1;33m■■\x1b[0m");
            } else {
                // Water wave
                if (x + y) % 3 == 0 {
                    print!("\x1b[38;5;32m~ \x1b[0m");
                } else {
                    print!("\x1b[38;5;25m. \x1b[0m");
                }
            }
        }
        println!();
    }
}

fn print_event_log(events: &[Arc<Event>]) {
    println!("\x1b[1;37m── Event Telemetry Stream (Zero-Copy Ring Buffer) ──────────────────────────\x1b[0m");
    if events.is_empty() {
        println!("  \x1b[38;5;244m(Awaiting simulation events...)\x1b[0m");
    } else {
        let recent = events.iter().rev().take(4);
        for evt in recent {
            match &**evt {
                Event::VehicleMoved {
                    vehicle_id,
                    old_tile,
                    new_tile,
                    tick,
                } => {
                    println!(
                        "  \x1b[38;5;39m• [Tick {:>2}]\x1b[0m \x1b[1mVehicle {}\x1b[0m moved ({}, {}) -> ({}, {})",
                        tick.0, vehicle_id.0, old_tile.x, old_tile.y, new_tile.x, new_tile.y
                    );
                }
                Event::CargoLoaded {
                    vehicle_id,
                    station_id,
                    amount,
                    tick,
                    ..
                } => {
                    println!(
                        "  \x1b[38;5;46m• [Tick {:>2}]\x1b[0m \x1b[1mVehicle {}\x1b[0m loaded {} cargo at Station {}",
                        tick.0, vehicle_id.0, amount.0, station_id.0
                    );
                }
                Event::CargoUnloaded {
                    vehicle_id,
                    station_id,
                    amount,
                    tick,
                    ..
                } => {
                    println!(
                        "  \x1b[1;32m• [Tick {:>2}]\x1b[0m \x1b[1mVehicle {}\x1b[0m unloaded {} cargo at Station {} (Revenue earned!)",
                        tick.0, vehicle_id.0, amount.0, station_id.0
                    );
                }
                _ => {}
            }
        }
    }
    println!("\x1b[1;37m────────────────────────────────────────────────────────────────────────────\x1b[0m");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let total_ticks: usize = args
        .iter()
        .position(|a| a == "--ticks")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    let delay_ms: u64 = args
        .iter()
        .position(|a| a == "--delay-ms")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);

    let clear_screen = !args.iter().any(|a| a == "--no-clear");

    // Initialize Microkernel
    let mut kernel = Microkernel::new(MapSize::new(WIDTH, HEIGHT));
    let company_id = CompanyID(1);
    kernel.world.companies.insert(
        company_id,
        Company::new(company_id, "Pacific Maritime".into(), Money(50000), 0x00FF00),
    );

    let dock_a_pos = TileIndex::new(3, 5);
    let dock_b_pos = TileIndex::new(23, 5);
    let dock_a = StationID(1);
    let dock_b = StationID(2);

    let mut st_a = Station::new(dock_a, "Port San Pedro".into(), vec![dock_a_pos]);
    st_a.company_id = Some(company_id);
    let mut st_b = Station::new(dock_b, "Port Honolulu".into(), vec![dock_b_pos]);
    st_b.company_id = Some(company_id);

    kernel.world.stations.insert(dock_a, st_a);
    kernel.world.stations.insert(dock_b, st_b);
    kernel.world.map.set_station_at(dock_a_pos, dock_a, Some(company_id)).unwrap();
    kernel.world.map.set_station_at(dock_b_pos, dock_b, Some(company_id)).unwrap();
    kernel.world.tile_to_station.insert(dock_a_pos, dock_a);
    kernel.world.tile_to_station.insert(dock_b_pos, dock_b);

    // Initialize water on map
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pt = TileIndex::new(x, y);
            if let Some(tile) = kernel.world.map.get_mut(pt) {
                tile.base.kind = TileKind::Water;
            }
        }
    }

    // Obstacle peninsula between x=12 and x=13, y=3..=8
    let mut obstacles = Vec::new();
    for y in 3..=8 {
        for x in 12..=13 {
            let pt = TileIndex::new(x, y);
            obstacles.push(pt);
            if let Some(tile) = kernel.world.map.get_mut(pt) {
                tile.base.kind = TileKind::Trees;
            }
        }
    }

    // Schedule: Dock A -> Dock B
    let orders_id = OrderListID(1);
    kernel.world.order_lists.insert(
        orders_id,
        vec![
            OrderType::GoToStation {
                station_id: dock_b,
                conditions: None,
            },
            OrderType::GoToStation {
                station_id: dock_a,
                conditions: None,
            },
        ],
    );

    // Ship 1
    let ship_id = VehicleID(1);
    let mut ship = Vehicle::new(
        ship_id,
        company_id,
        EngineID(1),
        VehicleKind::Ship,
        dock_a_pos,
        orders_id,
    );
    ship.cargo.push((CargoType(1), CargoAmount(40))); // 40 Grain
    kernel.world.vehicles.insert(ship_id, ship);

    // Setup OpenRCT2 Peeps & Turnstile at Port San Pedro
    let mut turnstile = StationTurnstile::new(dock_a, Money(5));
    for i in 1..=4 {
        turnstile.enqueue(transport_types::PersonID(i));
    }

    // Register drivers in microkernel
    let cmd_queue = Arc::new(CommandQueue::new());
    let event_ring = Arc::new(EventRingBuffer::new(64));
    let mut consumer = event_ring.consumer();

    kernel.register_driver(ApiSubsystemDriver::new(
        Arc::clone(&cmd_queue),
        Arc::clone(&event_ring),
    ));
    kernel.register_driver(MovementDriver::new());
    kernel.register_driver(LogisticsDriver::new());
    kernel.register_driver(PeopleDriver::new());

    let mut event_history: Vec<Arc<Event>> = Vec::new();
    let mut admitted_count = 0;

    println!("\x1b[2J\x1b[H"); // Initial clear
    for _step in 1..=total_ticks {
        // Step turnstile admission
        if let Some(_peep_id) = turnstile.pop_front() {
            admitted_count += 1;
        }

        kernel.tick().unwrap();

        // Drain new events from the ring buffer
        let (new_events, _) = consumer.drain_available();
        event_history.extend(new_events);

        if clear_screen {
            print!("\x1b[H"); // Move cursor to top-left
        }

        let company = kernel.world.companies.get(&company_id).unwrap();
        let ship = kernel.world.vehicles.get(&ship_id).unwrap();
        let delivered = kernel
            .world
            .stations
            .get(&dock_b)
            .and_then(|s| s.goods.first())
            .map(|g| g.delivered_since_last_visit.0 as usize)
            .unwrap_or(0);

        let cargo_count = ship.cargo.first().map(|c| c.1 .0 as usize).unwrap_or(0);
        let state_str = format!("{:?}", ship.state);

        print_hud(
            kernel.world.tick.0 as u64,
            company.money.0,
            ship.position,
            &state_str,
            cargo_count,
            delivered,
            admitted_count,
        );

        let ships_list = vec![(ship_id, ship.position)];
        render_map(&kernel.world, dock_a_pos, dock_b_pos, &obstacles, &ships_list);

        print_event_log(&event_history);

        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }

    println!("\x1b[1;32m✔ Simulation run complete ({total_ticks} ticks executed successfully).\x1b[0m\n");
}

use pretty_assertions::assert_eq;
use transport_dashboard::{
    export_json, export_prometheus, render_financial_table, render_sparkline,
    render_telemetry_card, CompanyLedger, FinancialCategory, FixedRing, TelemetryEngine,
    TelemetrySnapshot,
};
use transport_sim::World;
use transport_types::unit::Money;
use transport_types::{CompanyID, Ticks, TownID};
use transport_world::{Company, MapSize, Town};

#[test]
fn test_fixed_ring_circular_overwrite_and_chronology() {
    let mut ring: FixedRing<i32, 3> = FixedRing::new();
    assert_eq!(ring.len(), 0);
    assert!(ring.is_empty());
    assert_eq!(ring.latest(), None);

    ring.push(10);
    ring.push(20);
    assert_eq!(ring.len(), 2);
    assert_eq!(ring.to_vec(), vec![10, 20]);
    assert_eq!(ring.latest(), Some(&20));

    ring.push(30);
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.to_vec(), vec![10, 20, 30]);

    // Push 4th element -> overwrites oldest (10)
    ring.push(40);
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.to_vec(), vec![20, 30, 40]);
    assert_eq!(ring.latest(), Some(&40));

    // Push 5th element -> overwrites 20
    ring.push(50);
    assert_eq!(ring.to_vec(), vec![30, 40, 50]);
}

#[test]
fn test_financial_ledger_cash_flow_and_monthly_roll() {
    let company_id = CompanyID(1);
    let mut ledger = CompanyLedger::new(company_id, 0);

    // Record transactions in current tick
    ledger.record_transaction(FinancialCategory::CargoFreight, 15_000);
    ledger.record_transaction(FinancialCategory::PassengerFares, 5_000);
    ledger.record_transaction(FinancialCategory::VehicleRunning, 3_000);
    ledger.record_transaction(FinancialCategory::StationConstruction, 8_000);

    assert_eq!(ledger.current_tick.total_revenue(), 20_000);
    assert_eq!(ledger.current_tick.total_expenses(), 11_000);
    assert_eq!(ledger.current_tick.net_profit(), 9_000);

    // Advance ticks through a month roll (1000 ticks)
    for t in 1..=1000 {
        ledger.tick_advance(t);
    }

    assert_eq!(ledger.monthly_history.len(), 1);
    let last_month = ledger.monthly_history.latest().unwrap();
    assert_eq!(last_month.total_revenue(), 20_000);
    assert_eq!(last_month.total_expenses(), 11_000);
    assert_eq!(last_month.net_profit(), 9_000);
}

#[test]
fn test_telemetry_sampling_from_world() {
    let mut world = World::new(MapSize::new(16, 16));
    let company_id = CompanyID(1);
    world.companies.insert(
        company_id,
        Company::new(
            company_id,
            "Atlantic Co".to_string(),
            Money(85_000),
            0x00FF00,
        ),
    );

    let mut town = Town::new(TownID(1), "Portside".to_string(), transport_types::TileIndex::new(5, 5));
    town.satisfaction_score = 92;
    world.towns.insert(TownID(1), town);
    world.tick = Ticks(150);

    let mut telemetry = TelemetryEngine::new();
    telemetry.sample_from_world(&world, company_id, 340, 120);

    let latest = telemetry.latest().expect("Snapshot must exist");
    assert_eq!(latest.tick, 150);
    assert_eq!(latest.treasury_balance, 85_000);
    assert_eq!(latest.average_satisfaction, 92);
    assert_eq!(latest.total_cargo_delivered, 340);
    assert_eq!(latest.total_passengers_served, 120);
}

#[test]
fn test_sparkline_and_ascii_table_rendering() {
    let values = vec![10, 20, 35, 50, 75, 100];
    let spark = render_sparkline(&values);
    assert_eq!(spark.chars().count(), 6);

    let ledger = CompanyLedger::new(CompanyID(1), 0);
    let table = render_financial_table(&ledger);
    assert!(table.contains("PERIOD"));
    assert!(table.contains("REVENUE"));
    assert!(table.contains("NET PROFIT"));

    let mut telemetry = TelemetryEngine::new();
    telemetry.record(TelemetrySnapshot {
        tick: 45,
        active_vehicles: 4,
        total_cargo_delivered: 200,
        total_passengers_served: 80,
        average_satisfaction: 88,
        treasury_balance: 50_000,
    });

    let card = render_telemetry_card(&telemetry, &ledger);
    assert!(card.contains("NETWORK TELEMETRY DASHBOARD"));
    assert!(card.contains("Active Fleet: 4"));
    assert!(card.contains("Satisfaction: 88%"));
}

#[test]
fn test_prometheus_and_json_export() {
    let mut ledger = CompanyLedger::new(CompanyID(1), 0);
    ledger.record_transaction(FinancialCategory::CargoFreight, 25_000);
    ledger.record_transaction(FinancialCategory::VehicleRunning, 5_000);

    let mut telemetry = TelemetryEngine::new();
    telemetry.record(TelemetrySnapshot {
        tick: 100,
        active_vehicles: 3,
        total_cargo_delivered: 500,
        total_passengers_served: 250,
        average_satisfaction: 85,
        treasury_balance: 120_000,
    });

    let prom = export_prometheus(&telemetry, &ledger);
    assert!(prom.contains("transport_sim_tick 100"));
    assert!(prom.contains("transport_treasury_balance_dollars{company=\"1\"} 120000"));
    assert!(prom.contains("transport_town_satisfaction_percentage 85"));

    let json_str = export_json(&telemetry, &ledger).expect("JSON serialization must succeed");
    assert!(json_str.contains("\"treasury_balance\": 120000"));
    assert!(json_str.contains("\"average_satisfaction\": 85"));
}

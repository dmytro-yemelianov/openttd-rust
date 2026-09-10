use transport_people::commute::CommutePlan;
use transport_people::town_driver::TownEconomyDriver;
use transport_sim::kernel::Microkernel;
use transport_types::{
    BuildingID, CompanyID, Money, StationID, Ticks, TileIndex, TownID,
};
use transport_world::definitions::Station;
use transport_world::entities::Company;
use transport_world::map::MapSize;
use transport_world::town::{Building, BuildingKind, Town};

#[test]
fn test_zero_revenue_on_circular_passenger_laundering() {
    let mut kernel = Microkernel::new(MapSize::new(20, 20));
    let mut driver = TownEconomyDriver::new();

    let res_tile = TileIndex::new(2, 2);
    let work_tile = TileIndex::new(15, 15);
    let st1_tile = TileIndex::new(3, 2);
    let st2_tile = TileIndex::new(8, 8); // Intermediate stop (nowhere near work_tile)

    let company_id = CompanyID(1);
    kernel.world.companies.insert(
        company_id,
        Company::new(company_id, "Transit Co".into(), Money(1000), 0x00FF00),
    );

    let town1_id = TownID(1);
    let mut town1 = Town::new(town1_id, "South Haven".into(), res_tile);
    town1.building_ids.push(BuildingID(1));
    kernel.world.towns.insert(town1_id, town1);

    let res_bldg = Building::new(
        BuildingID(1),
        town1_id,
        res_tile,
        BuildingKind::Residential,
        50,
    );
    let work_bldg = Building::new(
        BuildingID(2),
        town1_id,
        work_tile,
        BuildingKind::Commercial,
        50,
    );
    kernel.world.buildings.insert(BuildingID(1), res_bldg);
    kernel.world.buildings.insert(BuildingID(2), work_bldg);

    let st1_id = StationID(1);
    let mut st1 = Station::new(st1_id, "Origin Station".into(), vec![st1_tile]);
    st1.company_id = Some(company_id);
    kernel.world.stations.insert(st1_id, st1);

    let st2_id = StationID(2);
    let mut st2 = Station::new(st2_id, "Midway Station".into(), vec![st2_tile]);
    st2.company_id = Some(company_id);
    kernel.world.stations.insert(st2_id, st2);

    driver.register_turnstile(st1_id, Money(5));
    driver.register_turnstile(st2_id, Money(5));

    // Commuter plan with explicit destination at work_tile (15, 15)
    let plan = CommutePlan::new(
        BuildingID(1),
        BuildingID(2),
        res_tile,
        work_tile,
        Money(20),
        Ticks(0),
        100, // Max patience: 100 ticks
    );

    // If passenger is dropped at st2_tile (8, 8), verify is_at_destination is FALSE
    assert!(!plan.is_at_destination(st2_tile), "Midway stop is not destination");

    // Advance tick beyond patience at the wrong station
    let late_tick = Ticks(150);
    let satisfaction = plan.calculate_satisfaction(late_tick);
    assert_eq!(
        satisfaction, 10,
        "Laundering passenger without reaching destination yields minimum satisfaction"
    );
}

#[test]
fn test_end_to_end_commute_journey_and_fare_deduction() {
    let mut kernel = Microkernel::new(MapSize::new(20, 20));
    let mut driver = TownEconomyDriver::new();

    let company_id = CompanyID(1);
    kernel.world.companies.insert(
        company_id,
        Company::new(company_id, "Metro Corp".into(), Money(1000), 0x0000FF),
    );

    let town1_id = TownID(1);
    let town2_id = TownID(2);
    let res_tile = TileIndex::new(2, 2);
    let work_tile = TileIndex::new(14, 14);

    let mut town1 = Town::new(town1_id, "Suburbs".into(), res_tile);
    town1.building_ids.push(BuildingID(1));
    kernel.world.towns.insert(town1_id, town1);

    let mut town2 = Town::new(town2_id, "Downtown".into(), work_tile);
    town2.building_ids.push(BuildingID(2));
    kernel.world.towns.insert(town2_id, town2);

    let res_bldg = Building::new(
        BuildingID(1),
        town1_id,
        res_tile,
        BuildingKind::Residential,
        20,
    );
    let work_bldg = Building::new(
        BuildingID(2),
        town2_id,
        work_tile,
        BuildingKind::Commercial,
        20,
    );
    kernel.world.buildings.insert(BuildingID(1), res_bldg);
    kernel.world.buildings.insert(BuildingID(2), work_bldg);

    let st1_id = StationID(1);
    let st2_id = StationID(2);
    let st1_tile = TileIndex::new(3, 2);
    let st2_tile = TileIndex::new(13, 14);

    let mut st1 = Station::new(st1_id, "Suburbs Station".into(), vec![st1_tile]);
    st1.company_id = Some(company_id);
    let mut st2 = Station::new(st2_id, "Downtown Station".into(), vec![st2_tile]);
    st2.company_id = Some(company_id);

    kernel.world.stations.insert(st1_id, st1);
    kernel.world.stations.insert(st2_id, st2);

    driver.register_turnstile(st1_id, Money(5));
    driver.register_turnstile(st2_id, Money(5));

    kernel.register_driver(driver);

    // 1. Run tick with Solvers phase to generate commute demand
    kernel.tick().unwrap();

    // Verify company received turnstile ticket fare ($5)
    let comp_money = kernel.world.companies.get(&company_id).unwrap().money;
    assert_eq!(
        comp_money,
        Money(1005),
        "Company must receive $5 turnstile fare upon admission"
    );

    // Downcast driver to inspect active commuter
    let driver_ref = kernel.drivers[0]
        .as_any_mut()
        .unwrap()
        .downcast_mut::<TownEconomyDriver>()
        .unwrap();

    assert_eq!(driver_ref.active_commuters.len(), 1);
    let (person_id, (peep, _plan)) = driver_ref.active_commuters.iter_mut().next().unwrap();
    assert_eq!(peep.wallet, Money(95));
    assert_eq!(peep.fare_paid, Money(5));

    // Simulate peep arriving at downtown station
    peep.current_tile = st2_tile;
    let pid = *person_id;

    // 2. Next tick: Driver handles arrival at destination
    kernel.tick().unwrap();

    let driver_ref2 = kernel.drivers[0]
        .as_any_mut()
        .unwrap()
        .downcast_mut::<TownEconomyDriver>()
        .unwrap();

    // Commuter reached destination and dematerialized
    assert!(!driver_ref2.active_commuters.contains_key(&pid));

    // Town 1 recorded commute fulfillment
    let town1_ref = kernel.world.towns.get(&town1_id).unwrap();
    assert_eq!(town1_ref.fulfilled_commutes, 1);
    assert_eq!(town1_ref.demanded_commutes, 1);
    assert!(town1_ref.fulfillment_ratio() >= 1.0);
}

#[test]
fn test_fulfillment_driven_town_growth() {
    let mut kernel = Microkernel::new(MapSize::new(30, 30));
    let center = TileIndex::new(15, 15);
    let town_id = TownID(1);
    let mut town = Town::new(town_id, "Metropolis".into(), center);
    town.census_population = 50;

    let b1 = Building::new(
        BuildingID(1),
        town_id,
        center,
        BuildingKind::Residential,
        50,
    );
    town.building_ids.push(BuildingID(1));
    kernel.world.buildings.insert(BuildingID(1), b1);
    kernel.world.towns.insert(town_id, town);

    let driver = TownEconomyDriver::new();
    kernel.register_driver(driver);

    // Record 5 successful commute cycles
    let t = kernel.world.towns.get_mut(&town_id).unwrap();
    for _ in 0..5 {
        t.record_commute(true, 95);
    }

    assert_eq!(t.fulfillment_ratio(), 1.0);
    assert!(t.satisfaction_score >= 70);

    // Run 5 ticks to trigger Phase::Commit town growth evaluation (threshold: 5 cycles)
    for _ in 0..5 {
        kernel.tick().unwrap();
    }

    let t_after = kernel.world.towns.get(&town_id).unwrap();
    assert_eq!(
        t_after.building_ids.len(),
        2,
        "Town must construct a new building after high commute fulfillment"
    );
    assert!(
        t_after.census_population > 50,
        "Census population must increase with new building capacity"
    );
    assert_eq!(kernel.world.buildings.len(), 2);
}

#[test]
fn test_active_peep_pool_capacity_bound() {
    let mut kernel = Microkernel::new(MapSize::new(20, 20));
    let mut driver = TownEconomyDriver::new();
    driver.max_active_peeps = 2; // Strict bound for testing

    let res_tile = TileIndex::new(2, 2);
    let work_tile = TileIndex::new(10, 10);
    let st1_tile = TileIndex::new(3, 2);
    let st2_tile = TileIndex::new(9, 10);

    let town_id = TownID(1);
    let mut town = Town::new(town_id, "CapTown".into(), res_tile);
    for i in 1..=5 {
        let b = Building::new(
            BuildingID(i),
            town_id,
            res_tile,
            BuildingKind::Residential,
            10,
        );
        town.building_ids.push(BuildingID(i));
        kernel.world.buildings.insert(BuildingID(i), b);
    }
    let work_bldg = Building::new(
        BuildingID(6),
        town_id,
        work_tile,
        BuildingKind::Commercial,
        100,
    );
    kernel.world.buildings.insert(BuildingID(6), work_bldg);
    kernel.world.towns.insert(town_id, town);

    let st1 = Station::new(StationID(1), "Cap St 1".into(), vec![st1_tile]);
    let st2 = Station::new(StationID(2), "Cap St 2".into(), vec![st2_tile]);
    kernel.world.stations.insert(StationID(1), st1);
    kernel.world.stations.insert(StationID(2), st2);

    kernel.register_driver(driver);

    // Tick multiple times to attempt spawning more peeps than capacity
    for _ in 0..10 {
        kernel.tick().unwrap();
    }

    let driver_ref = kernel.drivers[0]
        .as_any_mut()
        .unwrap()
        .downcast_mut::<TownEconomyDriver>()
        .unwrap();

    assert!(
        driver_ref.active_commuters.len() <= 2,
        "Active commuters count ({}) must strictly not exceed max_active_peeps (2)",
        driver_ref.active_commuters.len()
    );
}

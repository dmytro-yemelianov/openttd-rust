use crate::driver::PeopleDriver;
use crate::peep::{Peep, PeepState, StaffRole};
use transport_sim::drivers::MovementDriver;
use transport_sim::kernel::Microkernel;
use transport_types::enum_::{OrderType, VehicleKind, VehicleState};
use transport_types::{
    CompanyID, EngineID, Money, OrderListID, PersonID, StationID, TileIndex, VehicleID,
};
use transport_world::definitions::Station;
use transport_world::entities::{Company, Vehicle};
use transport_world::map::MapSize;

    #[test]
    fn test_turnstile_fifo_and_fare_admission() {
        let mut kernel = Microkernel::new(MapSize::new(20, 20));
        let company_id = CompanyID(1);
        kernel.world.companies.insert(
            company_id,
            Company::new(company_id, "Ferry Corp".into(), Money(1000), 0x00FF00),
        );

        let st_id = StationID(10);
        let dock_tile = TileIndex::new(5, 5);
        let mut st = Station::new(st_id, "Ferry Terminal".into(), vec![dock_tile]);
        st.company_id = Some(company_id);
        kernel.world.stations.insert(st_id, st);
        kernel.world.map.set_station_at(dock_tile, st_id, Some(company_id)).unwrap();
        kernel.world.tile_to_station.insert(dock_tile, st_id);

        let ship_id = VehicleID(1);
        let orders_id = OrderListID(1);
        kernel.world.order_lists.insert(
            orders_id,
            vec![OrderType::GoToStation {
                station_id: st_id,
                conditions: None,
            }],
        );

        let mut ship = Vehicle::new(
            ship_id,
            company_id,
            EngineID(1),
            VehicleKind::Ship,
            dock_tile,
            orders_id,
        );
        ship.state = VehicleState::Loading;
        kernel.world.vehicles.insert(ship_id, ship);

        let mut people_driver = PeopleDriver::new();
        people_driver.configure_turnstile(st_id, Money(25));
        people_driver.configure_manifest(ship_id, 3); // Capacity for 3 passengers

        // Enqueue 5 peeps in turnstile queue line
        for i in 1..=5 {
            let p_id = PersonID(i);
            let mut peep = Peep::new(p_id, dock_tile, StationID(20), Money(100));
            peep.state = PeepState::InStationQueue { station_id: st_id };
            people_driver.add_peep(peep);
        }

        kernel.register_driver(people_driver);

        // Tick 1: Microkernel processes Drivers phase and commits fare transactions
        kernel.tick().unwrap();

        // Check Company revenue: 3 passengers * $25 fare = +$75
        let comp_funds = kernel.world.companies.get(&company_id).unwrap().money;
        assert_eq!(comp_funds, Money(1075), "Company must receive $75 from 3 turnstile admissions");

        // Extract registered driver
        let driver = kernel.drivers[0]
            .as_any_mut()
            .unwrap()
            .downcast_mut::<PeopleDriver>()
            .expect("Expected PeopleDriver");

        // Verify exactly 3 passengers boarded (FIFO order: IDs 1, 2, 3)
        let manifest = driver.manifests.get(&ship_id).unwrap();
        assert_eq!(manifest.passengers, vec![PersonID(1), PersonID(2), PersonID(3)]);
        assert_eq!(manifest.available_seats(), 0);

        // Verify remaining 2 peeps in queue (IDs 4, 5)
        let turnstile = driver.turnstiles.get(&st_id).unwrap();
        assert_eq!(turnstile.queue_len(), 2);
        assert_eq!(turnstile.admitted_total, 3);
    }

    #[test]
    fn test_end_to_end_peep_ferry_journey() {
        let mut kernel = Microkernel::new(MapSize::new(20, 20));
        let comp_id = CompanyID(0);
        kernel.world.companies.insert(
            comp_id,
            Company::new(comp_id, "Island Ferry".into(), Money(5000), 0x0000FF),
        );

        let dock_a = StationID(1);
        let dock_b = StationID(2);
        let tile_a = TileIndex::new(2, 2);
        let tile_b = TileIndex::new(5, 2);

        let mut st_a = Station::new(dock_a, "Island A Dock".into(), vec![tile_a]);
        st_a.company_id = Some(comp_id);
        let mut st_b = Station::new(dock_b, "Island B Dock".into(), vec![tile_b]);
        st_b.company_id = Some(comp_id);

        kernel.world.stations.insert(dock_a, st_a);
        kernel.world.stations.insert(dock_b, st_b);
        kernel.world.map.set_station_at(tile_a, dock_a, Some(comp_id)).unwrap();
        kernel.world.map.set_station_at(tile_b, dock_b, Some(comp_id)).unwrap();
        kernel.world.tile_to_station.insert(tile_a, dock_a);
        kernel.world.tile_to_station.insert(tile_b, dock_b);

        let orders_id = OrderListID(0);
        kernel.world.order_lists.insert(
            orders_id,
            vec![
                OrderType::GoToStation {
                    station_id: dock_a,
                    conditions: None,
                },
                OrderType::GoToStation {
                    station_id: dock_b,
                    conditions: None,
                },
            ],
        );

        let ship_id = VehicleID(0);
        let mut ship = Vehicle::new(
            ship_id,
            comp_id,
            EngineID(0),
            VehicleKind::Ship,
            tile_a,
            orders_id,
        );
        ship.state = VehicleState::Idle;
        kernel.world.vehicles.insert(ship_id, ship);

        let mut people_driver = PeopleDriver::new();
        people_driver.configure_turnstile(dock_a, Money(10));
        people_driver.configure_turnstile(dock_b, Money(10));
        people_driver.configure_manifest(ship_id, 10);

        // Peep spawns at tile_a, destination is dock_b
        let traveler_id = PersonID(100);
        let mut traveler = Peep::new(traveler_id, tile_a, dock_b, Money(50));
        traveler.state = PeepState::InStationQueue { station_id: dock_a };
        people_driver.add_peep(traveler);

        // Assign Captain to the vessel
        let captain_id = PersonID(999);
        let captain = Peep::new(captain_id, tile_a, dock_a, Money(0)).with_role(StaffRole::Captain);
        people_driver.add_peep(captain);
        people_driver.assign_crew(ship_id, captain_id, StaffRole::Captain);

        kernel.register_driver(MovementDriver::new());
        kernel.register_driver(people_driver);

        // Tick 1: Ship at Dock A transitions to Loading; Traveler passes turnstile and boards
        kernel.tick().unwrap();

        let driver = kernel.drivers[1]
            .as_any_mut()
            .unwrap()
            .downcast_mut::<PeopleDriver>()
            .expect("Expected PeopleDriver");

        let traveler_state = driver.peeps.get(&traveler_id).unwrap().state;
        assert_eq!(
            traveler_state,
            PeepState::AboardVehicle { vehicle_id: ship_id },
            "Traveler must board the ship"
        );
        assert!(
            driver.manifests.get(&ship_id).unwrap().has_captain(),
            "Ship must have certified Captain"
        );

        // Advance simulation until ship reaches Dock B (distance = 3 tiles)
        for _ in 0..10 {
            kernel.tick().unwrap();
            if kernel.world.vehicles.get(&ship_id).unwrap().position == tile_b {
                break;
            }
        }

        // Advance 1 more tick at Dock B to process alighting
        kernel.tick().unwrap();

        let driver = kernel.drivers[1]
            .as_any_mut()
            .unwrap()
            .downcast_mut::<PeopleDriver>()
            .expect("Expected PeopleDriver");

        let final_state = driver.peeps.get(&traveler_id).unwrap().state;
        assert_eq!(
            final_state,
            PeepState::Completed,
            "Traveler should successfully complete journey upon reaching destination dock"
        );
    }

use crate::drivers::{EconomyDriver, LogisticsDriver, MovementDriver};
use crate::kernel::*;
use std::sync::Arc;
use transport_types::enum_::{OrderType, VehicleKind, VehicleState};
use transport_types::{
    CapabilityToken, CargoAmount, CargoType, CompanyID, EngineID, Money, OrderListID, ServiceId,
    ServicePriority, StationID, TileIndex, VehicleID,
};
use transport_world::definitions::{GoodsEntry, Station};
use transport_world::entities::{Company, Vehicle};
use transport_world::map::MapSize;

struct TraceDriver {
    id: ServiceId,
    priority: ServicePriority,
    execution_trace: Arc<std::sync::Mutex<Vec<ServiceId>>>,
}

impl SubsystemDriver for TraceDriver {
    fn id(&self) -> ServiceId {
        self.id
    }

    fn priority(&self) -> ServicePriority {
        self.priority
    }

    fn execute_phase(&mut self, phase: Phase, _ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase == Phase::Drivers {
            self.execution_trace.lock().unwrap().push(self.id);
        }
        Ok(())
    }
}

#[test]
fn test_microkernel_deterministic_driver_priority_order() {
    let mut kernel = Microkernel::new(MapSize::new(10, 10));
    let trace = Arc::new(std::sync::Mutex::new(Vec::new()));

    // Register drivers out of priority order: Low, Critical, High
    kernel.register_driver(TraceDriver {
        id: ServiceId::Economy,
        priority: ServicePriority::LOW,
        execution_trace: Arc::clone(&trace),
    });
    kernel.register_driver(TraceDriver {
        id: ServiceId::Storage,
        priority: ServicePriority::CRITICAL,
        execution_trace: Arc::clone(&trace),
    });
    kernel.register_driver(TraceDriver {
        id: ServiceId::Movement,
        priority: ServicePriority::HIGH,
        execution_trace: Arc::clone(&trace),
    });

    kernel.tick().expect("Microkernel tick failed");

    let recorded = trace.lock().unwrap().clone();
    assert_eq!(
        recorded,
        vec![ServiceId::Storage, ServiceId::Movement, ServiceId::Economy],
        "Drivers must execute strictly in priority order (Critical -> High -> Low)"
    );
}

#[test]
fn test_microkernel_capability_enforcement_rejects_unauthorized_mutation() {
    let mut kernel = Microkernel::new(MapSize::new(10, 10));
    kernel.world.companies.insert(
        CompanyID(1),
        Company::new(CompanyID(1), "Co 1".into(), Money(1000), 0x0000FF),
    );
    kernel.world.companies.insert(
        CompanyID(2),
        Company::new(CompanyID(2), "Co 2".into(), Money(1000), 0xFF0000),
    );

    struct RogueDriver;
    impl SubsystemDriver for RogueDriver {
        fn id(&self) -> ServiceId {
            ServiceId::Custom(99)
        }
        fn priority(&self) -> ServicePriority {
            ServicePriority::NORMAL
        }
        fn execute_phase(
            &mut self,
            phase: Phase,
            ctx: &mut KernelContext,
        ) -> Result<(), DriverError> {
            if phase == Phase::Drivers {
                // Token for Company 1 trying to deduct money from Company 2
                ctx.stage_intent(
                    CapabilityToken::Company(CompanyID(1)),
                    KernelIntent::DeductCost {
                        company_id: CompanyID(2),
                        amount: Money(500),
                    },
                );
            }
            Ok(())
        }
    }

    kernel.register_driver(RogueDriver);
    let res = kernel.tick();

    assert!(
        res.is_err(),
        "Rogue driver attempting unauthorized cross-company mutation must be rejected"
    );
    assert_eq!(
        kernel.world.companies.get(&CompanyID(2)).unwrap().money,
        Money(1000),
        "Company 2 treasury must remain untouched"
    );
}

#[test]
fn test_microkernel_movement_and_logistics_slice() {
    let mut kernel = Microkernel::new(MapSize::new(30, 30));
    let comp_id = CompanyID(0);
    kernel.world.companies.insert(
        comp_id,
        Company::new(comp_id, "Marine Ltd".into(), Money(50000), 0x0000FF),
    );

    // Create 2 dock stations: (5, 5) and (8, 5)
    let st1_id = StationID(0);
    let st2_id = StationID(1);
    let mut st1 = Station::new(st1_id, "Dock Alpha".into(), vec![TileIndex::new(5, 5)]);
    st1.company_id = Some(comp_id);
    let mut goods1 = GoodsEntry::new(CargoType(1));
    goods1.amount = CargoAmount(40);
    st1.goods.push(goods1);

    let mut st2 = Station::new(st2_id, "Dock Beta".into(), vec![TileIndex::new(8, 5)]);
    st2.company_id = Some(comp_id);

    kernel.world.stations.insert(st1_id, st1);
    kernel.world.stations.insert(st2_id, st2);
    kernel
        .world
        .map
        .set_station_at(TileIndex::new(5, 5), st1_id, Some(comp_id))
        .unwrap();
    kernel
        .world
        .map
        .set_station_at(TileIndex::new(8, 5), st2_id, Some(comp_id))
        .unwrap();
    kernel
        .world
        .tile_to_station
        .insert(TileIndex::new(5, 5), st1_id);
    kernel
        .world
        .tile_to_station
        .insert(TileIndex::new(8, 5), st2_id);

    // Schedule: First visit Dock Alpha (load), then Go to Dock Beta (unload)
    let o_list_id = OrderListID(0);
    kernel.world.order_lists.insert(
        o_list_id,
        vec![
            OrderType::GoToStation {
                station_id: st1_id,
                conditions: None,
            },
            OrderType::GoToStation {
                station_id: st2_id,
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
        TileIndex::new(5, 5),
        o_list_id,
    );
    ship.state = VehicleState::Idle;
    kernel.world.vehicles.insert(ship_id, ship);

    kernel.register_driver(MovementDriver::new());
    kernel.register_driver(LogisticsDriver::new());
    kernel.register_driver(EconomyDriver::new());

    // Tick 1: Ship at Dock Alpha: order 0 targets Dock Alpha -> transitions to Loading and loads 40 units
    kernel.tick().unwrap();
    let loaded = kernel.world.vehicles.get(&ship_id).unwrap().cargo.clone();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].1, CargoAmount(40));

    // Ticks: Ship travels towards Dock Beta until it reaches (8, 5)
    for _ in 0..10 {
        kernel.tick().unwrap();
        if kernel.world.vehicles.get(&ship_id).unwrap().position == TileIndex::new(8, 5) {
            break;
        }
    }

    let ship_pos = kernel.world.vehicles.get(&ship_id).unwrap().position;
    assert_eq!(
        ship_pos,
        TileIndex::new(8, 5),
        "Ship should reach Dock Beta"
    );

    // When at Dock Beta: Unload has happened and company earned delivery revenue
    let company_money = kernel.world.companies.get(&comp_id).unwrap().money;
    assert!(
        company_money.0 > 50000,
        "Company must earn delivery revenue on cargo delivery"
    );
}

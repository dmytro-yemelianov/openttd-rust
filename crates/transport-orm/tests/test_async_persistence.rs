use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use transport_orm::{
    load_snapshot_envelope, replay_journal, AsyncPersistenceDriver, JournalReader, JournalWriter,
};
use transport_sim::kernel::context::KernelContext;
use transport_sim::kernel::driver::{DriverError, SubsystemDriver};
use transport_sim::kernel::phase::Phase;
use transport_sim::kernel::Microkernel;
use transport_sim::KernelIntent;
use transport_types::{CapabilityToken, CompanyID, Money, ServiceId, ServicePriority};
use transport_world::entities::Company;
use transport_world::map::MapSize;

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("openttd_test_async_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_async_snapshot_non_blocking_and_flush() {
    let dir = test_dir("snapshot");
    let snapshot_path = dir.join("async_snapshot.json");

    let mut kernel = Microkernel::new(MapSize::new(10, 10));
    let comp_id = CompanyID(1);
    kernel.world.companies.insert(
        comp_id,
        Company::new(comp_id, "Fast Transit".into(), Money(5000), 0x00FF00),
    );

    let driver = AsyncPersistenceDriver::new(snapshot_path.clone());
    kernel.register_driver(driver);

    // Tick 1: Runs normally
    kernel.tick().unwrap();

    // Trigger asynchronous snapshot request
    let persistence_driver = kernel
        .drivers
        .iter_mut()
        .find_map(|d| d.as_any_mut()?.downcast_mut::<AsyncPersistenceDriver>())
        .expect("Expected AsyncPersistenceDriver");

    persistence_driver.request_snapshot(snapshot_path.clone());

    // Tick 2: Simulation tick dispatches snapshot to background worker without blocking
    kernel.tick().unwrap();

    // Tick 3: Advance simulation further while worker writes
    kernel.tick().unwrap();

    // Flush and wait for background worker
    let persistence_driver = kernel
        .drivers
        .iter_mut()
        .find_map(|d| d.as_any_mut()?.downcast_mut::<AsyncPersistenceDriver>())
        .expect("Expected AsyncPersistenceDriver");
    persistence_driver.flush_sync();

    // Verify snapshot file exists and envelope is valid
    assert!(snapshot_path.exists(), "Snapshot file must be written to disk");
    let loaded = load_snapshot_envelope(&snapshot_path).unwrap();
    assert_eq!(
        loaded.world.companies.get(&comp_id).unwrap().name,
        "Fast Transit"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_wal_append_and_direct_intent_replay() {
    let dir = test_dir("wal_replay");
    let snapshot_path = dir.join("base_snapshot.json");
    let journal_path = dir.join("journal.wal");

    let mut kernel = Microkernel::new(MapSize::new(20, 20));
    let comp_id = CompanyID(10);
    kernel.world.companies.insert(
        comp_id,
        Company::new(comp_id, "Oceanic Line".into(), Money(2000), 0x123456),
    );

    let initial_world_clone = kernel.world.clone();

    let persistence = AsyncPersistenceDriver::new(snapshot_path)
        .with_journal(journal_path.clone());

    struct IntentStager {
        comp_id: CompanyID,
    }
    impl SubsystemDriver for IntentStager {
        fn id(&self) -> ServiceId {
            ServiceId::Economy
        }
        fn priority(&self) -> ServicePriority {
            ServicePriority::HIGH
        }
        fn execute_phase(
            &mut self,
            phase: Phase,
            ctx: &mut KernelContext,
        ) -> Result<(), DriverError> {
            if phase == Phase::Drivers {
                ctx.stage_intent(
                    CapabilityToken::Company(self.comp_id),
                    KernelIntent::CreditRevenue {
                        company_id: self.comp_id,
                        amount: Money(100),
                    },
                );
            }
            Ok(())
        }
    }

    kernel.register_driver(IntentStager { comp_id });
    kernel.register_driver(persistence);

    // Run 5 ticks with intents staged and committed each tick
    for _ in 0..5 {
        kernel.tick().unwrap();
    }

    // Flush WAL background worker
    let driver = kernel
        .drivers
        .iter_mut()
        .find_map(|d| d.as_any_mut()?.downcast_mut::<AsyncPersistenceDriver>())
        .expect("Expected AsyncPersistenceDriver");
    driver.flush_sync();

    // Read records from WAL
    assert!(journal_path.exists(), "WAL file must exist");
    let mut reader = JournalReader::open(&journal_path).unwrap();
    let records = reader.read_all_valid().unwrap();
    assert_eq!(records.len(), 5, "Must recover all 5 WAL records");

    // Replay journal onto initial world state
    let replayed_world = replay_journal(initial_world_clone, &records);

    // Verify replayed state matches final live world state
    let live_funds = kernel.world.companies.get(&comp_id).unwrap().money;
    let replayed_funds = replayed_world.companies.get(&comp_id).unwrap().money;
    assert_eq!(replayed_funds, live_funds, "Replayed state must match live simulation state exactly");
    assert_eq!(replayed_funds, Money(2000 + 500));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_wal_crash_recovery_with_truncated_trailing_record() {
    let dir = test_dir("crash");
    let wal_path = dir.join("crash_test.wal");

    {
        let mut writer = JournalWriter::open(&wal_path).unwrap();
        // Write 3 valid records
        writer
            .append(
                1,
                vec![KernelIntent::CreditRevenue {
                    company_id: CompanyID(1),
                    amount: Money(50),
                }],
            )
            .unwrap();
        writer
            .append(
                2,
                vec![KernelIntent::CreditRevenue {
                    company_id: CompanyID(1),
                    amount: Money(100),
                }],
            )
            .unwrap();
        writer
            .append(
                3,
                vec![KernelIntent::CreditRevenue {
                    company_id: CompanyID(1),
                    amount: Money(150),
                }],
            )
            .unwrap();
        writer.sync_all().unwrap();
    }

    // Simulate unexpected crash/power loss during write: append partial junk bytes
    {
        let mut f = OpenOptions::new().append(true).open(&wal_path).unwrap();
        // Append an incomplete header (12 bytes instead of 28 bytes)
        f.write_all(b"TPWA\x03\x00\x00\x00\x00\x00\x00\x00").unwrap();
        f.flush().unwrap();
    }

    // Read using JournalReader: should gracefully recover the 3 valid records and stop at corrupted boundary
    let mut reader = JournalReader::open(&wal_path).unwrap();
    let recovered = reader.read_all_valid().unwrap();

    assert_eq!(
        recovered.len(),
        3,
        "Recovery engine must safely discard truncated trailing record and recover all preceding valid records"
    );
    assert_eq!(recovered[0].sequence_number, 0);
    assert_eq!(recovered[1].sequence_number, 1);
    assert_eq!(recovered[2].sequence_number, 2);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_snapshot_coalescing_under_rapid_ticks() {
    let dir = test_dir("coalescing");
    let snapshot_path = dir.join("snapshot.json");

    let mut kernel = Microkernel::new(MapSize::new(10, 10));
    let comp_id = CompanyID(1);
    kernel.world.companies.insert(
        comp_id,
        Company::new(comp_id, "Coalesce Test Corp".into(), Money(1000), 0x990000),
    );

    // Set snapshot interval to every 1 tick
    let persistence = AsyncPersistenceDriver::new(snapshot_path.clone())
        .with_snapshot_interval(1);

    kernel.register_driver(persistence);

    // Rapidly execute 50 ticks in succession
    for _ in 0..50 {
        kernel.tick().unwrap();
    }

    // Flush and wait for worker
    let driver = kernel
        .drivers
        .iter_mut()
        .find_map(|d| d.as_any_mut()?.downcast_mut::<AsyncPersistenceDriver>())
        .unwrap();
    driver.flush_sync();

    // Verify snapshot file exists and reflects recent state
    assert!(snapshot_path.exists(), "Snapshot must be persisted");
    let loaded = load_snapshot_envelope(&snapshot_path).unwrap();
    assert!(
        loaded.world.tick.0 > 0,
        "Snapshot tick must be updated without thread deadlocks"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

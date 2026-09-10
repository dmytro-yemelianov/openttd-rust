use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use transport_people::PeopleSimulator;
use transport_sim::kernel::context::KernelContext;
use transport_sim::kernel::driver::{DriverError, SubsystemDriver};
use transport_sim::kernel::intent::KernelIntent;
use transport_sim::kernel::phase::Phase;
use transport_types::{ServiceId, ServicePriority};

use crate::journal::JournalWriter;
use crate::{save_snapshot_envelope, SnapshotEnvelope};

/// Commands dispatched to the background persistence worker thread.
pub enum WorkerCommand {
    WriteSnapshot {
        envelope: Box<SnapshotEnvelope>,
        target_path: PathBuf,
        in_flight_flag: Arc<AtomicBool>,
    },
    AppendJournal {
        tick: u32,
        intents: Vec<KernelIntent>,
        journal_path: PathBuf,
    },
    Flush(SyncSender<()>),
    Terminate,
}

/// Background persistence worker that executes disk I/O, fsync, and atomic file replacement.
pub struct AsyncPersistenceWorker;

impl AsyncPersistenceWorker {
    pub fn spawn(rx: Receiver<WorkerCommand>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("transport-persistence-worker".into())
            .spawn(move || {
                let mut journal_writer: Option<(PathBuf, JournalWriter)> = None;

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        WorkerCommand::WriteSnapshot {
                            envelope,
                            target_path,
                            in_flight_flag,
                        } => {
                            let _ = save_snapshot_envelope(&envelope, &target_path);
                            in_flight_flag.store(false, Ordering::SeqCst);
                        }
                        WorkerCommand::AppendJournal {
                            tick,
                            intents,
                            journal_path,
                        } => {
                            let writer = match &mut journal_writer {
                                Some((path, writer)) if *path == journal_path => writer,
                                _ => {
                                    if let Ok(w) = JournalWriter::open(&journal_path) {
                                        journal_writer = Some((journal_path, w));
                                        &mut journal_writer.as_mut().unwrap().1
                                    } else {
                                        continue;
                                    }
                                }
                            };
                            let _ = writer.append(tick, intents);
                        }
                        WorkerCommand::Flush(responder) => {
                            if let Some((_, writer)) = &mut journal_writer {
                                let _ = writer.sync_all();
                            }
                            let _ = responder.send(());
                        }
                        WorkerCommand::Terminate => {
                            if let Some((_, writer)) = &mut journal_writer {
                                let _ = writer.sync_all();
                            }
                            break;
                        }
                    }
                }
            })
            .expect("Failed to spawn transport persistence worker thread")
    }
}

/// Asynchronous persistence subsystem driver running during `Phase::Egress`.
pub struct AsyncPersistenceDriver {
    command_tx: Sender<WorkerCommand>,
    worker_handle: Option<JoinHandle<()>>,
    snapshot_path: PathBuf,
    journal_path: Option<PathBuf>,
    snapshot_interval_ticks: Option<u32>,
    pending_snapshot_request: Option<PathBuf>,
    snapshot_in_flight: Arc<AtomicBool>,
}

impl AsyncPersistenceDriver {
    /// Create a new asynchronous persistence driver with default snapshot path.
    pub fn new(snapshot_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = AsyncPersistenceWorker::spawn(rx);
        Self {
            command_tx: tx,
            worker_handle: Some(handle),
            snapshot_path,
            journal_path: None,
            snapshot_interval_ticks: None,
            pending_snapshot_request: None,
            snapshot_in_flight: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Enable append-only Write-Ahead Logging (WAL) to the specified path.
    pub fn with_journal(mut self, journal_path: PathBuf) -> Self {
        self.journal_path = Some(journal_path);
        self
    }

    /// Set periodic snapshot interval in simulation ticks.
    pub fn with_snapshot_interval(mut self, ticks: u32) -> Self {
        self.snapshot_interval_ticks = Some(ticks);
        self
    }

    /// Explicitly request an asynchronous snapshot to be written on the next tick.
    pub fn request_snapshot(&mut self, path: PathBuf) {
        self.pending_snapshot_request = Some(path);
    }

    /// Synchronously flush all pending worker operations and wait for completion.
    pub fn flush_sync(&self) {
        let (sync_tx, sync_rx) = mpsc::sync_channel(1);
        if self.command_tx.send(WorkerCommand::Flush(sync_tx)).is_ok() {
            let _ = sync_rx.recv();
        }
    }
}

impl SubsystemDriver for AsyncPersistenceDriver {
    fn id(&self) -> ServiceId {
        ServiceId::Storage
    }

    fn priority(&self) -> ServicePriority {
        ServicePriority::LOW
    }

    fn execute_phase(&mut self, phase: Phase, ctx: &mut KernelContext) -> Result<(), DriverError> {
        if phase != Phase::Egress {
            return Ok(());
        }

        // 1. Stream committed intents to the Write-Ahead Log
        if let Some(j_path) = &self.journal_path {
            if !ctx.intents.is_empty() {
                let committed_intents: Vec<KernelIntent> =
                    ctx.intents.iter().map(|(_, i)| i.clone()).collect();
                let _ = self.command_tx.send(WorkerCommand::AppendJournal {
                    tick: ctx.tick.0,
                    intents: committed_intents,
                    journal_path: j_path.clone(),
                });
            }
        }

        // 2. Check if a snapshot should be scheduled
        let should_snapshot = if self.pending_snapshot_request.is_some() {
            true
        } else if let Some(interval) = self.snapshot_interval_ticks {
            ctx.tick.0 > 0 && (ctx.tick.0 % interval == 0)
        } else {
            false
        };

        if should_snapshot {
            let target_path = self
                .pending_snapshot_request
                .take()
                .unwrap_or_else(|| self.snapshot_path.clone());

            // Coalesce: only queue if a snapshot is not already being serialized/written
            if !self.snapshot_in_flight.swap(true, Ordering::SeqCst) {
                let envelope = Box::new(SnapshotEnvelope::from_world_and_people(
                    ctx.world,
                    &PeopleSimulator::new(),
                ));
                let _ = self.command_tx.send(WorkerCommand::WriteSnapshot {
                    envelope,
                    target_path,
                    in_flight_flag: Arc::clone(&self.snapshot_in_flight),
                });
            }
        }

        Ok(())
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

impl Drop for AsyncPersistenceDriver {
    fn drop(&mut self) {
        let _ = self.command_tx.send(WorkerCommand::Terminate);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

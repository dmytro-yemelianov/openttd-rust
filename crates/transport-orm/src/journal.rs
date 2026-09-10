use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use transport_sim::kernel::intent::KernelIntent;
use transport_sim::World;

/// Magic bytes at the start of every WAL frame: b"TPWA" (Transport Write-Ahead).
pub const WAL_MAGIC: [u8; 4] = [0x54, 0x50, 0x57, 0x41];

/// Compute standard IEEE 802.3 CRC32 checksum over byte slice.
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A discrete record in the Write-Ahead Log capturing committed intents for a tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalRecord {
    pub sequence_number: u64,
    pub tick: u32,
    pub intents: Vec<KernelIntent>,
}

/// Header for framed on-disk binary format (24 bytes fixed size).
/// Layout:
/// - 4 bytes: Magic (b"TPWA")
/// - 8 bytes: Sequence number (u64 LE)
/// - 4 bytes: Tick (u32 LE)
/// - 4 bytes: Payload length (u32 LE)
/// - 4 bytes: CRC32 checksum of payload (u32 LE)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub sequence_number: u64,
    pub tick: u32,
    pub payload_len: u32,
    pub checksum: u32,
}

impl FrameHeader {
    pub const SIZE: usize = 24;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&WAL_MAGIC);
        bytes[4..12].copy_from_slice(&self.sequence_number.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.tick.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.checksum.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8; Self::SIZE]) -> Option<Self> {
        if bytes[0..4] != WAL_MAGIC {
            return None;
        }
        let seq = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
        let tick = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let len = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let crc = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        Some(Self {
            sequence_number: seq,
            tick,
            payload_len: len,
            checksum: crc,
        })
    }
}

/// Append-only Write-Ahead Log writer with automatic framing and checksumming.
pub struct JournalWriter {
    file: File,
    next_sequence: u64,
}

impl JournalWriter {
    /// Open or create an append-only WAL file.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;

        // Scan existing frames to determine next sequence number
        let next_sequence = Self::scan_last_sequence(&mut file)?;
        Ok(Self {
            file,
            next_sequence,
        })
    }

    fn scan_last_sequence(file: &mut File) -> io::Result<u64> {
        file.seek(SeekFrom::Start(0))?;
        let mut last_seq = 0u64;
        let mut header_buf = [0u8; FrameHeader::SIZE];

        while let Ok(()) = file.read_exact(&mut header_buf) {
            if let Some(header) = FrameHeader::from_bytes(&header_buf) {
                last_seq = header.sequence_number + 1;
                file.seek(SeekFrom::Current(header.payload_len as i64))?;
            } else {
                break;
            }
        }
        file.seek(SeekFrom::End(0))?;
        Ok(last_seq)
    }

    /// Append a batch of committed intents to the WAL.
    pub fn append(&mut self, tick: u32, intents: Vec<KernelIntent>) -> io::Result<u64> {
        if intents.is_empty() {
            return Ok(self.next_sequence);
        }

        let record = JournalRecord {
            sequence_number: self.next_sequence,
            tick,
            intents,
        };

        let payload = serde_json::to_vec(&record)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let checksum = compute_crc32(&payload);

        let header = FrameHeader {
            sequence_number: self.next_sequence,
            tick,
            payload_len: payload.len() as u32,
            checksum,
        };

        self.file.write_all(&header.to_bytes())?;
        self.file.write_all(&payload)?;
        self.file.flush()?;

        let assigned_seq = self.next_sequence;
        self.next_sequence += 1;
        Ok(assigned_seq)
    }

    /// Force synchronous flush and sync to disk.
    pub fn sync_all(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }
}

/// Validating reader for recovering records from a Write-Ahead Log.
pub struct JournalReader {
    file: File,
}

impl JournalReader {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self { file })
    }

    /// Read all valid records up to the first corruption or unexpected EOF.
    pub fn read_all_valid(&mut self) -> io::Result<Vec<JournalRecord>> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut records = Vec::new();
        let mut header_buf = [0u8; FrameHeader::SIZE];

        loop {
            match self.file.read_exact(&mut header_buf) {
                Ok(()) => {
                    let Some(header) = FrameHeader::from_bytes(&header_buf) else {
                        // Invalid magic: corrupt boundary
                        break;
                    };

                    let mut payload = vec![0u8; header.payload_len as usize];
                    if self.file.read_exact(&mut payload).is_err() {
                        // Truncated payload from crash
                        break;
                    }

                    if compute_crc32(&payload) != header.checksum {
                        // CRC checksum mismatch: corrupted record
                        break;
                    }

                    if let Ok(record) = serde_json::from_slice::<JournalRecord>(&payload) {
                        records.push(record);
                    } else {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    // Normal clean EOF
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(records)
    }
}

/// Apply a single KernelIntent directly into World state.
pub fn apply_intent_to_world(world: &mut World, intent: &KernelIntent) {
    match intent {
        KernelIntent::CreditRevenue { company_id, amount } => {
            if let Some(company) = world.companies.get_mut(company_id) {
                company.money = company.money.saturating_add(*amount);
            }
        }
        KernelIntent::DeductCost { company_id, amount } => {
            if let Some(company) = world.companies.get_mut(company_id) {
                company.money = company.money.saturating_sub(*amount);
            }
        }
        KernelIntent::MoveVehicle {
            vehicle_id,
            to_tile,
        } => {
            if let Some(vehicle) = world.vehicles.get_mut(vehicle_id) {
                if world.map.size().is_valid_index(*to_tile) {
                    vehicle.position = *to_tile;
                }
            }
        }
        KernelIntent::TransferCargo {
            vehicle_id,
            station_id,
            cargo_type,
            amount,
            is_load,
        } => {
            if *is_load {
                let to_load = if let Some(station) = world.stations.get_mut(station_id) {
                    if let Some(goods) = station.goods.iter_mut().find(|g| g.cargo_type == *cargo_type) {
                        let transfer = std::cmp::min(goods.amount.0, amount.0);
                        goods.amount.0 -= transfer;
                        transport_types::CargoAmount(transfer)
                    } else {
                        transport_types::CargoAmount(0)
                    }
                } else {
                    transport_types::CargoAmount(0)
                };

                if to_load.0 > 0 {
                    if let Some(vehicle) = world.vehicles.get_mut(vehicle_id) {
                        if let Some(e) = vehicle.cargo.iter_mut().find(|(c, _)| *c == *cargo_type) {
                            e.1 .0 += to_load.0;
                        } else {
                            vehicle.cargo.push((*cargo_type, to_load));
                        }
                    }
                }
            } else {
                let to_unload = if let Some(vehicle) = world.vehicles.get_mut(vehicle_id) {
                    if let Some(e) = vehicle.cargo.iter_mut().find(|(c, _)| *c == *cargo_type) {
                        let transfer = std::cmp::min(e.1 .0, amount.0);
                        e.1 .0 -= transfer;
                        transport_types::CargoAmount(transfer)
                    } else {
                        transport_types::CargoAmount(0)
                    }
                } else {
                    transport_types::CargoAmount(0)
                };

                if to_unload.0 > 0 {
                    if let Some(station) = world.stations.get_mut(station_id) {
                        if let Some(g) = station.goods.iter_mut().find(|g| g.cargo_type == *cargo_type) {
                            g.delivered_since_last_visit.0 += to_unload.0;
                        } else {
                            let mut g = transport_world::definitions::GoodsEntry::new(*cargo_type);
                            g.delivered_since_last_visit = to_unload;
                            station.goods.push(g);
                        }
                    }
                }
            }
        }
        KernelIntent::EmitEvent(_) => {}
    }
}

/// Replay an entire sequence of journal records onto an initial World state.
pub fn replay_journal(mut world: World, records: &[JournalRecord]) -> World {
    for record in records {
        world.tick = transport_types::Ticks(record.tick);
        for intent in &record.intents {
            apply_intent_to_world(&mut world, intent);
        }
    }
    world.rebuild_derived_indexes();
    world
}

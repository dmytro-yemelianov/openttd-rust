use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use transport_types::{Money, PersonID, StationID};

/// Station admission turnstile gate adapted from OpenRCT2 ride entrance turnstiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationTurnstile {
    pub station_id: StationID,
    /// FIFO queue line of waiting peeps
    pub queue: VecDeque<PersonID>,
    /// Ticket fare charged upon passing through the turnstile
    pub ticket_fare: Money,
    /// Total cumulative passengers admitted through this turnstile
    pub admitted_total: u64,
}

impl StationTurnstile {
    pub fn new(station_id: StationID, ticket_fare: Money) -> Self {
        Self {
            station_id,
            queue: VecDeque::new(),
            ticket_fare,
            admitted_total: 0,
        }
    }

    /// Add a peep to the end of the queue line.
    pub fn enqueue(&mut self, peep_id: PersonID) {
        self.queue.push_back(peep_id);
    }

    /// Number of peeps currently waiting in the queue line.
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Pop the next peep from the front of the FIFO queue.
    pub fn pop_front(&mut self) -> Option<PersonID> {
        let peep = self.queue.pop_front();
        if peep.is_some() {
            self.admitted_total = self.admitted_total.saturating_add(1);
        }
        peep
    }
}

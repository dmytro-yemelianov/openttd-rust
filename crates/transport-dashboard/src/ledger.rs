use serde::{Deserialize, Serialize};
use transport_types::CompanyID;

use crate::rrd::FixedRing;

/// Categorization of financial cash flows.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinancialCategory {
    CargoFreight,
    PassengerFares,
    VehiclePurchase,
    VehicleRunning,
    StationConstruction,
    Demolition,
    LoanInterest,
}

/// Aggregate balance sheet for a discrete simulation timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FinancialPeriod {
    pub tick_start: u32,
    pub tick_end: u32,
    pub revenue_cargo: i64,
    pub revenue_passenger: i64,
    pub cost_running: i64,
    pub cost_construction: i64,
    pub cost_maintenance: i64,
    pub cost_interest: i64,
}

impl FinancialPeriod {
    pub fn new(tick: u32) -> Self {
        Self {
            tick_start: tick,
            tick_end: tick,
            ..Default::default()
        }
    }

    /// Sum of all revenue sources.
    pub fn total_revenue(&self) -> i64 {
        self.revenue_cargo + self.revenue_passenger
    }

    /// Sum of all operational and capital expenditures.
    pub fn total_expenses(&self) -> i64 {
        self.cost_running + self.cost_construction + self.cost_maintenance + self.cost_interest
    }

    /// Net cash flow (Revenue minus Expenses).
    pub fn net_profit(&self) -> i64 {
        self.total_revenue() - self.total_expenses()
    }

    /// Merge another period's numbers into this period.
    pub fn merge(&mut self, other: &FinancialPeriod) {
        self.tick_end = other.tick_end;
        self.revenue_cargo += other.revenue_cargo;
        self.revenue_passenger += other.revenue_passenger;
        self.cost_running += other.cost_running;
        self.cost_construction += other.cost_construction;
        self.cost_maintenance += other.cost_maintenance;
        self.cost_interest += other.cost_interest;
    }
}

/// Multi-scale financial accounting ledger for a transport enterprise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyLedger {
    pub company_id: CompanyID,
    pub current_tick: FinancialPeriod,
    pub current_month_acc: FinancialPeriod,
    pub current_year_acc: FinancialPeriod,
    pub tick_history: FixedRing<FinancialPeriod, 300>,
    pub monthly_history: FixedRing<FinancialPeriod, 120>,
    pub yearly_history: FixedRing<FinancialPeriod, 50>,
}

impl CompanyLedger {
    pub fn new(company_id: CompanyID, initial_tick: u32) -> Self {
        Self {
            company_id,
            current_tick: FinancialPeriod::new(initial_tick),
            current_month_acc: FinancialPeriod::new(initial_tick),
            current_year_acc: FinancialPeriod::new(initial_tick),
            tick_history: FixedRing::new(),
            monthly_history: FixedRing::new(),
            yearly_history: FixedRing::new(),
        }
    }

    /// Book an income or expenditure transaction to the active tick.
    pub fn record_transaction(&mut self, category: FinancialCategory, amount: i64) {
        match category {
            FinancialCategory::CargoFreight => self.current_tick.revenue_cargo += amount,
            FinancialCategory::PassengerFares => self.current_tick.revenue_passenger += amount,
            FinancialCategory::VehicleRunning => self.current_tick.cost_running += amount,
            FinancialCategory::VehiclePurchase | FinancialCategory::StationConstruction => {
                self.current_tick.cost_construction += amount
            }
            FinancialCategory::Demolition => self.current_tick.cost_maintenance += amount,
            FinancialCategory::LoanInterest => self.current_tick.cost_interest += amount,
        }
    }

    /// Close the current tick, push to high-resolution ring, and advance accumulators.
    pub fn tick_advance(&mut self, next_tick: u32) {
        let period = self.current_tick;
        self.tick_history.push(period);
        self.current_month_acc.merge(&period);
        self.current_year_acc.merge(&period);

        self.current_tick = FinancialPeriod::new(next_tick);

        // Standard simulation: 74 ticks = 1 day, ~2200 ticks = 1 month (or 1000 ticks per month in fast mode)
        // Check monthly roll (e.g. every 1000 ticks)
        if next_tick > 0 && next_tick % 1000 == 0 {
            self.monthly_history.push(self.current_month_acc);
            self.current_month_acc = FinancialPeriod::new(next_tick);
        }

        // Check yearly roll (every 12000 ticks)
        if next_tick > 0 && next_tick % 12000 == 0 {
            self.yearly_history.push(self.current_year_acc);
            self.current_year_acc = FinancialPeriod::new(next_tick);
        }
    }
}

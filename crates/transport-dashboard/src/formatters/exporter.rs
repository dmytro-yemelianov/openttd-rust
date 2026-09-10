use crate::ledger::CompanyLedger;
use crate::metrics::TelemetryEngine;

/// Export current state as Prometheus text-based metric exposition.
pub fn export_prometheus(telemetry: &TelemetryEngine, ledger: &CompanyLedger) -> String {
    let mut out = String::new();

    if let Some(s) = telemetry.latest() {
        out.push_str("# HELP transport_sim_tick Current discrete simulation tick\n");
        out.push_str("# TYPE transport_sim_tick counter\n");
        out.push_str(&format!("transport_sim_tick {}\n\n", s.tick));

        out.push_str("# HELP transport_fleet_active Number of active operational vehicles\n");
        out.push_str("# TYPE transport_fleet_active gauge\n");
        out.push_str(&format!(
            "transport_fleet_active{{company=\"{}\"}} {}\n\n",
            ledger.company_id.0, s.active_vehicles
        ));

        out.push_str("# HELP transport_treasury_balance_dollars Current company liquid cash\n");
        out.push_str("# TYPE transport_treasury_balance_dollars gauge\n");
        out.push_str(&format!(
            "transport_treasury_balance_dollars{{company=\"{}\"}} {}\n\n",
            ledger.company_id.0, s.treasury_balance
        ));

        out.push_str("# HELP transport_passengers_transported_total Cumulative passengers served\n");
        out.push_str("# TYPE transport_passengers_transported_total counter\n");
        out.push_str(&format!(
            "transport_passengers_transported_total{{company=\"{}\"}} {}\n\n",
            ledger.company_id.0, s.total_passengers_served
        ));

        out.push_str("# HELP transport_cargo_delivered_units_total Cumulative cargo units delivered\n");
        out.push_str("# TYPE transport_cargo_delivered_units_total counter\n");
        out.push_str(&format!(
            "transport_cargo_delivered_units_total{{company=\"{}\"}} {}\n\n",
            ledger.company_id.0, s.total_cargo_delivered
        ));

        out.push_str("# HELP transport_town_satisfaction_percentage Average town satisfaction\n");
        out.push_str("# TYPE transport_town_satisfaction_percentage gauge\n");
        out.push_str(&format!(
            "transport_town_satisfaction_percentage {}\n\n",
            s.average_satisfaction
        ));
    }

    let p = ledger.current_month_acc;
    out.push_str("# HELP transport_monthly_revenue_dollars Monthly accrued revenue\n");
    out.push_str("# TYPE transport_monthly_revenue_dollars counter\n");
    out.push_str(&format!(
        "transport_monthly_revenue_dollars{{company=\"{}\"}} {}\n\n",
        ledger.company_id.0,
        p.total_revenue()
    ));

    out.push_str("# HELP transport_monthly_expenses_dollars Monthly accrued expenses\n");
    out.push_str("# TYPE transport_monthly_expenses_dollars counter\n");
    out.push_str(&format!(
        "transport_monthly_expenses_dollars{{company=\"{}\"}} {}\n\n",
        ledger.company_id.0,
        p.total_expenses()
    ));

    out
}

/// Export telemetry and financial state as serialized JSON.
pub fn export_json(telemetry: &TelemetryEngine, ledger: &CompanyLedger) -> Result<String, serde_json::Error> {
    #[derive(serde::Serialize)]
    struct DashboardPayload<'a> {
        latest_telemetry: Option<&'a crate::metrics::TelemetrySnapshot>,
        recent_telemetry_history: Vec<crate::metrics::TelemetrySnapshot>,
        active_ledger: &'a CompanyLedger,
    }

    let payload = DashboardPayload {
        latest_telemetry: telemetry.latest(),
        recent_telemetry_history: telemetry.history.to_vec(),
        active_ledger: ledger,
    };

    serde_json::to_string_pretty(&payload)
}

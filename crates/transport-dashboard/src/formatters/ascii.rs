use crate::ledger::CompanyLedger;
use crate::metrics::TelemetryEngine;

const SPARK_CHARS: &[char] = &[' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a series of integer values as a compact Unicode sparkline string.
pub fn render_sparkline(values: &[i64]) -> String {
    if values.is_empty() {
        return String::new();
    }

    let min = *values.iter().min().unwrap_or(&0);
    let max = *values.iter().max().unwrap_or(&0);
    let range = (max - min).max(1);

    values
        .iter()
        .map(|&v| {
            let normalized = ((v - min) * 7) / range;
            let idx = (normalized as usize).min(7);
            SPARK_CHARS[idx]
        })
        .collect()
}

/// Format the financial ledger as an ASCII table.
pub fn render_financial_table(ledger: &CompanyLedger) -> String {
    let mut out = String::new();
    out.push_str("+-----------------------------------------------------------------------+\n");
    out.push_str("| PERIOD       | REVENUE      | EXPENSES     | NET PROFIT   | CASH FLOW |\n");
    out.push_str("+-----------------------------------------------------------------------+\n");

    let periods = ledger.monthly_history.to_vec();
    if periods.is_empty() {
        // Show current tick accumulator if monthly history has not rolled yet
        let p = ledger.current_month_acc;
        let rev = p.total_revenue();
        let exp = p.total_expenses();
        let net = p.net_profit();
        let sign = if net >= 0 { "+" } else { "" };
        out.push_str(&format!(
            "| Current Mth  | ${:<11} | ${:<11} | {}${:<10} | {:<9} |\n",
            rev,
            exp,
            sign,
            net,
            if net >= 0 { "SURPLUS" } else { "DEFICIT" }
        ));
    } else {
        for (i, p) in periods.iter().enumerate() {
            let rev = p.total_revenue();
            let exp = p.total_expenses();
            let net = p.net_profit();
            let sign = if net >= 0 { "+" } else { "" };
            out.push_str(&format!(
                "| Month {:<6} | ${:<11} | ${:<11} | {}${:<10} | {:<9} |\n",
                i + 1,
                rev,
                exp,
                sign,
                net,
                if net >= 0 { "SURPLUS" } else { "DEFICIT" }
            ));
        }
    }

    out.push_str("+-----------------------------------------------------------------------+\n");
    out
}

/// Render an interactive operational telemetry summary card.
pub fn render_telemetry_card(telemetry: &TelemetryEngine, ledger: &CompanyLedger) -> String {
    let mut out = String::new();
    let latest = telemetry.latest();

    let cash_series: Vec<i64> = telemetry
        .history
        .to_vec()
        .iter()
        .map(|s| s.treasury_balance)
        .collect();
    let sparkline_cash = render_sparkline(&cash_series);

    let sat_series: Vec<i64> = telemetry
        .history
        .to_vec()
        .iter()
        .map(|s| s.average_satisfaction as i64)
        .collect();
    let sparkline_sat = render_sparkline(&sat_series);

    out.push_str("=========================================================================\n");
    out.push_str("                       NETWORK TELEMETRY DASHBOARD                       \n");
    out.push_str("=========================================================================\n");

    if let Some(s) = latest {
        out.push_str(&format!(
            " Sim Tick: {:<8} | Active Fleet: {:<4} | Passengers: {:<8} | Cargo: {:<6}\n",
            s.tick, s.active_vehicles, s.total_passengers_served, s.total_cargo_delivered
        ));
        out.push_str(&format!(
            " Cash Balance: ${:<10} [{}]\n",
            s.treasury_balance, sparkline_cash
        ));
        out.push_str(&format!(
            " Satisfaction: {}%         [{}]\n",
            s.average_satisfaction, sparkline_sat
        ));
    } else {
        out.push_str(" [No Telemetry Data Sampled Yet]\n");
    }

    out.push_str("-------------------------------------------------------------------------\n");
    out.push_str(&render_financial_table(ledger));
    out
}

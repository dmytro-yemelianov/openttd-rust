use crate::oracle::schema::OracleTraceFrame;

/// Exact location and value divergence between Rust simulation and Oracle reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivergencePoint {
    pub tick: u32,
    pub entity: String,
    pub field: String,
    pub rust_value: String,
    pub oracle_value: String,
}

/// Comparison result report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonReport {
    pub total_ticks: usize,
    pub matched_ticks: usize,
    pub first_divergence: Option<DivergencePoint>,
}

impl ComparisonReport {
    pub fn is_equivalent(&self) -> bool {
        self.first_divergence.is_none()
    }
}

/// Differential comparator detecting the exact first point of divergence.
pub struct DivergenceDetector;

impl DivergenceDetector {
    pub fn compare(
        rust_trace: &[OracleTraceFrame],
        oracle_trace: &[OracleTraceFrame],
    ) -> ComparisonReport {
        let min_len = std::cmp::min(rust_trace.len(), oracle_trace.len());
        let mut matched = 0;

        for i in 0..min_len {
            let r = &rust_trace[i];
            let o = &oracle_trace[i];

            if r.tick != o.tick {
                let r_tick = r.tick;
                let o_tick = o.tick;
                return ComparisonReport {
                    total_ticks: min_len,
                    matched_ticks: matched,
                    first_divergence: Some(DivergencePoint {
                        tick: r_tick,
                        entity: "Timeline".into(),
                        field: "tick".into(),
                        rust_value: format!("{r_tick}"),
                        oracle_value: format!("{o_tick}"),
                    }),
                };
            }

            // 1. Compare Companies (treasuries)
            for (cid, &r_money) in &r.companies {
                match o.companies.get(cid) {
                    Some(&o_money) => {
                        if r_money != o_money {
                            return ComparisonReport {
                                total_ticks: min_len,
                                matched_ticks: matched,
                                first_divergence: Some(DivergencePoint {
                                    tick: r.tick,
                                    entity: format!("Company({cid:?})"),
                                    field: "money".into(),
                                    rust_value: format!("{r_money:?}"),
                                    oracle_value: format!("{o_money:?}"),
                                }),
                            };
                        }
                    }
                    None => {
                        return ComparisonReport {
                            total_ticks: min_len,
                            matched_ticks: matched,
                            first_divergence: Some(DivergencePoint {
                                tick: r.tick,
                                entity: format!("Company({cid:?})"),
                                field: "existence".into(),
                                rust_value: "Present".into(),
                                oracle_value: "Missing".into(),
                            }),
                        };
                    }
                }
            }

            // 2. Compare Vehicles (position, state, cargo)
            for (vid, r_v) in &r.vehicles {
                match o.vehicles.get(vid) {
                    Some(o_v) => {
                        if r_v.position != o_v.position {
                            return ComparisonReport {
                                total_ticks: min_len,
                                matched_ticks: matched,
                                first_divergence: Some(DivergencePoint {
                                    tick: r.tick,
                                    entity: format!("Vehicle({vid:?})"),
                                    field: "position".into(),
                                    rust_value: format!("{:?}", r_v.position),
                                    oracle_value: format!("{:?}", o_v.position),
                                }),
                            };
                        }
                        if r_v.state != o_v.state {
                            return ComparisonReport {
                                total_ticks: min_len,
                                matched_ticks: matched,
                                first_divergence: Some(DivergencePoint {
                                    tick: r.tick,
                                    entity: format!("Vehicle({vid:?})"),
                                    field: "state".into(),
                                    rust_value: format!("{:?}", r_v.state),
                                    oracle_value: format!("{:?}", o_v.state),
                                }),
                            };
                        }
                        if r_v.cargo != o_v.cargo {
                            return ComparisonReport {
                                total_ticks: min_len,
                                matched_ticks: matched,
                                first_divergence: Some(DivergencePoint {
                                    tick: r.tick,
                                    entity: format!("Vehicle({vid:?})"),
                                    field: "cargo".into(),
                                    rust_value: format!("{:?}", r_v.cargo),
                                    oracle_value: format!("{:?}", o_v.cargo),
                                }),
                            };
                        }
                    }
                    None => {
                        return ComparisonReport {
                            total_ticks: min_len,
                            matched_ticks: matched,
                            first_divergence: Some(DivergencePoint {
                                tick: r.tick,
                                entity: format!("Vehicle({vid:?})"),
                                field: "existence".into(),
                                rust_value: "Present".into(),
                                oracle_value: "Missing".into(),
                            }),
                        };
                    }
                }
            }

            // 3. Compare Stations (delivered goods)
            for (sid, r_goods) in &r.stations {
                match o.stations.get(sid) {
                    Some(o_goods) => {
                        if r_goods != o_goods {
                            return ComparisonReport {
                                total_ticks: min_len,
                                matched_ticks: matched,
                                first_divergence: Some(DivergencePoint {
                                    tick: r.tick,
                                    entity: format!("Station({sid:?})"),
                                    field: "goods".into(),
                                    rust_value: format!("{r_goods:?}"),
                                    oracle_value: format!("{o_goods:?}"),
                                }),
                            };
                        }
                    }
                    None => {
                        return ComparisonReport {
                            total_ticks: min_len,
                            matched_ticks: matched,
                            first_divergence: Some(DivergencePoint {
                                tick: r.tick,
                                entity: format!("Station({sid:?})"),
                                field: "existence".into(),
                                rust_value: "Present".into(),
                                oracle_value: "Missing".into(),
                            }),
                        };
                    }
                }
            }

            matched += 1;
        }

        if rust_trace.len() != oracle_trace.len() {
            let r_len = rust_trace.len();
            let o_len = oracle_trace.len();
            return ComparisonReport {
                total_ticks: std::cmp::max(r_len, o_len),
                matched_ticks: matched,
                first_divergence: Some(DivergencePoint {
                    tick: matched as u32,
                    entity: "TraceLength".into(),
                    field: "length".into(),
                    rust_value: format!("{r_len}"),
                    oracle_value: format!("{o_len}"),
                }),
            };
        }

        ComparisonReport {
            total_ticks: matched,
            matched_ticks: matched,
            first_divergence: None,
        }
    }
}

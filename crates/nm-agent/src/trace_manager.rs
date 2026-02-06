use std::collections::HashMap;
use std::net::IpAddr;

use nm_common::protocol::TargetConfig;
use uuid::Uuid;

pub struct TraceManager {
    pub targets: HashMap<Uuid, TargetState>,
}

pub struct TargetState {
    pub config: TargetConfig,
    pub session_id: Uuid,
    pub round_counter: u64,
    pub current_route: Vec<Option<IpAddr>>,
    pub previous_rtts: HashMap<u8, u32>,
    pub dest_ip: Option<IpAddr>,
}

impl TraceManager {
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
        }
    }

    pub fn add_target(&mut self, config: TargetConfig, dest_ip: Option<IpAddr>) {
        let session_id = Uuid::new_v4();
        self.targets.insert(config.target_id, TargetState {
            config,
            session_id,
            round_counter: 0,
            current_route: Vec::new(),
            previous_rtts: HashMap::new(),
            dest_ip,
        });
    }

    pub fn remove_target(&mut self, target_id: &Uuid) {
        self.targets.remove(target_id);
    }

    pub fn compute_jitter(&self, target_id: &Uuid, hop_number: u8, current_rtt: u32) -> Option<u32> {
        self.targets.get(target_id).and_then(|state| {
            state.previous_rtts.get(&hop_number).map(|&prev| {
                (current_rtt as i64 - prev as i64).unsigned_abs() as u32
            })
        })
    }
}

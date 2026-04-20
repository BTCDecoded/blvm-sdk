//! Maps EventType to EventPayload variant fields for DI-style handler parameter injection.
//!
//! Each event type maps to a payload variant with the same name. Field names are used
//! to match handler parameters by name.

use std::collections::HashMap;

/// Payload field info: (field_name, is_copy_type)
/// is_copy_type: true for u64, u32, usize, bool, etc. — pass *binding
/// is_copy_type: false for Hash, String, Vec — pass binding (reference)
pub fn payload_fields_for_event(event_ident: &str) -> Option<Vec<(&'static str, bool)>> {
    let map: HashMap<&str, Vec<(&str, bool)>> = [
        // Core blockchain
        ("NewBlock", vec![("block_hash", false), ("height", true)]),
        ("NewTransaction", vec![("tx_hash", false)]),
        ("BlockDisconnected", vec![("hash", false), ("height", true)]),
        ("ChainReorg", vec![("old_tip", false), ("new_tip", false)]),
        // Module lifecycle
        (
            "ModuleLoaded",
            vec![("module_name", false), ("version", false)],
        ),
        (
            "ModuleUnloaded",
            vec![("module_name", false), ("version", false)],
        ),
        (
            "ModuleReloaded",
            vec![
                ("module_name", false),
                ("old_version", false),
                ("new_version", false),
            ],
        ),
        // Mining
        (
            "BlockMined",
            vec![("block_hash", false), ("height", true), ("miner_id", false)],
        ),
        (
            "BlockTemplateUpdated",
            vec![("prev_hash", false), ("height", true), ("tx_count", true)],
        ),
        (
            "MiningDifficultyChanged",
            vec![
                ("old_difficulty", true),
                ("new_difficulty", true),
                ("height", true),
            ],
        ),
        (
            "MiningJobCreated",
            vec![("job_id", false), ("prev_hash", false), ("height", true)],
        ),
        (
            "ShareSubmitted",
            vec![
                ("job_id", false),
                ("share_hash", false),
                ("miner_id", false),
            ],
        ),
        (
            "MergeMiningReward",
            vec![
                ("secondary_chain", false),
                ("reward_amount", true),
                ("block_hash", false),
            ],
        ),
        (
            "MiningPoolConnected",
            vec![("pool_url", false), ("pool_id", false)],
        ),
        (
            "MiningPoolDisconnected",
            vec![("pool_url", false), ("reason", false)],
        ),
        // Network
        (
            "PeerConnected",
            vec![
                ("peer_addr", false),
                ("transport_type", false),
                ("services", true),
                ("version", true),
            ],
        ),
        (
            "PeerDisconnected",
            vec![("peer_addr", false), ("reason", false)],
        ),
        (
            "PeerBanned",
            vec![
                ("peer_addr", false),
                ("reason", false),
                ("ban_duration_seconds", true),
            ],
        ),
        ("PeerUnbanned", vec![("peer_addr", false)]),
        // Mesh (blvm-mesh): serialized mesh frame from a P2P peer
        (
            "MeshPacketReceived",
            vec![("packet_data", false), ("peer_addr", false)],
        ),
        // Mempool
        (
            "MempoolTransactionAdded",
            vec![
                ("tx_hash", false),
                ("fee_rate", true),
                ("mempool_size", true),
            ],
        ),
        (
            "MempoolTransactionRemoved",
            vec![
                ("tx_hash", false),
                ("reason", false),
                ("mempool_size", true),
            ],
        ),
        ("MempoolCleared", vec![("cleared_count", true)]),
        (
            "FeeRateChanged",
            vec![
                ("old_rate", true),
                ("new_rate", true),
                ("target_blocks", true),
            ],
        ),
        // Node lifecycle
        (
            "NodeStartupCompleted",
            vec![("duration_ms", true), ("components", false)],
        ),
        ("NodeShutdownCompleted", vec![("duration_ms", true)]),
        (
            "NodeShutdown",
            vec![("reason", false), ("timeout_seconds", true)],
        ),
        (
            "ConfigLoaded",
            vec![("changed_sections", false), ("config_json", false)],
        ),
        // Maintenance
        (
            "DataMaintenance",
            vec![
                ("operation", false),
                ("urgency", false),
                ("reason", false),
                ("target_age_days", false),
                ("timeout_seconds", false),
            ],
        ),
        (
            "DiskSpaceLow",
            vec![
                ("available_bytes", true),
                ("total_bytes", true),
                ("percent_free", true),
                ("disk_path", false),
            ],
        ),
        (
            "HealthCheck",
            vec![
                ("check_type", false),
                ("node_healthy", true),
                ("health_report", false),
            ],
        ),
    ]
    .into_iter()
    .collect();

    map.get(event_ident).cloned()
}

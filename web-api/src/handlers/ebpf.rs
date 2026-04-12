// Real eBPF handler module — queries kernel state via bpftool.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use super::{check_admin, paginate_json, PaginationQuery};
use crate::AppState;

// ---------------------------------------------------------------------------
// Helper: run bpftool with a 5-second timeout, return parsed JSON
// ---------------------------------------------------------------------------

async fn run_bpftool(args: &[&str]) -> Result<Value, String> {
    // Try local bpftool first
    let fut = tokio::process::Command::new("bpftool").args(args).output();
    match tokio::time::timeout(Duration::from_secs(5), fut).await {
        Ok(Ok(output)) if output.status.success() => {
            return serde_json::from_slice(&output.stdout)
                .map_err(|e| format!("Parse error: {}", e));
        }
        _ => {}
    }

    // Fallback: run bpftool via kubectl exec into a Cilium agent pod
    let mut kubectl_args = vec![
        "exec",
        "-n",
        "kube-system",
        "-l",
        "k8s-app=cilium",
        "-c",
        "cilium-agent",
        "--",
        "bpftool",
    ];
    kubectl_args.extend_from_slice(args);

    let fut = tokio::process::Command::new("kubectl")
        .args(&kubectl_args)
        .output();

    let output = tokio::time::timeout(Duration::from_secs(10), fut)
        .await
        .map_err(|_| "bpftool timed out (tried local and kubectl exec)".to_string())?
        .map_err(|e| format!("bpftool not available locally or via kubectl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "bpftool unavailable (tried local and kubectl exec into cilium-agent): {}",
            stderr.trim()
        ));
    }

    serde_json::from_slice(&output.stdout).map_err(|e| format!("Parse error: {}", e))
}

fn error_response(status: StatusCode, msg: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": msg })))
}

// ---------------------------------------------------------------------------
// Hex-array helpers
// ---------------------------------------------------------------------------

/// Parse a bpftool hex-string array (e.g. ["0x0a","0x00",...]) into bytes.
fn hex_array_to_bytes(arr: &[Value]) -> Vec<u8> {
    arr.iter()
        .filter_map(|v| {
            let s = v.as_str()?;
            u8::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16).ok()
        })
        .collect()
}

fn bytes_to_ipv4(b: &[u8]) -> String {
    if b.len() >= 4 {
        format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
    } else {
        "0.0.0.0".into()
    }
}

fn bytes_to_u16_be(b: &[u8]) -> u16 {
    if b.len() >= 2 {
        u16::from_be_bytes([b[0], b[1]])
    } else {
        0
    }
}

fn bytes_to_u64_le(b: &[u8]) -> u64 {
    b.get(..8)
        .and_then(|s| <[u8; 8]>::try_from(s).ok())
        .map(u64::from_le_bytes)
        .unwrap_or(0)
}

fn bytes_to_u32_le(b: &[u8]) -> u32 {
    b.get(..4)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}

fn protocol_name(p: u8) -> &'static str {
    match p {
        1 => "ICMP",
        6 => "TCP",
        17 => "UDP",
        _ => "OTHER",
    }
}

// ---------------------------------------------------------------------------
// Map lookup helpers
// ---------------------------------------------------------------------------

/// Find the first map whose name contains `substr`.
fn find_map_by_name<'a>(maps: &'a [Value], substr: &str) -> Option<&'a Value> {
    maps.iter().find(|m| {
        m.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.contains(substr))
            .unwrap_or(false)
    })
}

fn map_id(m: &Value) -> Option<u64> {
    m.get("id").and_then(|v| v.as_u64())
}

/// List all Cilium map IDs for validation.
async fn list_cilium_map_ids() -> Result<Vec<u64>, String> {
    let maps = run_bpftool(&["map", "list", "-j"]).await?;
    let arr = maps.as_array().ok_or("unexpected bpftool output")?;
    Ok(arr
        .iter()
        .filter(|m| {
            m.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.contains("cilium"))
                .unwrap_or(false)
        })
        .filter_map(map_id)
        .collect())
}

// ---------------------------------------------------------------------------
// (a) GET /api/v1/ebpf/programs — list real Cilium eBPF programs
// ---------------------------------------------------------------------------

pub async fn list_real_programs(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    let progs = match run_bpftool(&["prog", "list", "-j"]).await {
        Ok(v) => v,
        Err(_) => {
            return Ok(Json(paginate_json(vec![], &params, "programs")));
        }
    };

    let arr = match progs.as_array() {
        Some(a) => a,
        None => {
            return Ok(Json(paginate_json(vec![], &params, "programs")));
        }
    };

    let items: Vec<Value> = arr
        .iter()
        .filter(|p| {
            let name = p.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let ptype = p.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if name.is_empty() {
                return false;
            }
            name.contains("cil") || ptype == "sched_cls" || ptype == "xdp"
        })
        .map(|p| {
            json!({
                "id": p.get("id"),
                "name": p.get("name"),
                "type": p.get("type"),
                "tag": p.get("tag"),
                "run_cnt": p.get("run_cnt").or_else(|| p.get("run_count")),
                "run_time_ns": p.get("run_time_ns"),
                "bytes_xlated": p.get("bytes_xlated"),
                "bytes_jited": p.get("bytes_jited"),
                "loaded_at": p.get("loaded_at"),
            })
        })
        .collect();

    Ok(Json(paginate_json(items, &params, "programs")))
}

// ---------------------------------------------------------------------------
// (b) GET /api/v1/ebpf/maps — list real Cilium eBPF maps
// ---------------------------------------------------------------------------

pub async fn list_real_maps(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    let maps = match run_bpftool(&["map", "list", "-j"]).await {
        Ok(v) => v,
        Err(_) => {
            return Ok(Json(paginate_json(vec![], &params, "maps")));
        }
    };

    let empty = vec![];
    let arr = maps.as_array().unwrap_or(&empty);

    let items: Vec<Value> = arr
        .iter()
        .filter(|m| {
            m.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.contains("cilium"))
                .unwrap_or(false)
        })
        .map(|m| {
            json!({
                "id": m.get("id"),
                "name": m.get("name"),
                "type": m.get("type"),
                "bytes_key": m.get("bytes_key"),
                "bytes_value": m.get("bytes_value"),
                "max_entries": m.get("max_entries"),
            })
        })
        .collect();

    Ok(Json(paginate_json(items, &params, "maps")))
}

// ---------------------------------------------------------------------------
// (c) GET /api/v1/ebpf/programs/:id — full program details
// ---------------------------------------------------------------------------

pub async fn get_program_stats(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<u32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    let id_str = id.to_string();
    match run_bpftool(&["prog", "show", "id", &id_str, "-j"]).await {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    }
}

// ---------------------------------------------------------------------------
// (d) GET /api/v1/ebpf/maps/:id/entries?limit=50
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct MapEntriesQuery {
    pub limit: Option<usize>,
}

pub async fn dump_map_entries(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Path(id): Path<u32>,
    Query(params): Query<MapEntriesQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    // Validate the requested map ID is a known Cilium map
    let valid_ids = match list_cilium_map_ids().await {
        Ok(ids) => ids,
        Err(e) => return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    };
    if !valid_ids.contains(&(id as u64)) {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "Map ID is not a known Cilium map",
        ));
    }

    let id_str = id.to_string();
    let data = match run_bpftool(&["map", "dump", "id", &id_str, "-j"]).await {
        Ok(v) => v,
        Err(e) => return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    };

    let limit = params.limit.unwrap_or(50).min(200);

    let entries: Vec<Value> = data
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .take(limit)
        .map(|entry| {
            json!({
                "key": entry.get("key"),
                "value": entry.get("value"),
            })
        })
        .collect();

    Ok(Json(json!({
        "entries": entries,
        "total": data.as_array().map(|a| a.len()).unwrap_or(0),
        "limit": limit,
    })))
}

// ---------------------------------------------------------------------------
// (e) GET /api/v1/ebpf/conntrack — parsed conntrack entries
// ---------------------------------------------------------------------------

pub async fn get_conntrack(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    let maps = match run_bpftool(&["map", "list", "-j"]).await {
        Ok(v) => v,
        Err(_) => {
            return Ok(Json(json!({
                "entries": [],
                "total": 0,
                "message": "eBPF maps not accessible"
            })));
        }
    };

    let map_arr = maps.as_array().cloned().unwrap_or_default();

    let mut all_entries: Vec<Value> = Vec::new();

    // Look for cilium_ct4_glob* and cilium_ct_any4_ maps
    for m in &map_arr {
        let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if !name.contains("cilium_ct4_glob") && !name.contains("cilium_ct_any4_") {
            continue;
        }
        let mid = match map_id(m) {
            Some(id) => id,
            None => continue,
        };
        let id_str = mid.to_string();
        let dump = match run_bpftool(&["map", "dump", "id", &id_str, "-j"]).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        let entries = dump.as_array().cloned().unwrap_or_default();
        for entry in entries {
            let key_arr = entry
                .get("key")
                .and_then(|k| k.as_array())
                .cloned()
                .unwrap_or_default();
            let val_arr = entry
                .get("value")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let key_bytes = hex_array_to_bytes(&key_arr);
            let val_bytes = hex_array_to_bytes(&val_arr);

            if key_bytes.len() < 13 {
                continue;
            }

            let dst_ip = bytes_to_ipv4(&key_bytes[0..4]);
            let src_ip = bytes_to_ipv4(&key_bytes[4..8]);
            let dst_port = bytes_to_u16_be(&key_bytes[8..10]);
            let src_port = bytes_to_u16_be(&key_bytes[10..12]);
            let proto = key_bytes[12];

            let rx_packets = if val_bytes.len() >= 8 {
                bytes_to_u64_le(&val_bytes[0..8])
            } else {
                0
            };
            let rx_bytes = if val_bytes.len() >= 16 {
                bytes_to_u64_le(&val_bytes[8..16])
            } else {
                0
            };
            let tx_packets = if val_bytes.len() >= 24 {
                bytes_to_u64_le(&val_bytes[16..24])
            } else {
                0
            };
            let tx_bytes = if val_bytes.len() >= 32 {
                bytes_to_u64_le(&val_bytes[24..32])
            } else {
                0
            };

            all_entries.push(json!({
                "src_ip": src_ip,
                "dst_ip": dst_ip,
                "src_port": src_port,
                "dst_port": dst_port,
                "protocol": protocol_name(proto),
                "protocol_number": proto,
                "rx_packets": rx_packets,
                "tx_packets": tx_packets,
                "rx_bytes": rx_bytes,
                "tx_bytes": tx_bytes,
                "total_packets": rx_packets + tx_packets,
                "total_bytes": rx_bytes + tx_bytes,
            }));
        }
    }

    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);
    let total = all_entries.len();
    let page: Vec<Value> = all_entries.into_iter().skip(offset).take(limit).collect();

    Ok(Json(json!({
        "entries": page,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

// ---------------------------------------------------------------------------
// (f) GET /api/v1/ebpf/ipcache — parsed ipcache entries
// ---------------------------------------------------------------------------

pub async fn get_ipcache(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    let maps = match run_bpftool(&["map", "list", "-j"]).await {
        Ok(v) => v,
        Err(_) => {
            // bpftool not available — return empty result instead of error
            return Ok(Json(json!({
                "entries": [],
                "total": 0,
                "message": "eBPF maps not accessible (bpftool unavailable or no Cilium agent found)"
            })));
        }
    };
    let map_arr = maps.as_array().cloned().unwrap_or_default();

    let ipcache_map = match find_map_by_name(&map_arr, "cilium_ipcache") {
        Some(m) => m,
        None => {
            return Ok(Json(json!({
                "entries": [],
                "total": 0,
                "message": "cilium_ipcache map not found — Cilium may not be installed"
            })));
        }
    };
    let mid = match map_id(ipcache_map) {
        Some(id) => id,
        None => {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "cannot read map id",
            ))
        }
    };

    let id_str = mid.to_string();
    let dump = match run_bpftool(&["map", "dump", "id", &id_str, "-j"]).await {
        Ok(v) => v,
        Err(e) => return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    };

    let entries_arr = dump.as_array().cloned().unwrap_or_default();
    let mut items: Vec<Value> = Vec::new();

    for entry in &entries_arr {
        let key_arr = entry
            .get("key")
            .and_then(|k| k.as_array())
            .cloned()
            .unwrap_or_default();
        let val_arr = entry
            .get("value")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let key_bytes = hex_array_to_bytes(&key_arr);
        let val_bytes = hex_array_to_bytes(&val_arr);

        let ip = if key_bytes.len() >= 4 {
            bytes_to_ipv4(&key_bytes[0..4])
        } else {
            continue;
        };
        let prefix_len = if key_bytes.len() >= 8 {
            bytes_to_u32_le(&key_bytes[4..8])
        } else {
            32
        };
        let identity = if val_bytes.len() >= 4 {
            bytes_to_u32_le(&val_bytes[0..4])
        } else {
            0
        };

        items.push(json!({
            "ip": ip,
            "prefix_len": prefix_len,
            "identity": identity,
        }));
    }

    Ok(Json(paginate_json(items, &params, "entries")))
}

// ---------------------------------------------------------------------------
// (g) GET /api/v1/ebpf/lb — load-balancer backends
// ---------------------------------------------------------------------------

pub async fn get_lb_backends(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    let maps = match run_bpftool(&["map", "list", "-j"]).await {
        Ok(v) => v,
        Err(_) => {
            return Ok(Json(paginate_json(vec![], &params, "services")));
        }
    };
    let map_arr = maps.as_array().cloned().unwrap_or_default();

    let lb_map = match find_map_by_name(&map_arr, "cilium_lb4_serv") {
        Some(m) => m,
        None => {
            return Ok(Json(paginate_json(vec![], &params, "services")));
        }
    };
    let mid = match map_id(lb_map) {
        Some(id) => id,
        None => {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "cannot read map id",
            ))
        }
    };

    let id_str = mid.to_string();
    let dump = match run_bpftool(&["map", "dump", "id", &id_str, "-j"]).await {
        Ok(v) => v,
        Err(e) => return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    };

    let entries_arr = dump.as_array().cloned().unwrap_or_default();
    let mut items: Vec<Value> = Vec::new();

    for entry in &entries_arr {
        let key_arr = entry
            .get("key")
            .and_then(|k| k.as_array())
            .cloned()
            .unwrap_or_default();

        let key_bytes = hex_array_to_bytes(&key_arr);

        let service_ip = if key_bytes.len() >= 4 {
            bytes_to_ipv4(&key_bytes[0..4])
        } else {
            continue;
        };
        let service_port = if key_bytes.len() >= 6 {
            bytes_to_u16_be(&key_bytes[4..6])
        } else {
            0
        };
        let backend_slot = if key_bytes.len() >= 8 {
            bytes_to_u16_be(&key_bytes[6..8])
        } else {
            0
        };
        let proto = if key_bytes.len() >= 9 {
            key_bytes[8]
        } else {
            0
        };

        items.push(json!({
            "service_ip": service_ip,
            "service_port": service_port,
            "backend_slot": backend_slot,
            "protocol": protocol_name(proto),
            "protocol_number": proto,
        }));
    }

    Ok(Json(paginate_json(items, &params, "services")))
}

// ---------------------------------------------------------------------------
// (h) GET /api/v1/ebpf/drops — drop statistics
// ---------------------------------------------------------------------------

fn drop_reason_name(code: u32) -> &'static str {
    match code {
        0 => "Other",
        1 => "PolicyDenied",
        2 => "InvalidPacket",
        3 => "NoRoute",
        4 => "NoTunnel",
        5 => "NoEncap",
        6 => "UnknownL3",
        7 => "MissedTailCall",
        8 => "CTMapFull",
        9 => "InvalidSrcMAC",
        10 => "InvalidDstMAC",
        11 => "AuthRequired",
        _ => "Unknown",
    }
}

pub async fn get_drop_stats(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    let maps = match run_bpftool(&["map", "list", "-j"]).await {
        Ok(v) => v,
        Err(_) => {
            let mut r = paginate_json(vec![], &params, "drops");
            r.as_object_mut().map(|o| o.insert("total_drops".to_string(), json!(0)));
            return Ok(Json(r));
        }
    };
    let map_arr = maps.as_array().cloned().unwrap_or_default();

    let metrics_map = match find_map_by_name(&map_arr, "cilium_metrics") {
        Some(m) => m,
        None => {
            let mut r = paginate_json(vec![], &params, "drops");
            r.as_object_mut().map(|o| o.insert("total_drops".to_string(), json!(0)));
            return Ok(Json(r));
        }
    };
    let mid = match map_id(metrics_map) {
        Some(id) => id,
        None => {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "cannot read map id",
            ))
        }
    };

    let id_str = mid.to_string();
    let dump = match run_bpftool(&["map", "dump", "id", &id_str, "-j"]).await {
        Ok(v) => v,
        Err(e) => return Err(error_response(StatusCode::INTERNAL_SERVER_ERROR, &e)),
    };

    let entries_arr = dump.as_array().cloned().unwrap_or_default();
    let mut items: Vec<Value> = Vec::new();

    for entry in &entries_arr {
        let key_arr = entry
            .get("key")
            .and_then(|k| k.as_array())
            .cloned()
            .unwrap_or_default();
        let val_arr = entry
            .get("value")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let key_bytes = hex_array_to_bytes(&key_arr);
        let val_bytes = hex_array_to_bytes(&val_arr);

        let reason_code = if key_bytes.len() >= 4 {
            bytes_to_u32_le(&key_bytes[0..4])
        } else {
            continue;
        };
        let count = if val_bytes.len() >= 8 {
            bytes_to_u64_le(&val_bytes[0..8])
        } else {
            0
        };
        let bytes_val = if val_bytes.len() >= 16 {
            bytes_to_u64_le(&val_bytes[8..16])
        } else {
            0
        };

        items.push(json!({
            "reason": drop_reason_name(reason_code),
            "reason_code": reason_code,
            "count": count,
            "bytes": bytes_val,
        }));
    }

    let mut result = paginate_json(items, &params, "drops");
    // Add total_drops alias — the frontend expects this field alongside "total"
    if let Some(total) = result.get("total").cloned() {
        result
            .as_object_mut()
            .unwrap()
            .insert("total_drops".to_string(), total);
    }
    Ok(Json(result))
}

// ---------------------------------------------------------------------------
// (i) GET /api/v1/ebpf/summary — aggregate overview
// ---------------------------------------------------------------------------

pub async fn get_ebpf_summary(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    check_admin(&state, &claims)?;

    // Programs
    let total_programs = match run_bpftool(&["prog", "list", "-j"]).await {
        Ok(v) => v
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|p| {
                        let name = p.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        !name.is_empty() && name.contains("cil")
                    })
                    .count()
            })
            .unwrap_or(0),
        Err(_) => 0,
    };

    // Maps
    let maps_data = run_bpftool(&["map", "list", "-j"]).await.ok();
    let map_arr = maps_data
        .as_ref()
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let total_maps = map_arr
        .iter()
        .filter(|m| {
            m.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.contains("cilium"))
                .unwrap_or(false)
        })
        .count();

    // CT entries count
    let mut total_ct_entries: usize = 0;
    for m in &map_arr {
        let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if name.contains("cilium_ct4_glob") || name.contains("cilium_ct_any4_") {
            if let Some(id) = map_id(m) {
                let id_str = id.to_string();
                if let Ok(dump) = run_bpftool(&["map", "dump", "id", &id_str, "-j"]).await {
                    total_ct_entries += dump.as_array().map(|a| a.len()).unwrap_or(0);
                }
            }
        }
    }

    // Drops
    let mut total_drops: u64 = 0;
    let mut top_drop_reason = "None".to_string();
    let mut top_drop_count: u64 = 0;

    if let Some(metrics_map) = find_map_by_name(&map_arr, "cilium_metrics") {
        if let Some(mid) = map_id(metrics_map) {
            let id_str = mid.to_string();
            if let Ok(dump) = run_bpftool(&["map", "dump", "id", &id_str, "-j"]).await {
                for entry in dump.as_array().unwrap_or(&vec![]) {
                    let key_arr = entry
                        .get("key")
                        .and_then(|k| k.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let val_arr = entry
                        .get("value")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();

                    let key_bytes = hex_array_to_bytes(&key_arr);
                    let val_bytes = hex_array_to_bytes(&val_arr);

                    let reason_code = if key_bytes.len() >= 4 {
                        bytes_to_u32_le(&key_bytes[0..4])
                    } else {
                        continue;
                    };
                    let count = if val_bytes.len() >= 8 {
                        bytes_to_u64_le(&val_bytes[0..8])
                    } else {
                        0
                    };

                    total_drops += count;
                    if count > top_drop_count {
                        top_drop_count = count;
                        top_drop_reason = drop_reason_name(reason_code).to_string();
                    }
                }
            }
        }
    }

    Ok(Json(json!({
        "total_programs": total_programs,
        "total_maps": total_maps,
        "total_ct_entries": total_ct_entries,
        "total_drops": total_drops,
        "top_drop_reason": top_drop_reason,
    })))
}

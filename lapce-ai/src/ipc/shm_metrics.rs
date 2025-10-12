/// Prometheus metrics for Shared Memory IPC
/// 
/// Tracks:
/// - Slot claiming (pool size, available, waits)
/// - Connection latency (connect time distribution)
/// - Ring buffer occupancy (space used, backpressure events)
/// - Backpressure waits (duration, frequency)

use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, GaugeVec, HistogramVec, Opts,
};
use lazy_static::lazy_static;

lazy_static! {
    // Connection metrics
    pub static ref SHM_CONNECTIONS_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_connections_total", "Total SHM connections established"),
        &["status"]  // "success", "failed"
    ).unwrap();

    pub static ref SHM_CONNECTIONS_ACTIVE: GaugeVec = register_gauge_vec!(
        Opts::new("shm_connections_active", "Active SHM connections"),
        &["state"]  // "connecting", "established", "closing"
    ).unwrap();

    pub static ref SHM_CONNECT_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "shm_connect_duration_seconds",
        "SHM connection establishment latency",
        &["result"],  // "success", "timeout", "error"
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).unwrap();

    // Slot pool metrics
    pub static ref SHM_SLOTS_TOTAL: GaugeVec = register_gauge_vec!(
        Opts::new("shm_slots_total", "Total SHM slots in pool"),
        &["pool"]  // pool identifier
    ).unwrap();

    pub static ref SHM_SLOTS_AVAILABLE: GaugeVec = register_gauge_vec!(
        Opts::new("shm_slots_available", "Available SHM slots"),
        &["pool"]
    ).unwrap();

    pub static ref SHM_SLOTS_IN_USE: GaugeVec = register_gauge_vec!(
        Opts::new("shm_slots_in_use", "SHM slots currently in use"),
        &["pool"]
    ).unwrap();

    pub static ref SHM_SLOT_CLAIMS_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_slot_claims_total", "Total slot claim attempts"),
        &["result"]  // "success", "pool_full", "error"
    ).unwrap();

    pub static ref SHM_SLOT_CLAIM_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "shm_slot_claim_duration_seconds",
        "Slot claim operation latency",
        &["result"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap();

    // Ring buffer metrics
    pub static ref SHM_RING_OCCUPANCY_RATIO: GaugeVec = register_gauge_vec!(
        Opts::new("shm_ring_occupancy_ratio", "Ring buffer space used (0.0-1.0)"),
        &["buffer", "conn_id"]  // buffer="send"/"recv"
    ).unwrap();

    pub static ref SHM_RING_WRITES_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_ring_writes_total", "Total ring buffer writes"),
        &["buffer", "result"]  // result="success", "would_block", "error"
    ).unwrap();

    pub static ref SHM_RING_READS_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_ring_reads_total", "Total ring buffer reads"),
        &["buffer", "result"]  // result="success", "empty", "error"
    ).unwrap();

    pub static ref SHM_RING_BYTES_WRITTEN: CounterVec = register_counter_vec!(
        Opts::new("shm_ring_bytes_written", "Bytes written to ring buffers"),
        &["buffer"]
    ).unwrap();

    pub static ref SHM_RING_BYTES_READ: CounterVec = register_counter_vec!(
        Opts::new("shm_ring_bytes_read", "Bytes read from ring buffers"),
        &["buffer"]
    ).unwrap();

    // Backpressure metrics
    pub static ref SHM_BACKPRESSURE_EVENTS_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_backpressure_events_total", "Backpressure events (buffer full)"),
        &["buffer", "resolution"]  // resolution="retried", "failed", "timeout"
    ).unwrap();

    pub static ref SHM_BACKPRESSURE_WAIT_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "shm_backpressure_wait_duration_seconds",
        "Time spent waiting during backpressure",
        &["buffer", "result"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap();

    pub static ref SHM_BACKPRESSURE_RETRIES: HistogramVec = register_histogram_vec!(
        "shm_backpressure_retries",
        "Number of retries during backpressure",
        &["buffer"],
        vec![1.0, 2.0, 3.0, 5.0, 10.0, 20.0, 50.0, 100.0]
    ).unwrap();

    // Message throughput metrics
    pub static ref SHM_MESSAGES_SENT_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_messages_sent_total", "Total messages sent"),
        &["msg_type"]
    ).unwrap();

    pub static ref SHM_MESSAGES_RECEIVED_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_messages_received_total", "Total messages received"),
        &["msg_type"]
    ).unwrap();

    pub static ref SHM_MESSAGE_SIZE_BYTES: HistogramVec = register_histogram_vec!(
        "shm_message_size_bytes",
        "Message payload size distribution",
        &["direction"],  // "send", "recv"
        vec![64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0]
    ).unwrap();

    // Latency metrics
    pub static ref SHM_WRITE_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "shm_write_duration_seconds",
        "Write operation latency",
        &["result"],
        vec![0.000001, 0.000005, 0.00001, 0.00005, 0.0001, 0.0005, 0.001, 0.005, 0.01]
    ).unwrap();

    pub static ref SHM_READ_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "shm_read_duration_seconds",
        "Read operation latency",
        &["result"],
        vec![0.000001, 0.000005, 0.00001, 0.00005, 0.0001, 0.0005, 0.001, 0.005, 0.01]
    ).unwrap();

    pub static ref SHM_ROUNDTRIP_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "shm_roundtrip_duration_seconds",
        "Full round-trip (write + read) latency",
        &["msg_type"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();

    // Error tracking
    pub static ref SHM_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        Opts::new("shm_errors_total", "Total SHM errors"),
        &["error_type", "operation"]  // operation="connect", "write", "read", "accept"
    ).unwrap();

    // Lock file watcher metrics
    pub static ref SHM_LOCK_FILES_DETECTED: CounterVec = register_counter_vec!(
        Opts::new("shm_lock_files_detected", "Lock files detected by watcher"),
        &["action"]  // "accepted", "ignored", "stale"
    ).unwrap();

    pub static ref SHM_LOCK_FILE_SCAN_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "shm_lock_file_scan_duration_seconds",
        "Lock file directory scan latency",
        &[],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap();

    // Crash recovery metrics
    pub static ref SHM_STALE_LOCKS_CLEANED: CounterVec = register_counter_vec!(
        Opts::new("shm_stale_locks_cleaned", "Stale lock files cleaned at startup"),
        &["reason"]  // "timeout", "orphaned", "corrupted"
    ).unwrap();

    pub static ref SHM_ORPHANED_SLOTS_RECLAIMED: CounterVec = register_counter_vec!(
        Opts::new("shm_orphaned_slots_reclaimed", "Orphaned slots reclaimed"),
        &["reason"]  // "ttl_expired", "manual_gc"
    ).unwrap();
}

/// Helper functions for common metric operations
pub mod helpers {
    use super::*;
    use std::time::Instant;

    pub struct ConnectionTimer {
        start: Instant,
    }

    impl ConnectionTimer {
        pub fn new() -> Self {
            Self { start: Instant::now() }
        }

        pub fn record_success(self) {
            let duration = self.start.elapsed().as_secs_f64();
            SHM_CONNECT_DURATION_SECONDS.with_label_values(&["success"]).observe(duration);
            SHM_CONNECTIONS_TOTAL.with_label_values(&["success"]).inc();
        }

        pub fn record_failure(self, reason: &str) {
            let duration = self.start.elapsed().as_secs_f64();
            SHM_CONNECT_DURATION_SECONDS.with_label_values(&[reason]).observe(duration);
            SHM_CONNECTIONS_TOTAL.with_label_values(&["failed"]).inc();
        }
    }

    pub struct BackpressureTimer {
        start: Instant,
        buffer: String,
    }

    impl BackpressureTimer {
        pub fn new(buffer: &str) -> Self {
            SHM_BACKPRESSURE_EVENTS_TOTAL.with_label_values(&[buffer, "started"]).inc();
            Self {
                start: Instant::now(),
                buffer: buffer.to_string(),
            }
        }

        pub fn record_resolved(self, retries: u32) {
            let duration = self.start.elapsed().as_secs_f64();
            SHM_BACKPRESSURE_WAIT_DURATION_SECONDS
                .with_label_values(&[&self.buffer, "resolved"])
                .observe(duration);
            SHM_BACKPRESSURE_RETRIES
                .with_label_values(&[&self.buffer])
                .observe(retries as f64);
            SHM_BACKPRESSURE_EVENTS_TOTAL
                .with_label_values(&[&self.buffer, "retried"])
                .inc();
        }

        pub fn record_failed(self) {
            let duration = self.start.elapsed().as_secs_f64();
            SHM_BACKPRESSURE_WAIT_DURATION_SECONDS
                .with_label_values(&[&self.buffer, "failed"])
                .observe(duration);
            SHM_BACKPRESSURE_EVENTS_TOTAL
                .with_label_values(&[&self.buffer, "failed"])
                .inc();
        }
    }

    pub fn record_write_success(buffer: &str, bytes: usize, duration_secs: f64) {
        SHM_RING_WRITES_TOTAL.with_label_values(&[buffer, "success"]).inc();
        SHM_RING_BYTES_WRITTEN.with_label_values(&[buffer]).inc_by(bytes as f64);
        SHM_WRITE_DURATION_SECONDS.with_label_values(&["success"]).observe(duration_secs);
        SHM_MESSAGE_SIZE_BYTES.with_label_values(&["send"]).observe(bytes as f64);
    }

    pub fn record_read_success(buffer: &str, bytes: usize, duration_secs: f64) {
        SHM_RING_READS_TOTAL.with_label_values(&[buffer, "success"]).inc();
        SHM_RING_BYTES_READ.with_label_values(&[buffer]).inc_by(bytes as f64);
        SHM_READ_DURATION_SECONDS.with_label_values(&["success"]).observe(duration_secs);
        SHM_MESSAGE_SIZE_BYTES.with_label_values(&["recv"]).observe(bytes as f64);
    }

    pub fn update_ring_occupancy(buffer: &str, conn_id: u64, used: usize, capacity: usize) {
        let ratio = used as f64 / capacity as f64;
        SHM_RING_OCCUPANCY_RATIO
            .with_label_values(&[buffer, &conn_id.to_string()])
            .set(ratio);
    }
}

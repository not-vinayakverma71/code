use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lapce_ai_rust::monitoring::{
    AtomicMetrics, LatencyTracker, MetricsCollector, PerformanceMonitor,
    Tracer, Profiler,
};
use std::time::Duration;

fn benchmark_atomic_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("atomic_metrics");
    
    let metrics = AtomicMetrics::new();
    
    group.bench_function("record_request", |b| {
        b.iter(|| {
            metrics.record_request(
                black_box(Duration::from_millis(100)),
                black_box(true)
            );
        });
    });
    
    group.bench_function("record_bytes", |b| {
        b.iter(|| {
            metrics.record_bytes(
                black_box(1024),
                black_box(2048)
            );
        });
    });
    
    group.bench_function("record_cache_access", |b| {
        b.iter(|| {
            metrics.record_cache_access(black_box(true));
        });
    });
    
    group.bench_function("get_success_rate", |b| {
        // Pre-populate some data
        for _ in 0..1000 {
            metrics.record_request(Duration::from_millis(10), true);
        }
        
        b.iter(|| {
            black_box(metrics.get_success_rate());
        });
    });
    
    group.finish();
}

fn benchmark_latency_tracker(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_tracker");
    
    let tracker = LatencyTracker::new();
    
    group.bench_function("record_single", |b| {
        b.iter(|| {
            tracker.record(black_box(100));
        });
    });
    
    group.bench_function("record_duration", |b| {
        b.iter(|| {
            tracker.record_duration(black_box(Duration::from_micros(100)));
        });
    });
    
    // Benchmark with different batch sizes
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("record_batch", size),
            size,
            |b, &size| {
                b.iter(|| {
                    for i in 0..size {
                        tracker.record(black_box(i as u64 * 100));
                    }
                });
            },
        );
    }
    
    group.bench_function("get_percentiles", |b| {
        // Pre-populate with data
        for i in 0..10000 {
            tracker.record(i % 1000);
        }
        
        b.iter(|| {
            black_box(tracker.get_percentiles());
        });
    });
    
    group.finish();
}

fn benchmark_metrics_collector(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_collector");
    
    let collector = MetricsCollector::new().unwrap();
    
    group.bench_function("record_request", |b| {
        b.iter(|| {
            collector.record_request(
                black_box(Duration::from_millis(10)),
                black_box(1024),
                black_box(true)
            );
        });
    });
    
    group.bench_function("record_cache_access", |b| {
        b.iter(|| {
            collector.record_cache_access(black_box(true));
        });
    });
    
    group.bench_function("record_model_inference", |b| {
        b.iter(|| {
            collector.record_model_inference(
                black_box(Duration::from_secs(1)),
                black_box(100)
            );
        });
    });
    
    group.bench_function("update_system_metrics", |b| {
        b.iter(|| {
            collector.update_system_metrics(
                black_box(1024 * 1024),
                black_box(25.5),
                black_box(10)
            );
        });
    });
    
    group.finish();
}

fn benchmark_tracer(c: &mut Criterion) {
    let mut group = c.benchmark_group("tracer");
    
    let tracer = Tracer::with_config(1000, 1.0); // Always sample for benchmarking
    
    group.bench_function("start_trace", |b| {
        b.iter(|| {
            if let Some(span) = tracer.start_trace("test_op".to_string()) {
                let span_id = span.read().id.clone();
                tracer.finish_span(&span_id);
            }
        });
    });
    
    group.bench_function("nested_spans", |b| {
        b.iter(|| {
            if let Some(root) = tracer.start_trace("root".to_string()) {
                let root_id = root.read().id.clone();
                
                for i in 0..3 {
                    if let Some(child) = tracer.start_span(format!("child_{}", i)) {
                        let child_id = child.read().id.clone();
                        tracer.finish_span(&child_id);
                    }
                }
                
                tracer.finish_span(&root_id);
            }
        });
    });
    
    group.finish();
}

fn benchmark_profiler(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiler");
    
    let profiler = Profiler::new();
    profiler.enable();
    
    group.bench_function("function_profiling", |b| {
        b.iter(|| {
            if let Some(_guard) = profiler.start_function("test_func".to_string()) {
                // Function body simulation
                black_box(std::thread::sleep(Duration::from_nanos(1)));
            }
        });
    });
    
    group.bench_function("record_cpu_sample", |b| {
        b.iter(|| {
            profiler.record_cpu_sample(
                vec!["func1".to_string(), "func2".to_string()],
                Duration::from_millis(10)
            );
        });
    });
    
    group.bench_function("record_memory_sample", |b| {
        b.iter(|| {
            profiler.record_memory_sample(
                1024,
                512,
                2048,
                vec!["alloc_site".to_string()]
            );
        });
    });
    
    group.finish();
}

fn benchmark_concurrent_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_metrics");
    
    let metrics = std::sync::Arc::new(AtomicMetrics::new());
    
    for num_threads in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("atomic_metrics_parallel", num_threads),
            num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            let metrics = metrics.clone();
                            std::thread::spawn(move || {
                                for _ in 0..100 {
                                    metrics.record_request(
                                        Duration::from_millis(10),
                                        true
                                    );
                                }
                            })
                        })
                        .collect();
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_overhead");
    
    group.bench_function("monitor_creation", |b| {
        b.iter(|| {
            black_box(PerformanceMonitor::new().unwrap());
        });
    });
    
    group.bench_function("latency_tracker_creation", |b| {
        b.iter(|| {
            black_box(LatencyTracker::new());
        });
    });
    
    group.bench_function("metrics_collector_creation", |b| {
        b.iter(|| {
            black_box(MetricsCollector::new().unwrap());
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_atomic_metrics,
    benchmark_latency_tracker,
    benchmark_metrics_collector,
    benchmark_tracer,
    benchmark_profiler,
    benchmark_concurrent_metrics,
    benchmark_memory_overhead
);

criterion_main!(benches);

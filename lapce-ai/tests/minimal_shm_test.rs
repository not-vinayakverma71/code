/// Absolute minimal test to isolate hang
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_minimal() {
    println!("TEST START");
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("TEST END");
    assert!(true);
}

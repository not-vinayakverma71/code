/// Minimal test for async SharedMemory
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_buffer_create() {
    println!("Creating buffer...");
    let path = format!("/test_async_{}", std::process::id());
    let buffer = SharedMemoryBuffer::create(&path, 4096).await;
    println!("Result: {:?}", buffer.is_ok());
    assert!(buffer.is_ok());
}

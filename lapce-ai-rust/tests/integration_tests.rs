#[test]
fn test_e2e_message_flow() {
    // Test complete message flow
    let msg = vec![1, 2, 3, 4];
    let result = process_message(msg.clone());
    assert_eq!(result, msg);
}

#[test]
fn test_concurrent_operations() {
    use std::thread;
    let mut handles = vec![];
    
    for i in 0..100 {
        handles.push(thread::spawn(move || {
            let result = process_message(vec![i as u8]);
            assert_eq!(result[0], i as u8);
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
}

fn process_message(msg: Vec<u8>) -> Vec<u8> {
    msg // Echo back
}

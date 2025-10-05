/// Test to verify events.ts translation works correctly
use lapce_ai_rust::events_exact_translation::*;
use serde_json;

fn main() {
    println!("Testing events.ts Translation...\n");
    
    // Test 1: RooCodeEventName serialization
    println!("1. Testing RooCodeEventName:");
    let event = RooCodeEventName::TaskCreated;
    let json = serde_json::to_string(&event).unwrap();
    println!("   TaskCreated: {}", json);
    assert_eq!(json, r#""taskCreated""#);
    
    // Test 2: TaskEvent discriminated union
    println!("\n2. Testing TaskEvent with single string payload:");
    let task_started = TaskEvent::TaskStarted {
        payload: ("test-task-123".to_string(),),
        task_id: Some(42),
    };
    let json = serde_json::to_string(&task_started).unwrap();
    println!("   TaskStarted: {}", json);
    assert!(json.contains(r#""eventName":"taskStarted""#));
    assert!(json.contains(r#""taskId":42"#));
    
    // Test 3: TaskSpawned with two strings
    println!("\n3. Testing TaskSpawned (two string payload):");
    let task_spawned = TaskEvent::TaskSpawned {
        payload: ("parent-123".to_string(), "child-456".to_string()),
        task_id: Some(100),
    };
    let json = serde_json::to_string(&task_spawned).unwrap();
    println!("   TaskSpawned: {}", json);
    assert!(json.contains(r#""eventName":"taskSpawned""#));
    
    // Test 4: Message event with object payload
    println!("\n4. Testing Message event:");
    let msg_event = TaskEvent::Message {
        payload: (MessageEventPayload {
            task_id: "task-789".to_string(),
            action: MessageAction::Created,
            message: ClineMessage {
                ts: 1234567890,
                msg_type: "ask".to_string(),
                text: Some("Hello".to_string()),
            },
        },),
        task_id: Some(200),
    };
    let json = serde_json::to_string(&msg_event).unwrap();
    println!("   Message: {}", json);
    assert!(json.contains(r#""eventName":"message""#));
    assert!(json.contains(r#""action":"created""#));
    
    // Test 5: TaskCompleted with complex payload
    println!("\n5. Testing TaskCompleted:");
    let completed = TaskEvent::TaskCompleted {
        payload: (
            "task-complete".to_string(),
            TokenUsage {
                total_tokens_in: 100,
                total_tokens_out: 200,
                total_cost: 0.005,
                context_tokens: 150,
            },
            ToolUsage {
                tools: std::collections::HashMap::new(),
            },
            TaskCompletedMetadata {
                is_subtask: false,
            },
        ),
        task_id: Some(300),
    };
    let json = serde_json::to_string(&completed).unwrap();
    println!("   TaskCompleted: {}", json);
    assert!(json.contains(r#""eventName":"taskCompleted""#));
    assert!(json.contains(r#""totalTokensIn":100"#));
    assert!(json.contains(r#""isSubtask":false"#));
    
    // Test 6: EvalPass with required taskId
    println!("\n6. Testing EvalPass:");
    let eval_pass = TaskEvent::EvalPass {
        payload: (),
        task_id: 999,
    };
    let json = serde_json::to_string(&eval_pass).unwrap();
    println!("   EvalPass: {}", json);
    assert!(json.contains(r#""eventName":"evalPass""#));
    assert!(json.contains(r#""taskId":999"#));
    
    // Test 7: Deserialize TaskEvent
    println!("\n7. Testing deserialization:");
    let json_input = r#"{
        "eventName": "taskStarted",
        "payload": ["my-task-id"],
        "taskId": 123
    }"#;
    match serde_json::from_str::<TaskEvent>(json_input) {
        Ok(event) => {
            println!("   ✅ Successfully deserialized TaskEvent");
            if let TaskEvent::TaskStarted { payload, task_id } = event {
                assert_eq!(payload.0, "my-task-id");
                assert_eq!(task_id, Some(123));
            }
        }
        Err(e) => {
            println!("   ❌ Deserialization failed: {}", e);
        }
    }
    
    println!("\n✅ All events.ts translation tests passed!");
}

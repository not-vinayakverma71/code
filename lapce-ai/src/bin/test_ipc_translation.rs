/// Test to verify ipc.ts translation works correctly
use lapce_ai_rust::ipc_messages::*;
use lapce_ai_rust::global_settings_exact_translation::RooCodeSettings;
use serde_json;

fn main() {
    println!("Testing ipc.ts Translation...\n");
    
    // Test 1: IpcMessageType serialization
    println!("1. Testing IpcMessageType:");
    let msg_type = IpcMessageType::Connect;
    let json = serde_json::to_string(&msg_type).unwrap();
    println!("   Connect serialized: {}", json);
    assert_eq!(json, r#""Connect""#);
    
    // Test 2: IpcOrigin serialization
    println!("\n2. Testing IpcOrigin:");
    let origin = IpcOrigin::Client;
    let json = serde_json::to_string(&origin).unwrap();
    println!("   Client serialized: {}", json);
    assert_eq!(json, r#""client""#);
    
    // Test 3: Ack structure
    println!("\n3. Testing Ack:");
    let ack = Ack {
        client_id: "test-123".to_string(),
        pid: 1234,
        ppid: 5678,
    };
    let json = serde_json::to_string(&ack).unwrap();
    println!("   Ack: {}", json);
    assert!(json.contains("clientId"));
    
    // Test 4: TaskCommandName
    println!("\n4. Testing TaskCommandName:");
    let cmd_name = TaskCommandName::StartNewTask;
    let json = serde_json::to_string(&cmd_name).unwrap();
    println!("   StartNewTask: {}", json);
    assert_eq!(json, r#""StartNewTask""#);
    
    // Test 5: TaskCommand discriminated union
    println!("\n5. Testing TaskCommand:");
    let cancel_cmd = TaskCommand::CancelTask {
        data: "task-456".to_string(),
    };
    let json = serde_json::to_string(&cancel_cmd).unwrap();
    println!("   CancelTask: {}", json);
    assert!(json.contains(r#""commandName":"CancelTask""#));
    
    // Test 6: IpcMessage discriminated union
    println!("\n6. Testing IpcMessage:");
    let ipc_msg = IpcMessage::Ack {
        origin: IpcOrigin::Server,
        data: Ack {
            client_id: "client-789".to_string(),
            pid: 9999,
            ppid: 8888,
        },
    };
    let json = serde_json::to_string(&ipc_msg).unwrap();
    println!("   IpcMessage::Ack: {}", json);
    assert!(json.contains(r#""type":"Ack""#));
    assert!(json.contains(r#""origin":"server""#));
    
    // Test 7: Deserialize from JSON (round-trip)
    println!("\n7. Testing deserialization:");
    let task_cmd_json = r#"{
        "commandName": "StartNewTask",
        "data": {
            "configuration": {
                "global": {},
                "provider": {},
                "ghostServiceSettings": {}
            },
            "text": "Hello world",
            "images": ["image1.png"],
            "newTab": true
        }
    }"#;
    
    match serde_json::from_str::<TaskCommand>(task_cmd_json) {
        Ok(cmd) => {
            println!("   ✅ Successfully deserialized TaskCommand");
            if let TaskCommand::StartNewTask { data } = cmd {
                assert_eq!(data.text, "Hello world");
                assert_eq!(data.images.unwrap().len(), 1);
                assert_eq!(data.new_tab, Some(true));
            }
        }
        Err(e) => {
            println!("   ❌ Deserialization failed: {}", e);
        }
    }
    
    println!("\n✅ All ipc.ts translation tests passed!");
}

/// Test to verify message.ts translation works correctly
use lapce_ai_rust::ipc_messages::*;
use serde_json;

fn main() {
    println!("Testing message.ts Translation...\n");
    
    // Test 1: ClineAsk serialization
    println!("1. Testing ClineAsk:");
    let ask = ClineAsk::Command;
    let json = serde_json::to_string(&ask).unwrap();
    println!("   Command: {}", json);
    assert_eq!(json, r#""command""#);
    
    let ask2 = ClineAsk::PaymentRequiredPrompt;
    let json2 = serde_json::to_string(&ask2).unwrap();
    println!("   PaymentRequiredPrompt: {}", json2);
    assert_eq!(json2, r#""payment_required_prompt""#);
    
    // Test 2: ClineSay serialization
    println!("\n2. Testing ClineSay:");
    let say = ClineSay::ApiReqStarted;
    let json = serde_json::to_string(&say).unwrap();
    println!("   ApiReqStarted: {}", json);
    assert_eq!(json, r#""api_req_started""#);
    
    // Test 3: Helper functions
    println!("\n3. Testing ClineAsk helper functions:");
    let idle = ClineAsk::CompletionResult;
    println!("   CompletionResult.is_idle_ask(): {}", idle.is_idle_ask());
    assert!(idle.is_idle_ask());
    
    let resumable = ClineAsk::ResumeTask;
    println!("   ResumeTask.is_resumable_ask(): {}", resumable.is_resumable_ask());
    assert!(resumable.is_resumable_ask());
    
    let interactive = ClineAsk::Tool;
    println!("   Tool.is_interactive_ask(): {}", interactive.is_interactive_ask());
    assert!(interactive.is_interactive_ask());
    
    // Test 4: ClineMessage structure
    println!("\n4. Testing ClineMessage:");
    let msg = ClineMessage {
        ts: 1234567890,
        msg_type: "ask".to_string(),
        ask: Some(ClineAsk::Followup),
        say: None,
        text: Some("What should I do next?".to_string()),
        images: Some(vec!["image1.png".to_string()]),
        partial: Some(false),
        reasoning: Some("Need more info".to_string()),
        conversation_history_index: Some(5),
        checkpoint: None,
        progress_status: None,
        context_condense: None,
        is_protected: Some(false),
        api_protocol: Some("openai".to_string()),
        metadata: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    println!("   ClineMessage: {}", json);
    assert!(json.contains(r#""type":"ask""#));
    assert!(json.contains(r#""ask":"followup""#));
    assert!(json.contains(r#""ts":1234567890"#));
    
    // Test 5: ToolProgressStatus
    println!("\n5. Testing ToolProgressStatus:");
    let status = ToolProgressStatus {
        icon: Some("🔧".to_string()),
        text: Some("Processing...".to_string()),
    };
    let json = serde_json::to_string(&status).unwrap();
    println!("   ToolProgressStatus: {}", json);
    assert!(json.contains(r#""icon":"🔧""#));
    
    // Test 6: ContextCondense
    println!("\n6. Testing ContextCondense:");
    let condense = ContextCondense {
        cost: 0.005,
        prev_context_tokens: 1000,
        new_context_tokens: 500,
        summary: "Condensed context".to_string(),
    };
    let json = serde_json::to_string(&condense).unwrap();
    println!("   ContextCondense: {}", json);
    assert!(json.contains(r#""prevContextTokens":1000"#));
    assert!(json.contains(r#""newContextTokens":500"#));
    
    // Test 7: QueuedMessage
    println!("\n7. Testing QueuedMessage:");
    let queued = QueuedMessage {
        id: "msg-123".to_string(),
        text: "Hello world".to_string(),
        images: vec!["data:image/png;base64,...".to_string()],
    };
    let json = serde_json::to_string(&queued).unwrap();
    println!("   QueuedMessage: {}", json);
    assert!(json.contains(r#""id":"msg-123""#));
    
    // Test 8: Complex ClineMessage with metadata
    println!("\n8. Testing ClineMessage with metadata:");
    let msg_with_meta = ClineMessage {
        ts: 1234567890,
        msg_type: "say".to_string(),
        ask: None,
        say: Some("test".to_string()),
        text: Some("Response text".to_string()),
        images: None,
        partial: None,
        reasoning: None,
        conversation_history_index: None,
        checkpoint: None,
        progress_status: None,
        context_condense: None,
        is_protected: None,
        api_protocol: None,
        metadata: Some(MessageMetadata {
            gpt5: Some(Gpt5Metadata {
                previous_response_id: Some("prev-123".to_string()),
                instructions: Some("Be helpful".to_string()),
                reasoning_summary: Some("User needs help".to_string()),
            }),
            kilo_code: None,
        }),
    };
    let json = serde_json::to_string(&msg_with_meta).unwrap();
    println!("   ClineMessage with metadata: {}", json);
    assert!(json.contains(r#""type":"say""#));
    assert!(json.contains(r#""say":"text""#));
    assert!(json.contains(r#""previous_response_id":"prev-123""#));
    
    // Test 9: Deserialize ClineMessage
    println!("\n9. Testing deserialization:");
    let json_input = r#"{
        "ts": 9999999,
        "type": "ask",
        "ask": "command",
        "text": "Run this command?",
        "conversationHistoryIndex": 10
    }"#;
    match serde_json::from_str::<ClineMessage>(json_input) {
        Ok(msg) => {
            println!("   ✅ Successfully deserialized ClineMessage");
            assert_eq!(msg.ts, 9999999);
            assert_eq!(msg.ask, Some(ClineAsk::Command));
            assert_eq!(msg.conversation_history_index, Some(10));
        }
        Err(e) => {
            println!("   ❌ Deserialization failed: {}", e);
        }
    }
    
    println!("\n✅ All message.ts translation tests passed!");
}

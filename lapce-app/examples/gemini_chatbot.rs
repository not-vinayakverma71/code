use floem::{
    reactive::{create_effect, create_rw_signal, RwSignal, SignalGet, SignalUpdate},
    views::{container, dyn_stack, h_stack, label, scroll, text_input, v_stack, Decorators},
    IntoView,
};
use floem::ext_event::{register_ext_trigger, ExtSendTrigger};
use floem::quit_app;
use floem::peniko::Color;
use serde_json::{json, Value};
use std::io::Read;
use std::env;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const GEMINI_MODEL: &str = "gemini-2.5-flash";

fn gemini_api_key() -> Result<String, String> {
    env::var("GEMINI_API_KEY").map_err(|_| "GEMINI_API_KEY not set".to_string())
}

fn gemini_model() -> String {
    env::var("GEMINI_MODEL").unwrap_or_else(|_| GEMINI_MODEL.to_string())
}

const TEST_MESSAGE: &str = "hi"; // Auto-send on launch

#[derive(Clone, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Clone, Debug)]
enum StreamEvent {
    Chunk(String),
    Done,
}

fn main() {
    floem::launch(app_view);
}

fn start_stream_session(
    messages: RwSignal<Vec<Message>>,
    typing: RwSignal<String>,
    prompt: String,
    chunk_count: RwSignal<usize>,
) {
    println!("[UI] Sending: {}", prompt);

    // Add user message
    messages.update(|msgs| {
        msgs.push(Message {
            role: "user".to_string(),
            content: prompt.clone(),
        });
    });

    // Clear typing area
    typing.set(String::new());

    // Internal event queue + trigger to avoid coalescing; process all events per idle
    let queue: Arc<Mutex<VecDeque<StreamEvent>>> = Arc::new(Mutex::new(VecDeque::new()));
    let trigger = ExtSendTrigger::new();

    let messages_for_effect = messages;
    let typing_for_effect = typing;
    let auto_mode = env::var("GEMINI_UI_TEST").ok().is_some();
    let queue_effect = queue.clone();
    create_effect(move |_| {
        // Wake on external trigger
        trigger.track();

        // Drain all pending events and update UI incrementally
        loop {
            let ev_opt = queue_effect.lock().unwrap().pop_front();
            let Some(event) = ev_opt else { break };
            match event {
                StreamEvent::Chunk(chunk) => {
                    println!("[UI] chunk {} bytes", chunk.len());
                    chunk_count.update(|c| *c += 1);
                    typing_for_effect.update(|t| t.push_str(&chunk));

                    if auto_mode {
                        let chunks = chunk_count.get();
                        if chunks >= 2 {
                            println!("TEST_RESULT streaming_chunks={} pass=true", chunks);
                            quit_app();
                        }
                    }
                }
                StreamEvent::Done => {
                    println!("[UI] done");
                    let final_text = typing_for_effect.get();
                    if !final_text.is_empty() {
                        messages_for_effect.update(|msgs| {
                            msgs.push(Message { role: "assistant".to_string(), content: final_text.clone() });
                        });
                    }
                    typing_for_effect.set(String::new());

                    if auto_mode {
                        let chunks = chunk_count.get();
                        let pass = chunks >= 2;
                        println!("TEST_RESULT streaming_chunks={} pass={}", chunks, pass);
                        quit_app();
                    }
                }
            }
        }
    });

    // Spawn background thread to stream SSE and send chunks
    let prompt_clone = prompt.clone();
    let queue_publish = queue.clone();
    let queue_done = queue.clone();
    let trigger_bg = trigger; // ExtSendTrigger is Copy
    std::thread::spawn(move || {
        if let Err(e) = stream_gemini_sse(&prompt_clone, move |ev| {
            queue_publish.lock().unwrap().push_back(ev);
            register_ext_trigger(trigger_bg);
        }) {
            eprintln!("[SSE] Error: {}", e);
        }
        // Ensure UI finishes if provider doesn't send [DONE]
        queue_done.lock().unwrap().push_back(StreamEvent::Done);
        register_ext_trigger(trigger_bg);
    });
}

fn app_view() -> impl IntoView {
    let messages = create_rw_signal(Vec::<Message>::new());
    let input_text = create_rw_signal(String::new());
    let typing = create_rw_signal(String::new());
    let chunk_count = create_rw_signal(0usize);
    let started = create_rw_signal(false);

    // Auto-run UI streaming test if GEMINI_UI_TEST is set
    {
        let messages_auto = messages;
        let typing_auto = typing;
        let chunk_count_auto = chunk_count;
        create_effect(move |_| {
            if env::var("GEMINI_UI_TEST").ok().is_some() && !started.get() {
                started.set(true);
                let default_prompt = "Write a detailed 1500+ word essay to force streaming across multiple SSE events. Include sections, bullet points, and inline code samples.".to_string();
                let prompt = env::var("GEMINI_UI_PROMPT").unwrap_or(default_prompt);
                start_stream_session(messages_auto, typing_auto, prompt, chunk_count_auto);
            }
        });
    }
    
    v_stack((
        // Header
        container(
            label(|| "Gemini 2.5 Flash - Click Send Button".to_string())
                .style(|s| {
                    s.font_size(16.0)
                        .font_weight(floem::text::Weight::BOLD)
                        .color(Color::WHITE)
                })
        )
        .style(|s| {
            s.width_full()
                .padding(16.0)
                .background(Color::from_rgb8(0x1a, 0x1a, 0x1a))
                .border_bottom(1.0)
                .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
        }),
        
        // Messages area
        scroll(
            dyn_stack(
                move || messages.get(),
                |msg| (msg.role.clone(), msg.content.clone()),
                move |msg| {
                    let role_label = if msg.role == "user" { "You" } else { "Gemini" };
                    let bg_color = if msg.role == "user" {
                        Color::from_rgb8(0x2a, 0x2a, 0x2a)
                    } else {
                        Color::from_rgb8(0x1e, 0x1e, 0x1e)
                    };
                    let content = msg.content.clone();
                    
                    container(
                        v_stack((
                            label(move || role_label.to_string())
                                .style(|s| {
                                    s.font_size(12.0)
                                        .font_weight(floem::text::Weight::BOLD)
                                        .color(Color::from_rgb8(0x88, 0x88, 0xff))
                                        .margin_bottom(4.0)
                                }),
                            label(move || content.clone())
                                .style(|s| {
                                    s.font_size(14.0)
                                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                                        .line_height(1.5)
                                }),
                        ))
                        .style(|s| s.flex_col().gap(4.0))
                    )
                    .style(move |s| {
                        s.width_full()
                            .padding(12.0)
                            .margin_bottom(8.0)
                            .background(bg_color)
                            .border_radius(8.0)
                    })
                },
            )
            .style(|s| s.flex_col().width_full())
        )
        .style(|s| {
            s.flex_grow(1.0)
                .width_full()
                .padding(16.0)
        }),

        // Typing area (shows live streaming content)
        container(
            v_stack((
                label(|| "Gemini".to_string()).style(|s| {
                    s.font_size(12.0)
                        .font_weight(floem::text::Weight::BOLD)
                        .color(Color::from_rgb8(0x88, 0x88, 0xff))
                        .margin_bottom(4.0)
                }),
                label(move || typing.get()).style(|s| {
                    s.font_size(14.0)
                        .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                        .line_height(1.5)
                }),
            ))
            .style(|s| s.flex_col().gap(4.0))
        )
        .style(|s| {
            s.width_full()
                .padding(12.0)
                .margin_bottom(8.0)
                .background(Color::from_rgb8(0x1e, 0x1e, 0x1e))
                .border_radius(8.0)
        }),
        
        // Input area with WORKING button
        container(
            h_stack((
                text_input(input_text)
                    .placeholder("Type your message...".to_string())
                    .style(|s| {
                        s.flex_grow(1.0)
                            .padding(12.0)
                            .background(Color::from_rgb8(0x2a, 0x2a, 0x2a))
                            .border(1.0)
                            .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
                            .border_radius(8.0)
                            .color(Color::from_rgb8(0xcc, 0xcc, 0xcc))
                            .font_size(14.0)
                            .margin_right(8.0)
                    }),
                container(
                    label(|| "Send".to_string())
                        .style(|s| {
                            s.font_size(14.0)
                                .color(Color::WHITE)
                        })
                )
                .on_click_stop(move |_| {
                    let text = input_text.get();
                    if !text.trim().is_empty() {
                        start_stream_session(messages, typing, text.clone(), chunk_count);
                        input_text.set(String::new());
                    }
                })
                .style(|s| {
                    s.padding(12.0)
                        .padding_horiz(24.0)
                        .background(Color::from_rgb8(0x00, 0x7a, 0xcc))
                        .border_radius(8.0)
                        .cursor(floem::style::CursorStyle::Pointer)
                }),
            ))
        )
        .style(|s| {
            s.width_full()
                .padding(16.0)
                .background(Color::from_rgb8(0x1a, 0x1a, 0x1a))
                .border_top(1.0)
                .border_color(Color::from_rgb8(0x45, 0x45, 0x45))
        }),
    ))
    .style(|s| {
        s.width_full()
            .height_full()
            .flex_col()
            .background(Color::from_rgb8(0x1e, 0x1e, 0x1e))
    })
}

fn stream_gemini_sse(
    prompt: &str,
    mut on_event: impl FnMut(StreamEvent) + Send + 'static,
) -> Result<(), String> {
    use std::time::Duration;

    let api_key = gemini_api_key()?;
    let base_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
        gemini_model()
    );

    // Build robust client: enable keep-alives and avoid global timeout to allow long-lived SSE
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .user_agent("Lapce-Gemini-Streaming/0.1")
        .build()
        .map_err(|e| format!("HTTP client build failed: {}", e))?;
    let payload = json!({
        "contents": [{
            "parts": [{ "text": prompt }]
        }],
        "generationConfig": {
            "temperature": 0.9,
            "topK": 40,
            "topP": 0.95,
            "maxOutputTokens": 4096,
        }
    });
    // Retry send on transient failures (timeout/connect/429/5xx)
    let mut attempt = 0u32;
    let mut response = loop {
        attempt += 1;
        let send_res = client
            .post(&base_url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("x-goog-api-key", &api_key)
            .json(&payload)
            .send();

        match send_res {
            Ok(resp) => {
                if resp.status().is_success() {
                    break resp;
                } else {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_else(|_| "<no body>".into());
                    if status.is_server_error() || status.as_u16() == 429 {
                        if attempt < 3 {
                            eprintln!("[SSE] server error {}, retrying attempt {}...", status, attempt);
                            std::thread::sleep(Duration::from_millis(750 * attempt as u64));
                            continue;
                        }
                    }
                    return Err(format!("API error {}: {}", status, body));
                }
            }
            Err(e) => {
                if (e.is_timeout() || e.is_connect()) && attempt < 3 {
                    eprintln!("[SSE] transient error '{}', retrying attempt {}...", e, attempt);
                    std::thread::sleep(Duration::from_millis(750 * attempt as u64));
                    continue;
                }
                return Err(format!("Request failed: {}", e));
            }
        }
    };

    // Read small chunks and split into lines for fastest UI updates
    let mut buf = [0u8; 1024];
    let mut line_buf: Vec<u8> = Vec::with_capacity(2048);

    loop {
        match response.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                for &b in &buf[..n] {
                    if b == b'\n' {
                        // Process a complete line
                        if !line_buf.is_empty() {
                            if let Ok(line) = String::from_utf8(line_buf.clone()) {
                                // Debug: print raw SSE line
                                println!("[SSE] {}", line);
                                if let Some(json_str) = line.strip_prefix("data: ") {
                                    let trimmed = json_str.trim();
                                    if trimmed == "[DONE]" {
                                        on_event(StreamEvent::Done);
                                        return Ok(());
                                    } else if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                                        // Emit chunk text when present
                                        let text_opt = json["candidates"][0]["content"]["parts"][0]["text"].as_str()
                                            .or_else(|| json["candidates"][0]["delta"]["parts"][0]["text"].as_str());
                                        if let Some(text) = text_opt {
                                            on_event(StreamEvent::Chunk(text.to_string()));
                                        }

                                        // Detect model completion and terminate stream proactively
                                        let finish_opt = json["candidates"][0]["finishReason"].as_str()
                                            .or_else(|| json["finishReason"].as_str());
                                        if let Some(_finish) = finish_opt {
                                            on_event(StreamEvent::Done);
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            line_buf.clear();
                        }
                    } else if b != b'\r' {
                        line_buf.push(b);
                    }
                }
            }
            Err(e) => return Err(format!("Read error: {}", e)),
        }
    }
    Ok(())
}

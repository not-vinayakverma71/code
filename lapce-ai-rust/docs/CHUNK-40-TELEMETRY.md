# CHUNK-40: PACKAGES/TELEMETRY - ANALYTICS & TRACKING

## 📁 MODULE STRUCTURE

```
packages/telemetry/src/
├── index.ts                      - Exports (telemetry disabled by default)
├── TelemetryService.ts           - Main service (production)
├── TelemetryService.stub.ts      - Stub implementation (no-op)
├── BaseTelemetryClient.ts        - Abstract client interface
├── PostHogTelemetryClient.ts     - PostHog implementation
└── __tests__/
    └── PostHogTelemetryClient.test.ts
```

**Total**: 6 files, ~500 lines

---

## 🎯 PURPOSE

Privacy-first analytics system with **opt-in telemetry** for:
- Feature usage tracking
- Error reporting
- Performance metrics
- User journey analytics

**Key Feature**: Telemetry is **disabled by default**, users must explicitly opt-in.

---

## 🔧 IMPLEMENTATION

### 1. TelemetryService.stub.ts - No-Op Implementation

```typescript
export class TelemetryService {
    static initialize(): void {
        // No-op when telemetry disabled
    }
    
    static identify(userId: string, traits?: Record<string, any>): void {
        // No-op
    }
    
    static track(event: string, properties?: Record<string, any>): void {
        // No-op
    }
    
    static shutdown(): Promise<void> {
        return Promise.resolve()
    }
}
```

### 2. BaseTelemetryClient.ts - Abstract Interface

```typescript
export abstract class BaseTelemetryClient {
    abstract identify(
        userId: string, 
        traits?: Record<string, any>
    ): void
    
    abstract track(
        event: string, 
        properties?: Record<string, any>
    ): void
    
    abstract shutdown(): Promise<void>
}
```

### 3. PostHogTelemetryClient.ts - PostHog Integration

```typescript
import { PostHog } from 'posthog-node'

export class PostHogTelemetryClient extends BaseTelemetryClient {
    private client: PostHog
    private userId?: string
    
    constructor(apiKey: string, options?: {
        host?: string
        flushInterval?: number
    }) {
        super()
        this.client = new PostHog(apiKey, {
            host: options?.host || 'https://app.posthog.com',
            flushInterval: options?.flushInterval || 30000,
        })
    }
    
    identify(userId: string, traits?: Record<string, any>): void {
        this.userId = userId
        this.client.identify({
            distinctId: userId,
            properties: traits,
        })
    }
    
    track(event: string, properties?: Record<string, any>): void {
        if (!this.userId) {
            console.warn('Cannot track event: user not identified')
            return
        }
        
        this.client.capture({
            distinctId: this.userId,
            event,
            properties,
        })
    }
    
    async shutdown(): Promise<void> {
        await this.client.shutdown()
    }
}
```

### 4. TelemetryService.ts - Production Service

```typescript
import { PostHogTelemetryClient } from './PostHogTelemetryClient'
import { BaseTelemetryClient } from './BaseTelemetryClient'

export class TelemetryService {
    private static client?: BaseTelemetryClient
    
    static initialize(config: {
        enabled: boolean
        apiKey?: string
        userId?: string
        userTraits?: Record<string, any>
    }): void {
        if (!config.enabled || !config.apiKey) {
            return // Telemetry disabled
        }
        
        this.client = new PostHogTelemetryClient(config.apiKey)
        
        if (config.userId) {
            this.client.identify(config.userId, config.userTraits)
        }
    }
    
    static identify(userId: string, traits?: Record<string, any>): void {
        this.client?.identify(userId, traits)
    }
    
    static track(event: string, properties?: Record<string, any>): void {
        this.client?.track(event, properties)
    }
    
    static async shutdown(): Promise<void> {
        await this.client?.shutdown()
    }
}

// Event helpers
export const TelemetryEvents = {
    TASK_STARTED: 'task_started',
    TASK_COMPLETED: 'task_completed',
    TASK_FAILED: 'task_failed',
    TOOL_USED: 'tool_used',
    MODE_SWITCHED: 'mode_switched',
    API_CALL: 'api_call',
    ERROR_OCCURRED: 'error_occurred',
}
```

---

## 📊 TYPICAL USAGE

### In Extension

```typescript
import { TelemetryService, TelemetryEvents } from '@clean-code/telemetry'

// On extension activation
const config = vscode.workspace.getConfiguration('rooCode')
TelemetryService.initialize({
    enabled: config.get('telemetryEnabled', false),
    apiKey: process.env.POSTHOG_API_KEY,
    userId: context.globalState.get('userId'),
    userTraits: {
        version: context.extension.packageJSON.version,
        platform: process.platform,
    }
})

// Track events
TelemetryService.track(TelemetryEvents.TASK_STARTED, {
    mode: 'code',
    model: 'claude-sonnet-4',
})

TelemetryService.track(TelemetryEvents.TOOL_USED, {
    tool: 'write_to_file',
    success: true,
})

// On deactivation
await TelemetryService.shutdown()
```

---

## 🦀 RUST TRANSLATION

### Option 1: PostHog Rust Client

```rust
use reqwest::Client;
use serde_json::json;

pub struct PostHogClient {
    client: Client,
    api_key: String,
    host: String,
    user_id: Option<String>,
}

impl PostHogClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            host: "https://app.posthog.com".to_string(),
            user_id: None,
        }
    }
    
    pub fn identify(&mut self, user_id: String, traits: serde_json::Value) {
        self.user_id = Some(user_id.clone());
        
        let payload = json!({
            "api_key": self.api_key,
            "event": "$identify",
            "distinct_id": user_id,
            "properties": traits,
        });
        
        let client = self.client.clone();
        let host = self.host.clone();
        
        tokio::spawn(async move {
            client.post(format!("{}/capture", host))
                .json(&payload)
                .send()
                .await
                .ok();
        });
    }
    
    pub fn track(&self, event: &str, properties: serde_json::Value) {
        let Some(user_id) = &self.user_id else {
            return;
        };
        
        let payload = json!({
            "api_key": self.api_key,
            "event": event,
            "distinct_id": user_id,
            "properties": properties,
        });
        
        let client = self.client.clone();
        let host = self.host.clone();
        
        tokio::spawn(async move {
            client.post(format!("{}/capture", host))
                .json(&payload)
                .send()
                .await
                .ok();
        });
    }
}
```

### Option 2: Use Existing Crate

```toml
[dependencies]
# Option A: segment-rs (Segment/PostHog compatible)
analytics = "0.2"

# Option B: Direct HTTP
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
```

### Telemetry Service

```rust
use std::sync::{Arc, Mutex};
use once_cell::sync::OnceCell;

static TELEMETRY: OnceCell<Arc<Mutex<TelemetryService>>> = OnceCell::new();

pub struct TelemetryService {
    client: Option<PostHogClient>,
    enabled: bool,
}

impl TelemetryService {
    pub fn initialize(config: TelemetryConfig) -> Result<()> {
        let service = if config.enabled && config.api_key.is_some() {
            let mut client = PostHogClient::new(config.api_key.unwrap());
            
            if let Some(user_id) = config.user_id {
                client.identify(user_id, config.user_traits.unwrap_or_default());
            }
            
            Self {
                client: Some(client),
                enabled: true,
            }
        } else {
            Self {
                client: None,
                enabled: false,
            }
        };
        
        TELEMETRY.set(Arc::new(Mutex::new(service)))
            .map_err(|_| anyhow!("Telemetry already initialized"))?;
        
        Ok(())
    }
    
    pub fn track(event: &str, properties: serde_json::Value) {
        if let Some(service) = TELEMETRY.get() {
            let service = service.lock().unwrap();
            if service.enabled {
                if let Some(client) = &service.client {
                    client.track(event, properties);
                }
            }
        }
    }
    
    pub fn identify(user_id: String, traits: serde_json::Value) {
        if let Some(service) = TELEMETRY.get() {
            let mut service = service.lock().unwrap();
            if service.enabled {
                if let Some(client) = &mut service.client {
                    client.identify(user_id, traits);
                }
            }
        }
    }
}

// Event constants
pub mod events {
    pub const TASK_STARTED: &str = "task_started";
    pub const TASK_COMPLETED: &str = "task_completed";
    pub const TOOL_USED: &str = "tool_used";
    pub const MODE_SWITCHED: &str = "mode_switched";
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Opt-In by Default

**Why**: Privacy-first approach
- Users must explicitly enable telemetry
- No data collection without consent
- Stub implementation when disabled (zero overhead)

### 2. Async Fire-and-Forget

**Why**: Don't block main thread
- Events sent asynchronously
- Failures don't crash the app
- Batching for efficiency

### 3. Abstract Client Interface

**Why**: Provider flexibility
- Easy to swap PostHog for alternatives
- Can add multiple providers
- Testing with mock clients

---

## 📊 TRANSLATION COMPLEXITY

| Component | Lines | Complexity | Effort | Notes |
|-----------|-------|------------|--------|-------|
| Stub | ~30 | Low | 1h | Simple no-op |
| Base interface | ~50 | Low | 1h | Trait definition |
| PostHog client | ~200 | Medium | 3-4h | HTTP API calls |
| Service | ~150 | Low | 2-3h | Singleton pattern |
| **TOTAL** | **~500** | **Low** | **8-10 hours** | Straightforward |

---

## 🎓 KEY TAKEAWAYS

✅ **Privacy-First**: Disabled by default, opt-in only

✅ **Low Overhead**: Stub implementation when disabled

✅ **Provider Agnostic**: Abstract interface for flexibility

✅ **Async Events**: Non-blocking telemetry

✅ **Simple Translation**: Straightforward Rust port

✅ **Optional Feature**: Can be compiled out entirely

---

## 🔧 RUST CARGO FEATURES

```toml
[features]
default = []
telemetry = ["reqwest", "serde_json"]

[dependencies]
reqwest = { version = "0.11", features = ["json"], optional = true }
serde_json = { version = "1.0", optional = true }
```

**Usage**:
```bash
# Without telemetry (smaller binary)
cargo build

# With telemetry
cargo build --features telemetry
```

---

**Status**: ✅ Complete analysis of packages/telemetry (6 files, ~500 lines)
**Priority**: LOW - Optional analytics feature
**Effort**: 8-10 hours for full translation
**Next**: CHUNK-41 (apps/) and CHUNK-42 (.kilocode, benchmark)

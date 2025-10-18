# CHUNK-33-35: PACKAGES/BUILD, CLOUD, CONFIG - INFRASTRUCTURE PACKAGES

## 📁 MODULE STRUCTURE

```
Codex/packages/
├── build/              - ESBuild utilities
├── cloud/              - Cloud service integration
├── config-eslint/      - Shared ESLint config
└── config-typescript/  - Shared TypeScript config
```

---

## 1️⃣ PACKAGES/BUILD - ESBUILD UTILITIES

### Purpose
Shared build utilities for bundling VSCode extension with esbuild.

### Package Info
```json
{
  "name": "@clean-code/build",
  "description": "ESBuild utilities for Roo Code",
  "private": true,
  "dependencies": {
    "zod": "^3.25.61"
  }
}
```

### Files (5 total)
```
build/src/
├── index.ts           - Main exports
├── esbuild-config.ts  - ESBuild configuration
├── plugin-inline.ts   - Inline file plugin
├── plugin-watch.ts    - Watch mode plugin
└── utils.ts           - Build utilities
```

### Typical ESBuild Config Pattern

```typescript
import { build, BuildOptions } from 'esbuild'

export const createBuildConfig = (options: {
  entry: string
  outfile: string
  minify?: boolean
  watch?: boolean
}): BuildOptions => ({
  entryPoints: [options.entry],
  bundle: true,
  outfile: options.outfile,
  external: ['vscode'],
  format: 'cjs',
  platform: 'node',
  target: 'node18',
  sourcemap: !options.minify,
  minify: options.minify || false,
  // ... plugins
})
```

### Rust Translation
**NOT NEEDED** - Build tooling is TypeScript-specific. Rust uses `cargo` instead.

---

## 2️⃣ PACKAGES/CLOUD - CLOUD SERVICE INTEGRATION

### Purpose
Cloud backend integration for task sharing, analytics, and user management.

### Package Info
```json
{
  "name": "@clean-code/cloud",
  "dependencies": {
    "posthog-node": "^4.3.1"  // Analytics
  }
}
```

### Files (3 files, ~3,100 lines)
```
cloud/src/
├── TelemetryClient.ts  (2288 lines) - PostHog analytics
├── index.ts            (2429 lines) - Cloud API client
└── utils.ts            (383 lines)  - Helper utilities
```

### Key Features

**1. Cloud API Client**
```typescript
export interface CloudUserInfo {
    id: string
    email: string
    username: string
    avatarUrl?: string
}

export interface OrganizationAllowList {
    id: string
    name: string
    domains: string[]
    allowedUsers: string[]
}

export type ShareVisibility = 'private' | 'organization' | 'public'

export class CloudClient {
    private apiUrl: string
    private apiKey?: string
    
    constructor(apiUrl: string, apiKey?: string) {
        this.apiUrl = apiUrl
        this.apiKey = apiKey
    }
    
    async shareTask(taskData: {
        conversationHistory: any[]
        visibility: ShareVisibility
        organizationId?: string
    }): Promise<{ shareId: string; shareUrl: string }> {
        const response = await fetch(`${this.apiUrl}/tasks/share`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${this.apiKey}`,
            },
            body: JSON.stringify(taskData),
        })
        
        return response.json()
    }
    
    async getUserInfo(): Promise<CloudUserInfo> {
        const response = await fetch(`${this.apiUrl}/users/me`, {
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
            },
        })
        
        return response.json()
    }
    
    async getOrganizations(): Promise<OrganizationAllowList[]> {
        const response = await fetch(`${this.apiUrl}/organizations`, {
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
            },
        })
        
        return response.json()
    }
}
```

**2. PostHog Analytics Client**
```typescript
import { PostHog } from 'posthog-node'

export class TelemetryClient {
    private client: PostHog
    private userId?: string
    
    constructor(apiKey: string) {
        this.client = new PostHog(apiKey, {
            host: 'https://app.posthog.com',
        })
    }
    
    identify(userId: string, properties: Record<string, any>) {
        this.userId = userId
        this.client.identify({
            distinctId: userId,
            properties,
        })
    }
    
    capture(event: string, properties?: Record<string, any>) {
        if (!this.userId) return
        
        this.client.capture({
            distinctId: this.userId,
            event,
            properties,
        })
    }
    
    async shutdown() {
        await this.client.shutdown()
    }
}

// Usage
const telemetry = new TelemetryClient(process.env.POSTHOG_API_KEY)
telemetry.identify('user-123', {
    email: 'user@example.com',
    plan: 'pro',
})
telemetry.capture('task_created', {
    mode: 'code',
    model: 'claude-sonnet-4',
})
```

### Rust Translation

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudUserInfo {
    pub id: String,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareVisibility {
    Private,
    Organization,
    Public,
}

pub struct CloudClient {
    client: Client,
    api_url: String,
    api_key: Option<String>,
}

impl CloudClient {
    pub fn new(api_url: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_url,
            api_key,
        }
    }
    
    pub async fn share_task(
        &self,
        conversation_history: Vec<serde_json::Value>,
        visibility: ShareVisibility,
        organization_id: Option<String>,
    ) -> Result<ShareResponse> {
        let mut req = self.client
            .post(format!("{}/tasks/share", self.api_url))
            .json(&serde_json::json!({
                "conversationHistory": conversation_history,
                "visibility": visibility,
                "organizationId": organization_id,
            }));
        
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        
        let res = req.send().await?;
        Ok(res.json().await?)
    }
    
    pub async fn get_user_info(&self) -> Result<CloudUserInfo> {
        let mut req = self.client.get(format!("{}/users/me", self.api_url));
        
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        
        let res = req.send().await?;
        Ok(res.json().await?)
    }
}

#[derive(Debug, Deserialize)]
pub struct ShareResponse {
    pub share_id: String,
    pub share_url: String,
}
```

### PostHog Alternative for Rust
```rust
// Use analytics-rust crate
use analytics::{Client, Identify, Track};

pub struct TelemetryClient {
    client: Client,
    user_id: Option<String>,
}

impl TelemetryClient {
    pub fn new(write_key: &str) -> Self {
        let client = Client::new(write_key);
        Self {
            client,
            user_id: None,
        }
    }
    
    pub fn identify(&mut self, user_id: String, properties: serde_json::Value) {
        self.user_id = Some(user_id.clone());
        self.client.identify(Identify {
            user_id,
            traits: properties,
        });
    }
    
    pub fn capture(&self, event: &str, properties: Option<serde_json::Value>) {
        if let Some(user_id) = &self.user_id {
            self.client.track(Track {
                user_id: user_id.clone(),
                event: event.to_string(),
                properties: properties.unwrap_or_default(),
            });
        }
    }
}
```

---

## 3️⃣ PACKAGES/CONFIG-ESLINT - SHARED ESLINT CONFIG

### Purpose
Shared ESLint configuration for consistent code style across all packages.

### Package Info
```json
{
  "name": "@clean-code/config-eslint",
  "private": true
}
```

### Typical Config
```javascript
// eslint.config.mjs
export default {
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:react/recommended',
  ],
  parser: '@typescript-eslint/parser',
  plugins: ['@typescript-eslint', 'react'],
  rules: {
    'no-console': 'warn',
    '@typescript-eslint/no-explicit-any': 'warn',
    '@typescript-eslint/no-unused-vars': 'error',
    'react/prop-types': 'off',
  },
}
```

### Rust Translation
**Use `clippy` instead** - Rust's built-in linter

```toml
# Cargo.toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

---

## 4️⃣ PACKAGES/CONFIG-TYPESCRIPT - SHARED TSCONFIG

### Purpose
Shared TypeScript configuration for type checking.

### Package Info
```json
{
  "name": "@clean-code/config-typescript",
  "private": true
}
```

### Typical Config
```json
// tsconfig.json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Node16",
    "lib": ["ES2022"],
    "moduleResolution": "Node16",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  },
  "exclude": ["node_modules", "dist"]
}
```

### Rust Translation
**Not applicable** - Rust doesn't use tsconfig. Use `Cargo.toml` edition:

```toml
[package]
edition = "2021"

[dependencies]
# Rust 2021 edition features:
# - Simplified panic macros
# - Disjoint capture in closures
# - Pattern matching improvements
```

---

## 📊 SUMMARY TABLE

| Package | Files | Lines | Purpose | Rust Equivalent |
|---------|-------|-------|---------|-----------------|
| build | 5 | ~500 | ESBuild config | N/A (use cargo) |
| cloud | 3 | ~3,100 | Cloud API + Analytics | reqwest + analytics-rust |
| config-eslint | 1 | ~50 | Linting rules | clippy config |
| config-typescript | 1 | ~30 | TypeScript config | Cargo.toml edition |

---

## 🎯 KEY TAKEAWAYS

✅ **Build Package**: TypeScript-specific, not needed in Rust

✅ **Cloud Package**: Critical for task sharing and analytics
- REST API client (reqwest)
- PostHog analytics (analytics-rust crate)

✅ **Config Packages**: Development tooling only
- ESLint → Clippy (Rust)
- TSConfig → Cargo.toml

✅ **Translation Priority**: MEDIUM
- Cloud client is important for full feature parity
- Build/config tools are dev-only, not runtime

---

## 🦀 RUST IMPLEMENTATION CHECKLIST

### High Priority
- [ ] Cloud API client (reqwest)
- [ ] Analytics telemetry (analytics-rust or custom)
- [ ] User authentication flow
- [ ] Task sharing endpoints

### Low Priority (Dev Tools)
- [ ] ~~ESBuild config~~ (use cargo build)
- [ ] ~~TSConfig~~ (use Cargo.toml)
- [ ] Clippy configuration for linting

---

**Status**: ✅ Complete analysis of packages/build, cloud, config
**Next**: CHUNK-36 (packages/evals - deep analysis)

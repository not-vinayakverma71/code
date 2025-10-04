# DEEP ANALYSIS 08: MCP INTEGRATION - MODEL CONTEXT PROTOCOL

## 📁 Analyzed Files

```
Codex/
├── packages/types/src/ (mcp.ts, marketplace.ts)
└── webview-ui/src/components/mcp/ (16 components)
│       ├── McpInstallationMethod
│       ├── McpParameter
│       └── InstallOptions
│
├── webview-ui/src/components/
│   ├── mcp/
│   │   ├── McpView.tsx               (579 lines, main MCP UI)
│   │   │   ├── Server List
│   │   │   ├── Enable/Disable Toggle
│   │   │   ├── Restart/Delete Actions
│   │   │   └── Timeout Configuration
│   │   │
│   │   ├── McpToolRow.tsx            (144 lines, tool display)
│   │   │   ├── Always Allow Toggle
│   │   │   ├── Enable for Prompt
│   │   │   └── Parameter Schema
│   │   │
│   │   ├── McpResourceRow.tsx
│   │   └── McpErrorRow.tsx
│   │
│   └── kilocodeMcp/marketplace/
│       ├── McpMarketplaceView.tsx    (289 lines, browse/install)
│       │   ├── Search/Filter UI
│       │   ├── Category Filtering
│       │   ├── Sort Options
│       │   └── Installation Flow
│       │
│       ├── McpMarketplaceCard.tsx
│       └── McpSubmitCard.tsx
│
└── MCP Configuration Files
    ├── ~/.roo/mcp.json               (Global MCP servers)
    └── .roo-local/mcp.json           (Project MCP servers)

Total: 16 MCP components → Rust server management + marketplace API
```

---

## Overview
**MCP (Model Context Protocol)** extends AI capabilities through external tools and resources via **MCP servers**.

---

## 1. Core Types

```typescript
interface McpServer {
    name: string
    config: string  // JSON: command, args, env, timeout
    status: "connected" | "connecting" | "disconnected"
    disabled: boolean
    source?: "global" | "project"
    tools?: McpTool[]
    resources?: McpResource[]
    resourceTemplates?: McpResourceTemplate[]
    instructions?: string
    errorHistory?: McpError[]
}

interface McpTool {
    name: string
    description?: string
    inputSchema: { type: "object"; properties: Record<string, any>; required?: string[] }
    alwaysAllow?: boolean
    enabledForPrompt?: boolean
}
```

**Rust:**

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpServer {
    pub name: String,
    pub config: String,
    pub status: McpStatus,
    pub disabled: bool,
    pub source: Option<McpSource>,
    pub tools: Option<Vec<McpTool>>,
    pub resources: Option<Vec<McpResource>>,
    pub instructions: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub always_allow: Option<bool>,
    pub enabled_for_prompt: Option<bool>,
}
```

## 2. Marketplace

```typescript
interface McpMarketplaceItem {
    id: string
    name: string
    description: string
    category: string
    url: string
    content: string | McpInstallationMethod[]
    githubStars: number
}
```

**Backend:**

```rust
pub async fn fetch_mcp_marketplace() -> Result<McpMarketplaceCatalog> {
    let response = reqwest::get("https://marketplace.kilocode.ai/api/mcp-catalog.json")
        .await?
        .json::<McpMarketplaceCatalog>()
        .await?;
    Ok(response)
}
```

**STATUS:** MCP analysis complete

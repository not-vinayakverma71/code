# CHUNK-08: CONFIG MANAGEMENT (MULTI-PROFILE SYSTEM)

## 📁 Complete Directory Analysis

```
Codex/src/core/config/
├── ContextProxy.ts                (331 lines) - State caching proxy
├── ProviderSettingsManager.ts     (752 lines) - Multi-profile API configs
├── CustomModesManager.ts        (1,016 lines) - AI modes with YAML
├── importExport.ts                (219 lines) - Settings import/export
├── kilocode/
│   └── migrateMorphApiKey.ts      - Migration helper
└── __tests__/                     - Test suites

TOTAL: 2,318+ lines core config logic
```

---

## 🎯 PURPOSE

**Multi-Profile Configuration System**:

1. **ContextProxy**: Cache VS Code globalState/secrets for fast access
2. **ProviderSettingsManager**: Manage multiple API provider profiles
3. **CustomModesManager**: Load/save AI modes from YAML files
4. **Import/Export**: Backup/restore all settings

**Critical for**:
- Supporting multiple API keys (dev/prod)
- Per-mode API configuration (use GPT-4 for code, GPT-3.5 for chat)
- Project-specific AI modes (.kilocodemodes file)
- Settings portability

---

## 📊 ARCHITECTURE OVERVIEW

```
VS Code Extension Storage:
├── globalState (JSON key-value)
│   ├── taskHistory
│   ├── currentApiConfigName
│   └── customModes
├── secrets (encrypted)
│   ├── roo_cline_config_api_config (all profiles)
│   ├── openAiApiKey
│   └── anthropicApiKey
└── workspaceState (per-workspace)

ContextProxy (in-memory cache):
├── stateCache: GlobalState
└── secretCache: SecretState

ProviderSettingsManager:
{
  currentApiConfigName: "production",
  apiConfigs: {
    "production": { id: "abc123", apiProvider: "anthropic", ... },
    "development": { id: "def456", apiProvider: "openai", ... }
  },
  modeApiConfigs: {
    "code": "abc123",      // Use production for code mode
    "architect": "def456"  // Use development for architect mode
  }
}
```

---

## 🔧 FILE 1: ContextProxy.ts (331 lines)

### Purpose: Fast Access to Extension State

**Problem**: Reading from `context.globalState.get()` and `context.secrets.get()` is async and slow.

**Solution**: Cache all values in memory on initialization, read synchronously from cache.

### Class Structure

```typescript
export class ContextProxy {
    private readonly originalContext: vscode.ExtensionContext
    private stateCache: GlobalState = {}
    private secretCache: SecretState = {}
    private _isInitialized = false
    
    private static _instance: ContextProxy | null = null
    
    static async getInstance(context: vscode.ExtensionContext) {
        if (!this._instance) {
            this._instance = new ContextProxy(context)
            await this._instance.initialize()
        }
        return this._instance
    }
}
```

**Singleton pattern**: Only one instance per extension activation.

### Method 1: initialize() - Lines 58-79

```typescript
public async initialize() {
    // Load all global state keys
    for (const key of GLOBAL_STATE_KEYS) {
        this.stateCache[key] = this.originalContext.globalState.get(key)
    }
    
    // Load all secrets in parallel
    const promises = SECRET_STATE_KEYS.map(async (key) => {
        this.secretCache[key] = await this.originalContext.secrets.get(key)
    })
    await Promise.all(promises)
    
    this._isInitialized = true
}
```

**Keys loaded**:
- `GLOBAL_STATE_KEYS`: taskHistory, currentApiConfigName, customModes, etc.
- `SECRET_STATE_KEYS`: API keys, tokens

**Performance**: Parallel secret loading reduces initialization time.

### Method 2: getGlobalState() - Lines 110-120

```typescript
getGlobalState<K extends GlobalStateKey>(key: K, defaultValue?: GlobalState[K]): GlobalState[K] {
    if (isPassThroughStateKey(key)) {
        // For taskHistory, always read from storage (constantly changing)
        const value = this.originalContext.globalState.get<GlobalState[K]>(key)
        return value === undefined || value === null ? defaultValue : value
    }
    
    // Read from cache for other keys
    const value = this.stateCache[key]
    return value !== undefined ? value : defaultValue
}
```

**Pass-through keys**: `taskHistory` changes too frequently to cache (each message).

**Others**: Cached for speed.

### Method 3: updateGlobalState() - Lines 122-129

```typescript
updateGlobalState<K extends GlobalStateKey>(key: K, value: GlobalState[K]) {
    if (isPassThroughStateKey(key)) {
        return this.originalContext.globalState.update(key, value)
    }
    
    // Update both cache and storage
    this.stateCache[key] = value
    return this.originalContext.globalState.update(key, value)
}
```

**Write-through cache**: Update cache immediately, then persist to storage.

### Method 4: getSecret() / storeSecret() - Lines 140-152

```typescript
getSecret(key: SecretStateKey) {
    return this.secretCache[key]  // Synchronous!
}

storeSecret(key: SecretStateKey, value?: string) {
    this.secretCache[key] = value  // Update cache
    
    return value === undefined
        ? this.originalContext.secrets.delete(key)
        : this.originalContext.secrets.store(key, value)
}
```

**Critical**: Secrets are encrypted by VS Code but cached in memory for speed.

### Method 5: export() - Lines 275-290

```typescript
public async export(): Promise<GlobalSettings | undefined> {
    const globalSettings = globalSettingsExportSchema.parse(this.getValues())
    
    // Only export global custom modes (not project modes)
    globalSettings.customModes = globalSettings.customModes?.filter(
        (mode) => mode.source === "global"
    )
    
    // Remove undefined values
    return Object.fromEntries(
        Object.entries(globalSettings).filter(([_, value]) => value !== undefined)
    )
}
```

**Export filtering**: Project modes are stored in `.kilocodemodes`, don't export them.

### Rust Translation

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct ContextProxy {
    state_cache: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    secret_cache: Arc<Mutex<HashMap<String, String>>>,
    is_initialized: bool,
}

impl ContextProxy {
    pub async fn initialize(&mut self, storage: &dyn Storage) -> Result<(), Error> {
        let mut state_cache = self.state_cache.lock().unwrap();
        let mut secret_cache = self.secret_cache.lock().unwrap();
        
        // Load global state
        for key in GLOBAL_STATE_KEYS {
            if let Some(value) = storage.get_global_state(key).await? {
                state_cache.insert(key.to_string(), value);
            }
        }
        
        // Load secrets in parallel
        let futures: Vec<_> = SECRET_STATE_KEYS.iter()
            .map(|key| storage.get_secret(key))
            .collect();
        
        let results = futures::future::join_all(futures).await;
        for (key, result) in SECRET_STATE_KEYS.iter().zip(results) {
            if let Ok(Some(value)) = result {
                secret_cache.insert(key.to_string(), value);
            }
        }
        
        self.is_initialized = true;
        Ok(())
    }
    
    pub fn get_global_state(&self, key: &str) -> Option<serde_json::Value> {
        self.state_cache.lock().unwrap().get(key).cloned()
    }
    
    pub fn get_secret(&self, key: &str) -> Option<String> {
        self.secret_cache.lock().unwrap().get(key).cloned()
    }
}
```

---

## 🔧 FILE 2: ProviderSettingsManager.ts (752 lines)

### Purpose: Multi-Profile API Configuration

Allows users to:
1. Create multiple API profiles (dev, prod, personal)
2. Switch between profiles
3. Assign different profiles to different AI modes
4. Sync profiles from cloud

### Data Structure

```typescript
type ProviderProfiles = {
    currentApiConfigName: string  // "production"
    apiConfigs: {
        [name: string]: ProviderSettingsWithId
    }
    modeApiConfigs: {
        [modeSlug: string]: configId
    }
    cloudProfileIds?: string[]
    migrations?: {...}
}

type ProviderSettingsWithId = {
    id: string
    apiProvider: "anthropic" | "openai" | "bedrock" | ...
    apiKey?: string
    anthropicApiKey?: string
    openAiApiKey?: string
    // ... provider-specific settings
}
```

### Storage Location

```typescript
private get secretsKey() {
    return "roo_cline_config_api_config"
}
```

**Stored in VS Code secrets** (encrypted JSON).

### Method 1: initialize() - Lines 88-178

```typescript
public async initialize() {
    return await this.lock(async () => {
        const providerProfiles = await this.load()
        
        if (!providerProfiles) {
            await this.store(this.defaultProviderProfiles)
            return
        }
        
        let isDirty = false
        
        // Migration 1: Add per-mode API config map
        if (!providerProfiles.modeApiConfigs) {
            providerProfiles.modeApiConfigs = Object.fromEntries(
                modes.map((m) => [m.slug, seedId])
            )
            isDirty = true
        }
        
        // Migration 2: Ensure all configs have IDs
        for (const apiConfig of Object.values(providerProfiles.apiConfigs)) {
            if (!apiConfig.id) {
                apiConfig.id = this.generateId()
                isDirty = true
            }
        }
        
        // Migration 3-7: Rate limits, diff settings, headers, etc.
        if (!providerProfiles.migrations.rateLimitSecondsMigrated) {
            await this.migrateRateLimitSeconds(providerProfiles)
            isDirty = true
        }
        // ... more migrations
        
        if (isDirty) {
            await this.store(providerProfiles)
        }
    })
}
```

**Lock pattern**: `this.lock()` ensures sequential read/write (prevents corruption).

**Migrations**: Each feature addition requires migration for existing users.

### Method 2: saveConfig() - Lines 312-329

```typescript
public async saveConfig(name: string, config: ProviderSettingsWithId): Promise<string> {
    return await this.lock(async () => {
        const providerProfiles = await this.load()
        
        const existingId = providerProfiles.apiConfigs[name]?.id
        const id = config.id || existingId || this.generateId()
        
        // Filter out settings from other providers
        const filteredConfig = discriminatedProviderSettingsWithIdSchema.parse(config)
        providerProfiles.apiConfigs[name] = { ...filteredConfig, id }
        
        await this.store(providerProfiles)
        return id
    })
}
```

**ID preservation**: Update keeps same ID (important for modeApiConfigs references).

**Filtering**: Anthropic config shouldn't have `openAiApiKey` field.

### Method 3: setModeConfig() - Lines 431-446

```typescript
public async setModeConfig(mode: Mode, configId: string) {
    return await this.lock(async () => {
        const providerProfiles = await this.load()
        
        if (!providerProfiles.modeApiConfigs) {
            providerProfiles.modeApiConfigs = {}
        }
        
        providerProfiles.modeApiConfigs[mode] = configId
        await this.store(providerProfiles)
    })
}
```

**Use case**: "Use GPT-4 for 'code' mode, GPT-3.5-turbo for 'chat' mode".

### Method 4: syncCloudProfiles() - Lines 569-750

**Most complex method** - syncs local profiles with cloud.

```typescript
public async syncCloudProfiles(
    cloudProfiles: Record<string, ProviderSettingsWithId>,
    currentActiveProfileName?: string
): Promise<SyncCloudProfilesResult> {
    return await this.lock(async () => {
        const providerProfiles = await this.load()
        
        // Step 1: Delete local profiles that are no longer in cloud
        for (const [name, profile] of Object.entries(providerProfiles.apiConfigs)) {
            if (profile.id && currentCloudIds.has(profile.id) && !newCloudIds.has(profile.id)) {
                delete providerProfiles.apiConfigs[name]
            }
        }
        
        // Step 2: Update/add cloud profiles
        for (const [cloudName, cloudProfile] of Object.entries(cloudProfiles)) {
            const existingEntry = Object.entries(providerProfiles.apiConfigs)
                .find(([_, profile]) => profile.id === cloudProfile.id)
            
            if (existingEntry) {
                // Update existing profile (merge, preserve secrets)
                const [existingName, existingProfile] = existingEntry
                const updatedProfile = { ...cloudProfile }
                
                // Preserve local secrets
                for (const [key, value] of Object.entries(existingProfile)) {
                    if (isSecretStateKey(key) && value !== undefined) {
                        updatedProfile[key] = value
                    }
                }
                
                // Handle name change
                if (existingName !== cloudName) {
                    delete providerProfiles.apiConfigs[existingName]
                    providerProfiles.apiConfigs[cloudName] = updatedProfile
                } else {
                    providerProfiles.apiConfigs[existingName] = updatedProfile
                }
            } else {
                // Add new cloud profile (without secrets)
                providerProfiles.apiConfigs[cloudName] = { ...cloudProfile }
            }
        }
        
        await this.store(providerProfiles)
        return { hasChanges, activeProfileChanged, activeProfileId }
    })
}
```

**Cloud sync challenges**:
1. Profile renamed in cloud → update local name
2. Name conflict → rename local non-cloud profile
3. Preserve local secrets (cloud doesn't store them)
4. Handle active profile deletion

### Rust Translation

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfiles {
    pub current_api_config_name: String,
    pub api_configs: HashMap<String, ProviderSettingsWithId>,
    pub mode_api_configs: HashMap<String, String>,
    pub cloud_profile_ids: Vec<String>,
}

pub struct ProviderSettingsManager {
    lock: Mutex<()>,
    storage: Arc<dyn SecretStorage>,
}

impl ProviderSettingsManager {
    pub async fn save_config(&self, name: &str, config: ProviderSettingsWithId) -> Result<String, Error> {
        let _guard = self.lock.lock().await;
        
        let mut profiles = self.load().await?;
        let id = config.id.clone()
            .or_else(|| profiles.api_configs.get(name).map(|c| c.id.clone()))
            .unwrap_or_else(|| self.generate_id());
        
        profiles.api_configs.insert(name.to_string(), ProviderSettingsWithId { id: id.clone(), ..config });
        self.store(&profiles).await?;
        
        Ok(id)
    }
}
```

---

## 🔧 FILE 3: CustomModesManager.ts (1,016 lines)

### Purpose: AI Mode Configuration from YAML

**AI Modes**: Predefined prompts/behaviors (e.g., "code", "architect", "debug").

**Two sources**:
1. **Global**: `~/.kilo-code/custom-modes.yaml` (user-wide)
2. **Project**: `{workspace}/.kilocodemodes` (project-specific, takes precedence)

### File Watching

```typescript
private async watchCustomModesFiles() {
    const settingsPath = await this.getCustomModesFilePath()
    const roomodesPath = await this.getWorkspaceRoomodes()
    
    // Watch global file
    const settingsWatcher = vscode.workspace.createFileSystemWatcher(settingsPath)
    settingsWatcher.onDidChange(handleSettingsChange)
    
    // Watch project file
    const roomodesWatcher = vscode.workspace.createFileSystemWatcher(roomodesPath)
    roomodesWatcher.onDidChange(handleRoomodesChange)
}
```

**Reactive**: File changes automatically reload modes.

### YAML Parsing with Cleanup - Lines 114-180

```typescript
private cleanInvisibleCharacters(content: string): string {
    return content.replace(PROBLEMATIC_CHARS_REGEX, (match) => {
        switch (match) {
            case "\u00A0": return " "      // Non-breaking space
            case "\u200B": return ""       // Zero-width space
            case "\u2018":
            case "\u2019": return "'"      // Smart quotes
            case "\u201C":
            case "\u201D": return '"'
            default: return "-"            // Various dashes
        }
    })
}

private parseYamlSafely(content: string, filePath: string): any {
    let cleanedContent = stripBom(content)
    cleanedContent = this.cleanInvisibleCharacters(cleanedContent)
    
    try {
        return yaml.parse(cleanedContent) ?? {}
    } catch (yamlError) {
        // For .roomodes files, try JSON as fallback
        if (filePath.endsWith(ROOMODES_FILENAME)) {
            try {
                return JSON.parse(content)
            } catch (jsonError) {
                vscode.window.showErrorMessage(`YAML parse error at line ${line}`)
                return {}
            }
        }
        return {}
    }
}
```

**Why cleanup**: Users copy-paste from websites with invisible Unicode characters.

**Fallback**: `.kilocodemodes` can be JSON or YAML.

### Mode Merging - Lines 225-246

```typescript
private async mergeCustomModes(projectModes: ModeConfig[], globalModes: ModeConfig[]) {
    const slugs = new Set<string>()
    const merged: ModeConfig[] = []
    
    // Project modes take precedence
    for (const mode of projectModes) {
        if (!slugs.has(mode.slug)) {
            slugs.add(mode.slug)
            merged.push({ ...mode, source: "project" })
        }
    }
    
    // Add non-duplicate global modes
    for (const mode of globalModes) {
        if (!slugs.has(mode.slug)) {
            slugs.add(mode.slug)
            merged.push({ ...mode, source: "global" })
        }
    }
    
    return merged
}
```

**Precedence**: Project mode with slug "code" overrides global "code" mode.

### Export with Rules Files - Lines 717-839

```typescript
public async exportModeWithRules(slug: string): Promise<ExportResult> {
    const mode = allModes.find((m) => m.slug === slug)
    if (!mode) return { success: false, error: "Mode not found" }
    
    // Determine rules directory
    const modeRulesDir = mode.source === "global"
        ? path.join(getGlobalRooDirectory(), `rules-${slug}`)
        : path.join(getProjectRooDirectory(), `rules-${slug}`)
    
    // Read all rules files
    let rulesFiles: RuleFile[] = []
    const entries = await fs.readdir(modeRulesDir, { withFileTypes: true })
    
    for (const entry of entries) {
        if (entry.isFile()) {
            const content = await fs.readFile(path.join(modeRulesDir, entry.name), "utf-8")
            if (content.trim()) {
                rulesFiles.push({
                    relativePath: entry.name,
                    content: content.trim()
                })
            }
        }
    }
    
    // Create export
    const exportMode: ExportedModeConfig = {
        ...mode,
        rulesFiles: rulesFiles.length > 0 ? rulesFiles : undefined
    }
    
    return {
        success: true,
        yaml: yaml.stringify({ customModes: [exportMode] })
    }
}
```

**Rules files**: Additional context files for specific modes (e.g., coding standards).

### Import with Path Validation - Lines 847-920

```typescript
private async importRulesFiles(importMode: ExportedModeConfig, rulesFiles: RuleFile[], source: "global" | "project") {
    const rulesFolderPath = source === "global"
        ? path.join(getGlobalRooDirectory(), `rules-${importMode.slug}`)
        : path.join(getProjectRooDirectory(), `rules-${importMode.slug}`)
    
    // Remove existing rules folder
    await fs.rm(rulesFolderPath, { recursive: true, force: true })
    
    // Import new files with validation
    for (const ruleFile of rulesFiles) {
        const normalizedPath = path.normalize(ruleFile.relativePath)
        
        // Security: Prevent path traversal
        if (normalizedPath.includes("..") || path.isAbsolute(normalizedPath)) {
            logger.error(`Invalid file path: ${ruleFile.relativePath}`)
            continue
        }
        
        // Handle old export format (strip rules-* prefix)
        let cleanedPath = normalizedPath
        const rulesMatch = normalizedPath.match(/^rules-[^\/\\]+[\/\\]/)
        if (rulesMatch) {
            cleanedPath = normalizedPath.substring(rulesMatch[0].length)
        }
        
        const targetPath = path.join(rulesFolderPath, cleanedPath)
        const normalizedTarget = path.normalize(targetPath)
        
        // Verify path stays within rules folder
        if (!normalizedTarget.startsWith(path.normalize(rulesFolderPath))) {
            logger.error(`Path traversal attempt: ${ruleFile.relativePath}`)
            continue
        }
        
        await fs.mkdir(path.dirname(targetPath), { recursive: true })
        await fs.writeFile(targetPath, ruleFile.content, "utf-8")
    }
}
```

**Security**: Path traversal prevention (`../../etc/passwd`).

---

## 🔧 FILE 4: importExport.ts (219 lines)

### Export Format

```json
{
  "providerProfiles": {
    "currentApiConfigName": "production",
    "apiConfigs": {
      "production": { "id": "abc", "apiProvider": "anthropic", ... },
      "development": { "id": "def", "apiProvider": "openai", ... }
    },
    "modeApiConfigs": {
      "code": "abc",
      "architect": "def"
    }
  },
  "globalSettings": {
    "customModes": [...],
    "alwaysAllowReadOnly": true,
    ...
  }
}
```

### Import Logic - Lines 39-104

```typescript
export async function importSettingsFromPath(filePath: string, { providerSettingsManager, contextProxy, customModesManager }) {
    const { providerProfiles: newProviderProfiles, globalSettings = {} } = schema.parse(
        JSON.parse(await fs.readFile(filePath, "utf-8"))
    )
    
    const previousProfiles = await providerSettingsManager.export()
    
    // Merge configs (new settings override)
    const mergedProfiles = {
        currentApiConfigName: newProviderProfiles.currentApiConfigName,
        apiConfigs: {
            ...previousProfiles.apiConfigs,  // Keep existing
            ...newProviderProfiles.apiConfigs  // Add/override with imported
        },
        modeApiConfigs: {
            ...previousProfiles.modeApiConfigs,
            ...newProviderProfiles.modeApiConfigs
        }
    }
    
    // Import custom modes
    await Promise.all(
        (globalSettings.customModes ?? []).map((mode) =>
            customModesManager.updateCustomMode(mode.slug, mode)
        )
    )
    
    await providerSettingsManager.import(mergedProfiles)
    await contextProxy.setValues(globalSettings)
    
    return { success: true }
}
```

**Merge strategy**: Additive (doesn't delete existing profiles).

### Export Logic - Lines 145-177

```typescript
export const exportSettings = async ({ providerSettingsManager, contextProxy }) => {
    const uri = await vscode.window.showSaveDialog({
        filters: { JSON: ["json"] },
        defaultUri: vscode.Uri.file(path.join(os.homedir(), "Documents", "kilo-code-settings.json"))
    })
    
    if (!uri) return
    
    const providerProfiles = await providerSettingsManager.export()
    const globalSettings = await contextProxy.export()
    
    if (!providerProfiles) return
    
    await safeWriteJson(uri.fsPath, { providerProfiles, globalSettings })
}
```

**Safe write**: Atomic file write (temp file + rename).

---

## 🎯 RUST TRANSLATION PATTERNS

```rust
// 1. Context Proxy
pub struct ContextProxy {
    state_cache: DashMap<String, serde_json::Value>,
    secret_cache: DashMap<String, String>,
    storage: Arc<dyn Storage>,
}

// 2. Provider Settings Manager
pub struct ProviderSettingsManager {
    lock: Mutex<()>,
    secret_storage: Arc<dyn SecretStorage>,
}

impl ProviderSettingsManager {
    async fn load(&self) -> Result<ProviderProfiles, Error> {
        let json = self.secret_storage.get("roo_cline_config_api_config").await?;
        Ok(serde_json::from_str(&json)?)
    }
    
    async fn store(&self, profiles: &ProviderProfiles) -> Result<(), Error> {
        let json = serde_json::to_string_pretty(profiles)?;
        self.secret_storage.set("roo_cline_config_api_config", &json).await
    }
}

// 3. Custom Modes Manager
pub struct CustomModesManager {
    cache: RwLock<Option<(Vec<ModeConfig>, Instant)>>,
    cache_ttl: Duration,
}

impl CustomModesManager {
    pub async fn get_custom_modes(&self) -> Result<Vec<ModeConfig>, Error> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((modes, cached_at)) = &*cache {
                if cached_at.elapsed() < self.cache_ttl {
                    return Ok(modes.clone());
                }
            }
        }
        
        // Load from files
        let global_modes = self.load_from_file(&self.global_path()).await?;
        let project_modes = self.load_from_file(&self.project_path()).await?;
        let merged = self.merge_modes(project_modes, global_modes);
        
        // Update cache
        *self.cache.write().await = Some((merged.clone(), Instant::now()));
        
        Ok(merged)
    }
}
```

---

## ✅ COMPLETION CHECKLIST

- [x] All 4 source files analyzed  
- [x] Multi-profile system explained
- [x] YAML parsing edge cases documented
- [x] Cloud sync algorithm traced
- [x] Security validations identified
- [x] Rust patterns defined

**STATUS**: CHUNK-08 COMPLETE (3,200+ words)

# CHUNK-38: PACKAGES/IPC - INTER-PROCESS COMMUNICATION

## 📁 MODULE STRUCTURE

```
Codex/packages/ipc/
├── src/
│   ├── index.ts           (3 lines)   - Public exports
│   ├── ipc-client.ts      (130 lines) - Client implementation
│   └── ipc-server.ts      (135 lines) - Server implementation
├── README.md              (91 lines)  - Documentation
└── package.json           (24 lines)  - Package configuration
```

**Total**: 268 lines of TypeScript + 91 lines of documentation

---

## 🎯 PURPOSE

Enable external applications to control and monitor Codex tasks through socket-based IPC:
1. **Remote Task Control**: Start, cancel, close, resume tasks from external tools
2. **Event Monitoring**: Listen to task lifecycle events (started, completed, aborted)
3. **Automation**: Integrate Codex into CI/CD pipelines or testing frameworks
4. **Multi-Process Architecture**: Separate evaluation/testing processes from main extension

**Primary Use Case**: Automated testing and evaluation of AI coding assistant

---

## 🔌 IPC ARCHITECTURE

### Communication Pattern

```
┌─────────────────────────────────────────────────────────────┐
│                    External Application                     │
│                  (Testing/Evaluation Tool)                  │
└────────────────────────┬────────────────────────────────────┘
                         │ IpcClient
                         │
                         ▼
              ┌──────────────────────┐
              │   Unix Domain Socket │
              │   /tmp/roo-code.sock │
              └──────────────────────┘
                         │
                         ▼ IpcServer
┌─────────────────────────────────────────────────────────────┐
│                 VSCode Extension (Codex)                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │
│  │  Task 1  │  │  Task 2  │  │  Task 3  │                 │
│  └──────────┘  └──────────┘  └──────────┘                 │
└─────────────────────────────────────────────────────────────┘
```

### Socket Paths

**Unix/Linux/macOS**:
```
/tmp/roo-code-{id}.sock
```

**Windows**:
```
\\.\pipe\roo-code-{id}
```

---

## 🖥️ IPC SERVER: ipc-server.ts (135 lines)

### Class Structure

```typescript
import EventEmitter from "node:events"
import { Socket } from "node:net"
import * as crypto from "node:crypto"
import ipc from "node-ipc"

export class IpcServer extends EventEmitter<IpcServerEvents> 
    implements RooCodeIpcServer {
    
    private readonly _socketPath: string
    private readonly _log: (...args: unknown[]) => void
    private readonly _clients: Map<string, Socket>
    private _isListening = false
    
    constructor(socketPath: string, log = console.log) {
        super()
        this._socketPath = socketPath
        this._log = log
        this._clients = new Map()
    }
}
```

### Server Lifecycle

#### 1. Start Listening

```typescript
public listen() {
    this._isListening = true
    
    // Configure node-ipc
    ipc.config.silent = true  // Disable internal logging
    
    // Start server
    ipc.serve(this.socketPath, () => {
        ipc.server.on("connect", (socket) => this.onConnect(socket))
        ipc.server.on("socket.disconnected", (socket) => this.onDisconnect(socket))
        ipc.server.on("message", (data) => this.onMessage(data))
    })
    
    ipc.server.start()
}
```

#### 2. Handle Client Connection

```typescript
private onConnect(socket: Socket) {
    // Generate unique client ID
    const clientId = crypto.randomBytes(6).toString("hex")
    this._clients.set(clientId, socket)
    
    this.log(`[server#onConnect] clientId = ${clientId}, # clients = ${this._clients.size}`)
    
    // Send acknowledgment with client ID and process info
    this.send(socket, {
        type: IpcMessageType.Ack,
        origin: IpcOrigin.Server,
        data: { 
            clientId, 
            pid: process.pid,      // Server process ID
            ppid: process.ppid     // Parent process ID
        },
    })
    
    // Emit connect event
    this.emit(IpcMessageType.Connect, clientId)
}
```

**Client ID Generation**: 12-character hex string (6 random bytes)
- Example: `"a3f7b9c2d1e4"`
- Unique identifier for tracking connected clients

#### 3. Handle Client Disconnection

```typescript
private onDisconnect(destroyedSocket: Socket) {
    let disconnectedClientId: string | undefined
    
    // Find client by socket reference
    for (const [clientId, socket] of this._clients.entries()) {
        if (socket === destroyedSocket) {
            disconnectedClientId = clientId
            this._clients.delete(clientId)
            break
        }
    }
    
    this.log(`[server#socket.disconnected] clientId = ${disconnectedClientId}, # clients = ${this._clients.size}`)
    
    if (disconnectedClientId) {
        this.emit(IpcMessageType.Disconnect, disconnectedClientId)
    }
}
```

#### 4. Handle Incoming Messages

```typescript
private onMessage(data: unknown) {
    // Validate data type
    if (typeof data !== "object") {
        this.log("[server#onMessage] invalid data", data)
        return
    }
    
    // Parse and validate message schema
    const result = ipcMessageSchema.safeParse(data)
    
    if (!result.success) {
        this.log("[server#onMessage] invalid payload", result.error.format(), data)
        return
    }
    
    const payload = result.data
    
    // Handle client-originated messages
    if (payload.origin === IpcOrigin.Client) {
        switch (payload.type) {
            case IpcMessageType.TaskCommand:
                this.emit(IpcMessageType.TaskCommand, payload.clientId, payload.data)
                break
            default:
                this.log(`[server#onMessage] unhandled payload: ${JSON.stringify(payload)}`)
                break
        }
    }
}
```

**Message Validation**: Uses Zod schema (`ipcMessageSchema` from `@clean-code/types`)
- Type-safe parsing
- Automatic validation
- Detailed error reporting

### Sending Messages

#### Broadcast to All Clients

```typescript
public broadcast(message: IpcMessage) {
    ipc.server.broadcast("message", message)
}
```

**Use Case**: Notify all connected clients of global events
- Task system status changes
- Extension shutdown
- Configuration updates

#### Send to Specific Client

```typescript
public send(client: string | Socket, message: IpcMessage) {
    if (typeof client === "string") {
        // Lookup socket by client ID
        const socket = this._clients.get(client)
        
        if (socket) {
            ipc.server.emit(socket, "message", message)
        }
    } else {
        // Direct socket reference
        ipc.server.emit(client, "message", message)
    }
}
```

**Flexibility**: Accept client ID string or socket reference
- **Client ID**: Convenient for application logic
- **Socket**: Direct reference from connection handler

---

## 💻 IPC CLIENT: ipc-client.ts (130 lines)

### Class Structure

```typescript
import EventEmitter from "node:events"
import * as crypto from "node:crypto"
import ipc from "node-ipc"

export class IpcClient extends EventEmitter<IpcClientEvents> {
    private readonly _socketPath: string
    private readonly _id: string
    private readonly _log: (...args: unknown[]) => void
    private _isConnected = false
    private _clientId?: string  // Assigned by server
    
    constructor(socketPath: string, log = console.log) {
        super()
        
        this._socketPath = socketPath
        this._id = `roo-code-evals-${crypto.randomBytes(6).toString("hex")}`
        this._log = log
        
        // Configure and connect
        ipc.config.silent = true
        
        ipc.connectTo(this._id, this.socketPath, () => {
            ipc.of[this._id]?.on("connect", () => this.onConnect())
            ipc.of[this._id]?.on("disconnect", () => this.onDisconnect())
            ipc.of[this._id]?.on("message", (data) => this.onMessage(data))
        })
    }
}
```

**Client ID**: Local identifier for `node-ipc` connection
- Format: `roo-code-evals-{12-char-hex}`
- Different from server-assigned `clientId`

### Connection Lifecycle

#### 1. Connect Event

```typescript
private onConnect() {
    // Prevent duplicate connect events
    if (this._isConnected) {
        return
    }
    
    this.log("[client#onConnect]")
    this._isConnected = true
    this.emit(IpcMessageType.Connect)
}
```

#### 2. Disconnect Event

```typescript
private onDisconnect() {
    // Prevent duplicate disconnect events
    if (!this._isConnected) {
        return
    }
    
    this.log("[client#onDisconnect]")
    this._isConnected = false
    this.emit(IpcMessageType.Disconnect)
}
```

#### 3. Message Handler

```typescript
private onMessage(data: unknown) {
    // Validate data type
    if (typeof data !== "object") {
        this._log("[client#onMessage] invalid data", data)
        return
    }
    
    // Parse and validate
    const result = ipcMessageSchema.safeParse(data)
    
    if (!result.success) {
        this.log("[client#onMessage] invalid payload", result.error, data)
        return
    }
    
    const payload = result.data
    
    // Handle server-originated messages
    if (payload.origin === IpcOrigin.Server) {
        switch (payload.type) {
            case IpcMessageType.Ack:
                // Store server-assigned client ID
                this._clientId = payload.data.clientId
                this.emit(IpcMessageType.Ack, payload.data)
                break
            case IpcMessageType.TaskEvent:
                this.emit(IpcMessageType.TaskEvent, payload.data)
                break
        }
    }
}
```

### Sending Commands

```typescript
public sendCommand(command: TaskCommand) {
    const message: IpcMessage = {
        type: IpcMessageType.TaskCommand,
        origin: IpcOrigin.Client,
        clientId: this._clientId!,  // Must have received Ack first
        data: command,
    }
    
    this.sendMessage(message)
}

public sendMessage(message: IpcMessage) {
    ipc.of[this._id]?.emit("message", message)
}
```

### Client State

```typescript
public get isConnected() {
    return this._isConnected
}

public get isReady() {
    // Connected AND has received client ID from server
    return this._isConnected && this._clientId !== undefined
}
```

**Ready State**: Client must receive `Ack` message before sending commands
- `isConnected`: Socket connection established
- `isReady`: Received server acknowledgment with client ID

### Disconnection

```typescript
public disconnect() {
    try {
        ipc.disconnect(this._id)
        // @TODO: Should we set _isConnected here?
    } catch (error) {
        this.log("[client#disconnect] error disconnecting", error)
    }
}
```

**Note**: Comment indicates potential improvement
- Currently relies on disconnect event callback
- Could immediately set `_isConnected = false`

---

## 📡 MESSAGE PROTOCOL

### Message Types (from `@clean-code/types`)

```typescript
enum IpcMessageType {
    Connect = "Connect",           // Connection established
    Disconnect = "Disconnect",     // Connection closed
    Ack = "Ack",                  // Server acknowledgment
    TaskCommand = "TaskCommand",   // Execute task command
    TaskEvent = "TaskEvent",       // Task lifecycle event
}

enum IpcOrigin {
    Server = "Server",
    Client = "Client",
}
```

### Message Schema

```typescript
type IpcMessage = {
    type: IpcMessageType
    origin: IpcOrigin
    clientId?: string
    data: any
}

// Zod validation schema
const ipcMessageSchema = z.object({
    type: z.nativeEnum(IpcMessageType),
    origin: z.nativeEnum(IpcOrigin),
    clientId: z.string().optional(),
    data: z.any(),
})
```

### Task Commands

```typescript
type TaskCommand = 
    | { commandName: "StartNewTask", data: StartNewTaskData }
    | { commandName: "CancelTask", data: string }      // Task ID
    | { commandName: "CloseTask", data: string }       // Task ID
    | { commandName: "ResumeTask", data: string }      // Task ID

type StartNewTaskData = {
    configuration?: RooCodeSettings
    text: string
    images?: string[]
    newTab?: boolean
}
```

### Task Events

```typescript
type TaskEvent = 
    | { type: "TaskStarted", taskId: string, timestamp: number }
    | { type: "TaskCompleted", taskId: string, timestamp: number }
    | { type: "TaskAborted", taskId: string, timestamp: number }
    | { type: "Message", taskId: string, message: ClineMessage }
```

---

## 🔄 PROTOCOL FLOW

### Connection Handshake

```
CLIENT                          SERVER
  |                               |
  |--- TCP Connect -------------->|
  |                               |
  |<--- Ack (clientId) -----------|
  |                               |
  |   (Client stores clientId)    |
  |   (Client is now ready)       |
  |                               |
```

### Task Command Flow

```
CLIENT                                    SERVER
  |                                         |
  |--- TaskCommand (StartNewTask) -------->|
  |                                         |
  |                          [Server starts task]
  |                                         |
  |<--- TaskEvent (TaskStarted) -----------|
  |                                         |
  |<--- TaskEvent (Message) --------------|
  |<--- TaskEvent (Message) --------------|
  |<--- TaskEvent (Message) --------------|
  |                                         |
  |<--- TaskEvent (TaskCompleted) ---------|
  |                                         |
```

### Resume Task Flow

```
CLIENT                                    SERVER
  |                                         |
  |--- TaskCommand (ResumeTask) ---------->|
  |                                         |
  |                      [Server loads task history]
  |                      [Server re-initializes task]
  |                                         |
  |<--- TaskEvent (TaskStarted) -----------|
  |                                         |
  |       [Task continues from last state]  |
  |                                         |
```

---

## 🔧 USAGE EXAMPLES

### Server Setup (Extension Side)

```typescript
import { IpcServer } from "@clean-code/ipc"
import * as os from "os"
import * as path from "path"

// Create socket path
const socketPath = path.join(os.tmpdir(), `roo-code-${Date.now()}.sock`)

// Initialize server
const server = new IpcServer(socketPath, console.log)

// Handle task commands
server.on("TaskCommand", (clientId, command) => {
    switch (command.commandName) {
        case "StartNewTask":
            const taskId = await startNewTask(command.data)
            server.send(clientId, {
                type: IpcMessageType.TaskEvent,
                origin: IpcOrigin.Server,
                data: { type: "TaskStarted", taskId, timestamp: Date.now() }
            })
            break
        
        case "CancelTask":
            await cancelTask(command.data)
            break
        
        case "ResumeTask":
            await resumeTask(command.data)
            break
    }
})

// Start listening
server.listen()

// Store socket path for clients to discover
await fs.writeFile("/tmp/roo-code-socket-path", socketPath)
```

### Client Usage (Testing Tool)

```typescript
import { IpcClient } from "@clean-code/ipc"
import * as fs from "fs/promises"

// Discover socket path
const socketPath = await fs.readFile("/tmp/roo-code-socket-path", "utf8")

// Connect to server
const client = new IpcClient(socketPath, console.log)

// Wait for ready
await new Promise((resolve) => {
    client.on("Ack", (data) => {
        console.log(`Connected with client ID: ${data.clientId}`)
        resolve()
    })
})

// Listen to task events
client.on("TaskEvent", (event) => {
    console.log("Task event:", event)
})

// Start a new task
client.sendCommand({
    commandName: "StartNewTask",
    data: {
        text: "Refactor this function to use async/await",
        images: [],
        newTab: false,
    }
})

// Wait for completion
await new Promise((resolve) => {
    client.on("TaskEvent", (event) => {
        if (event.type === "TaskCompleted") {
            resolve()
        }
    })
})

// Cleanup
client.disconnect()
```

---

## 🦀 RUST TRANSLATION

```rust
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessageType {
    Connect,
    Disconnect,
    Ack { data: AckData },
    TaskCommand { data: TaskCommand },
    TaskEvent { data: TaskEvent },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcOrigin {
    Server,
    Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub origin: IpcOrigin,
    pub client_id: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckData {
    pub client_id: String,
    pub pid: u32,
    pub ppid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "commandName", content = "data")]
pub enum TaskCommand {
    StartNewTask(StartNewTaskData),
    CancelTask(String),
    CloseTask(String),
    ResumeTask(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartNewTaskData {
    pub configuration: Option<serde_json::Value>,
    pub text: String,
    pub images: Option<Vec<String>>,
    pub new_tab: Option<bool>,
}

/// IPC Server
pub struct IpcServer {
    socket_path: PathBuf,
    clients: Arc<RwLock<HashMap<String, UnixStream>>>,
    is_listening: Arc<RwLock<bool>>,
}

impl IpcServer {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            clients: Arc::new(RwLock::new(HashMap::new())),
            is_listening: Arc::new(RwLock::new(false)),
        }
    }
    
    pub async fn listen(&self) -> Result<()> {
        *self.is_listening.write().await = true;
        
        // Remove existing socket file
        let _ = tokio::fs::remove_file(&self.socket_path).await;
        
        let listener = UnixListener::bind(&self.socket_path)?;
        log::info!("IPC server listening on {:?}", self.socket_path);
        
        loop {
            let (stream, _) = listener.accept().await?;
            
            let clients = Arc::clone(&self.clients);
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, clients).await {
                    log::error!("Error handling client: {}", e);
                }
            });
        }
    }
    
    pub async fn broadcast(&self, message: &IpcMessage) -> Result<()> {
        let clients = self.clients.read().await;
        let message_json = serde_json::to_string(message)?;
        
        for (client_id, stream) in clients.iter() {
            if let Err(e) = send_message(stream, &message_json).await {
                log::error!("Failed to broadcast to {}: {}", client_id, e);
            }
        }
        
        Ok(())
    }
    
    pub async fn send(&self, client_id: &str, message: &IpcMessage) -> Result<()> {
        let clients = self.clients.read().await;
        
        if let Some(stream) = clients.get(client_id) {
            let message_json = serde_json::to_string(message)?;
            send_message(stream, &message_json).await?;
        }
        
        Ok(())
    }
}

async fn handle_client(
    mut stream: UnixStream,
    clients: Arc<RwLock<HashMap<String, UnixStream>>>,
) -> Result<()> {
    // Generate client ID
    let client_id = uuid::Uuid::new_v4().to_string();
    
    // Send acknowledgment
    let ack = IpcMessage {
        msg_type: "Ack".to_string(),
        origin: IpcOrigin::Server,
        client_id: Some(client_id.clone()),
        data: serde_json::json!({
            "clientId": client_id,
            "pid": std::process::id(),
            "ppid": 0, // TODO: Get parent PID
        }),
    };
    
    let ack_json = serde_json::to_string(&ack)?;
    send_message(&stream, &ack_json).await?;
    
    // Register client
    clients.write().await.insert(client_id.clone(), stream.try_clone()?);
    
    // Handle messages
    let mut buffer = vec![0u8; 8192];
    loop {
        let n = stream.read(&mut buffer).await?;
        
        if n == 0 {
            break; // Connection closed
        }
        
        let message_str = String::from_utf8_lossy(&buffer[..n]);
        
        match serde_json::from_str::<IpcMessage>(&message_str) {
            Ok(message) => {
                // Handle message
                handle_message(message).await?;
            }
            Err(e) => {
                log::error!("Failed to parse message: {}", e);
            }
        }
    }
    
    // Cleanup
    clients.write().await.remove(&client_id);
    log::info!("Client {} disconnected", client_id);
    
    Ok(())
}

async fn send_message(stream: &UnixStream, message: &str) -> Result<()> {
    let mut stream = stream;
    stream.write_all(message.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    Ok(())
}

/// IPC Client
pub struct IpcClient {
    socket_path: PathBuf,
    stream: Option<UnixStream>,
    client_id: Option<String>,
    is_connected: bool,
}

impl IpcClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            stream: None,
            client_id: None,
            is_connected: false,
        }
    }
    
    pub async fn connect(&mut self) -> Result<()> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        self.stream = Some(stream);
        self.is_connected = true;
        
        // Wait for Ack
        let ack = self.receive_message().await?;
        
        if let Some(client_id) = ack.client_id {
            self.client_id = Some(client_id);
        }
        
        Ok(())
    }
    
    pub async fn send_command(&mut self, command: TaskCommand) -> Result<()> {
        let message = IpcMessage {
            msg_type: "TaskCommand".to_string(),
            origin: IpcOrigin::Client,
            client_id: self.client_id.clone(),
            data: serde_json::to_value(command)?,
        };
        
        self.send_message(&message).await
    }
    
    pub async fn send_message(&mut self, message: &IpcMessage) -> Result<()> {
        if let Some(ref mut stream) = self.stream {
            let message_json = serde_json::to_string(message)?;
            stream.write_all(message_json.as_bytes()).await?;
            stream.write_all(b"\n").await?;
        }
        
        Ok(())
    }
    
    pub async fn receive_message(&mut self) -> Result<IpcMessage> {
        if let Some(ref mut stream) = self.stream {
            let mut buffer = vec![0u8; 8192];
            let n = stream.read(&mut buffer).await?;
            
            let message_str = String::from_utf8_lossy(&buffer[..n]);
            let message = serde_json::from_str(&message_str)?;
            
            Ok(message)
        } else {
            Err(anyhow::anyhow!("Not connected"))
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_connected && self.client_id.is_some()
    }
}
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Unix Domain Sockets

**Why not TCP/IP?**
- **Security**: No network exposure
- **Performance**: No network stack overhead
- **Simplicity**: No port management
- **Local-only**: Designed for same-machine communication

### 2. Client ID Assignment

**Server-assigned vs client-generated**:
- **Server assigns**: Centralized control, prevents collisions
- **Client includes in all messages**: Easy message routing
- **Stored after Ack**: Client must wait for server response

### 3. Message Validation

**Zod schema validation**:
- Type-safe at runtime
- Detailed error messages
- Prevents invalid data from crashing server
- Graceful degradation

### 4. Event Emitter Pattern

**Why EventEmitter?**
- **Familiar**: Standard Node.js pattern
- **Decoupled**: Handler registration separate from implementation
- **Flexible**: Multiple listeners per event
- **Async-friendly**: Works with async handlers

### 5. Silent Mode for node-ipc

```typescript
ipc.config.silent = true
```

**Why disable logging?**
- `node-ipc` has verbose default logging
- Extension has its own logging system
- Prevents console pollution
- Custom log function passed to constructors

---

## 🔗 DEPENDENCIES

**NPM Packages**:
- `node-ipc` (^12.0.0) - IPC implementation
- `@clean-code/types` (workspace) - Shared types and schemas

**Rust Crates**:
- `tokio` (1.35) - Async runtime, Unix sockets
- `serde` (1.0) - Serialization
- `serde_json` (1.0) - JSON parsing
- `uuid` (1.6) - Client ID generation
- `anyhow` (1.0) - Error handling
- `log` (0.4) - Logging

---

## 📊 PERFORMANCE CHARACTERISTICS

### Latency
- **Local socket**: ~0.1-1ms per message
- **No serialization overhead**: JSON is fast
- **No network stack**: Direct kernel communication

### Scalability
- **Multiple clients**: Supported, each tracked separately
- **Concurrent messages**: Async handlers prevent blocking
- **Memory**: O(n) where n = connected clients

### Reliability
- **Automatic reconnection**: Not implemented (client must handle)
- **Message delivery**: No acknowledgment (fire-and-forget)
- **Connection loss**: Detected via socket events

---

## 🎓 KEY TAKEAWAYS

✅ **Socket-Based IPC**: Unix domain sockets for local communication

✅ **Type-Safe Protocol**: Zod validation ensures message integrity

✅ **Bidirectional**: Server can broadcast, clients can command

✅ **Event-Driven**: EventEmitter pattern for clean architecture

✅ **Client ID System**: Server-assigned IDs for routing

✅ **External Automation**: Enables testing and CI/CD integration

✅ **Small Package**: Only 268 lines, focused functionality

---

## 📊 TRANSLATION ESTIMATE

**Complexity**: Medium
**Estimated Effort**: 5-7 hours
**Lines of Rust**: ~400-500 lines
**Dependencies**: `tokio`, `serde_json`, async I/O
**Key Challenge**: Async socket handling, Unix vs Windows sockets
**Risk**: Medium - platform-specific socket code

---

**Status**: ✅ Deep analysis complete
**Next**: CHUNK-40 (packages/types/)

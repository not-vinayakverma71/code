# CHUNK-36: PACKAGES/EVALS - AI EVALUATION SYSTEM (DEEP ANALYSIS)

## 🎯 OVERVIEW

**Purpose**: Distributed AI evaluation platform for testing coding capabilities across multiple programming languages in isolated Docker containers.

**Scale**: 
- 20+ TypeScript files
- 5+ supported languages (Go, Java, JavaScript, Python, Rust)
- Docker-based architecture with PostgreSQL + Redis
- Parallel task execution (1-25 concurrent tasks)

---

## 📐 ARCHITECTURE

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Next.js Web App                          │
│              (apps/web-evals)                               │
│  - Create evaluation runs                                   │
│  - Monitor progress (SSE)                                   │
│  - View results                                             │
└────────────┬────────────────────────────┬───────────────────┘
             │                            │
             ▼                            ▼
      ┌──────────┐                 ┌──────────┐
      │PostgreSQL│                 │  Redis   │
      │  Tasks   │                 │ Pub/Sub  │
      │  Runs    │                 │Heartbeat │
      └──────────┘                 └──────────┘
             │                            │
             ▼                            ▼
    ┌────────────────────────────────────────────────┐
    │         Controller Container                   │
    │    (evals-runner in controller mode)          │
    │  - p-queue for task distribution               │
    │  - Git workspace setup                         │
    │  - Spawn N runner containers                   │
    │  - Result aggregation                          │
    └────────────────┬───────────────────────────────┘
                     │
         ┌───────────┴───────────┬───────────┐
         ▼                       ▼           ▼
  ┌─────────────┐         ┌─────────────┐   ...
  │  Runner 1   │         │  Runner 2   │
  │             │         │             │
  │ - VS Code   │         │ - VS Code   │
  │ - Roo Ext   │         │ - Roo Ext   │
  │ - IPC       │         │ - IPC       │
  │ - Unit Test │         │ - Unit Test │
  └─────────────┘         └─────────────┘
```

### Key Design Decisions

**1. Container-Per-Task Model**
- **Problem**: Running tasks sequentially causes memory accumulation, state contamination
- **Solution**: Each task gets fresh container → clean slate, parallel execution
- **Result**: Isolated failures, automatic memory cleanup

**2. Controller + Runners Architecture**
- **Controller**: Orchestrates (p-queue in-memory, not Redis)
- **Runners**: Execute single task → terminate
- **Why**: Separation of concerns, scalability

**3. Redis for Events, Not Queuing**
- **Pub/Sub**: Real-time progress updates
- **Registration**: Track active runners
- **Heartbeat**: Monitor controller health
- **NOT used for**: Task queue (p-queue handles this in controller memory)

---

## 📂 FILE STRUCTURE

### packages/evals/

```
evals/
├── src/
│   ├── cli/                    - Command-line interface
│   │   ├── index.ts           - CLI entry point
│   │   ├── runEvals.ts        - Controller: run orchestration
│   │   ├── runTask.ts         - Runner: single task execution
│   │   ├── runUnitTest.ts     - Test execution per language
│   │   ├── runCi.ts           - CI/CD integration
│   │   ├── redis.ts           - Redis client utilities
│   │   └── utils.ts           - Helper functions
│   ├── db/                     - Database layer
│   │   ├── db.ts              - Drizzle ORM client
│   │   ├── schema.ts          - Database schema definitions
│   │   └── queries/
│   │       ├── runs.ts        - Run CRUD operations
│   │       ├── tasks.ts       - Task CRUD operations
│   │       ├── taskMetrics.ts - Metrics aggregation
│   │       ├── errors.ts      - Error tracking
│   │       └── copyRun.ts     - Run duplication
│   ├── exercises/
│   │   └── index.ts           - Exercise type definitions
│   └── index.ts               - Package exports
├── scripts/
│   └── setup.sh               - macOS local development setup
├── Dockerfile.runner          - Container image for controller/runners
├── Dockerfile.web             - Web app container image
├── docker-compose.yml         - Full stack orchestration
├── drizzle.config.ts          - Database migrations config
├── vitest.config.ts           - Test configuration
├── ARCHITECTURE.md            - System design documentation
├── ADDING-EVALS.md            - Guide for adding exercises
└── README.md                  - Setup and usage guide
```

---

## 🔧 CORE MODULES

### 1. CLI System (src/cli/)

#### runEvals.ts - Controller Orchestration

```typescript
import PQueue from 'p-queue'
import { db } from '../db'
import { spawnRunner } from './utils'

export async function runEvals(runId: string) {
    const run = await db.query.runs.findFirst({ 
        where: eq(runs.id, runId),
        with: { tasks: true }
    })
    
    // In-memory queue with concurrency limit
    const queue = new PQueue({ 
        concurrency: run.concurrency,
        autoStart: true 
    })
    
    // Setup git workspace
    await setupWorkspace(run.exercisesRepo)
    
    // Start heartbeat
    const heartbeat = setInterval(() => {
        redis.publish(`run:${runId}:heartbeat`, Date.now())
    }, 5000)
    
    // Queue all tasks
    for (const task of run.tasks) {
        queue.add(async () => {
            // Spawn isolated runner container
            const container = await spawnRunner({
                taskId: task.id,
                runId: run.id,
                exercise: task.exercise,
                language: task.language,
                model: run.model,
            })
            
            // Wait for completion
            await container.wait()
            
            // Collect results
            await db.update(tasks)
                .set({ status: 'completed', finishedAt: new Date() })
                .where(eq(tasks.id, task.id))
        })
    }
    
    // Wait for all tasks to complete
    await queue.onIdle()
    
    clearInterval(heartbeat)
    
    // Finalize run
    await finalizeRun(runId)
}
```

#### runTask.ts - Single Task Execution

```typescript
import { IpcClient } from '../ipc'

export async function runTask(taskId: string) {
    const task = await db.query.tasks.findFirst({ 
        where: eq(tasks.id, taskId) 
    })
    
    // 1. Launch VS Code with Roo extension
    const vscode = await launchVSCode({
        workspace: `/workspace/${task.language}/${task.exercise}`,
        extensions: ['roo-code'],
    })
    
    // 2. Connect via IPC
    const ipc = new IpcClient('/tmp/roo.sock')
    await ipc.connect()
    
    // 3. Load exercise prompt
    const prompt = await loadPrompt(task.language, task.exercise)
    
    // 4. Send to AI agent
    const taskHandle = await ipc.startTask({
        prompt,
        mode: 'code',
        model: task.model,
        settings: task.settings,
    })
    
    // 5. Stream events to Redis
    ipc.on('event', (event) => {
        redis.publish(`run:${task.runId}:events`, JSON.stringify({
            taskId: task.id,
            event: event.type,
            data: event.data,
            timestamp: Date.now(),
        }))
    })
    
    // 6. Wait for completion (with 30min timeout)
    const result = await taskHandle.waitForCompletion({ 
        timeout: 30 * 60 * 1000 
    })
    
    // 7. Run unit tests
    const testResult = await runUnitTest(task.language, task.exercise)
    
    // 8. Save metrics
    await db.insert(taskMetrics).values({
        taskId: task.id,
        tokensIn: result.tokensIn,
        tokensOut: result.tokensOut,
        cost: result.cost,
        duration: result.duration,
        testsPassed: testResult.passed,
        testsFailed: testResult.failed,
        success: testResult.success,
    })
    
    // 9. Cleanup
    await vscode.close()
    process.exit(0)
}
```

#### runUnitTest.ts - Language-Specific Testing

```typescript
export async function runUnitTest(
    language: string, 
    exercise: string
): Promise<TestResult> {
    const workspacePath = `/workspace/${language}/${exercise}`
    
    switch (language) {
        case 'python':
            return runCommand(
                'uv run python3 -m pytest -o markers=task',
                workspacePath
            )
        
        case 'go':
            return runCommand('go test', workspacePath)
        
        case 'rust':
            return runCommand('cargo test', workspacePath)
        
        case 'javascript':
            return runCommand('npm test', workspacePath)
        
        case 'java':
            return runCommand('mvn test', workspacePath)
        
        default:
            throw new Error(`Unsupported language: ${language}`)
    }
}

interface TestResult {
    passed: number
    failed: number
    success: boolean
    output: string
    duration: number
}
```

---

### 2. Database Layer (src/db/)

#### schema.ts - Database Schema

```typescript
import { pgTable, text, integer, timestamp, boolean, jsonb } from 'drizzle-orm/pg-core'

export const runs = pgTable('runs', {
    id: text('id').primaryKey(),
    status: text('status').$type<'pending' | 'running' | 'completed' | 'failed'>(),
    model: text('model').notNull(),
    concurrency: integer('concurrency').default(5),
    exercisesRepo: text('exercises_repo'),
    settings: jsonb('settings'),
    createdAt: timestamp('created_at').defaultNow(),
    startedAt: timestamp('started_at'),
    completedAt: timestamp('completed_at'),
})

export const tasks = pgTable('tasks', {
    id: text('id').primaryKey(),
    runId: text('run_id').references(() => runs.id),
    language: text('language').notNull(),
    exercise: text('exercise').notNull(),
    status: text('status').$type<'pending' | 'running' | 'completed' | 'failed'>(),
    createdAt: timestamp('created_at').defaultNow(),
    startedAt: timestamp('started_at'),
    completedAt: timestamp('completed_at'),
})

export const taskMetrics = pgTable('task_metrics', {
    id: text('id').primaryKey(),
    taskId: text('task_id').references(() => tasks.id),
    tokensIn: integer('tokens_in'),
    tokensOut: integer('tokens_out'),
    cost: integer('cost'), // in cents
    duration: integer('duration'), // milliseconds
    testsPassed: integer('tests_passed'),
    testsFailed: integer('tests_failed'),
    success: boolean('success'),
    toolUsage: jsonb('tool_usage'),
})

export const taskErrors = pgTable('task_errors', {
    id: text('id').primaryKey(),
    taskId: text('task_id').references(() => tasks.id),
    errorType: text('error_type'),
    errorMessage: text('error_message'),
    stackTrace: text('stack_trace'),
    timestamp: timestamp('timestamp').defaultNow(),
})
```

---

### 3. Docker Configuration

#### Dockerfile.runner - Multi-Language Container

```dockerfile
FROM ubuntu:22.04

# Install base dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    ca-certificates

# Install Node.js 20
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

# Install Python 3.13
RUN apt-get install -y software-properties-common \
    && add-apt-repository ppa:deadsnakes/ppa \
    && apt-get install -y python3.13 python3.13-venv \
    && curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Go 1.24
ARG GO_VERSION=1.24.2
RUN curl -OL https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz \
    && tar -C /usr/local -xzf go${GO_VERSION}.linux-amd64.tar.gz \
    && rm go${GO_VERSION}.linux-amd64.tar.gz
ENV PATH="/usr/local/go/bin:${PATH}"

# Install Rust 1.85
ARG RUST_VERSION=1.85.1
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
    -y --default-toolchain ${RUST_VERSION}
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Java 17
RUN apt-get install -y openjdk-17-jdk maven

# Install VS Code Server
RUN curl -Lk 'https://code.visualstudio.com/sha/download?build=stable&os=cli-alpine-x64' \
    --output /tmp/vscode-cli.tar.gz \
    && tar -xf /tmp/vscode-cli.tar.gz -C /usr/local/bin

# Install Docker (for Docker-in-Docker capability)
RUN curl -fsSL https://get.docker.com | sh

# Copy evals CLI
COPY dist /app
WORKDIR /app

# Clone exercises repository
ARG EXERCISES_REPO=https://github.com/RooCodeInc/Roo-Code-Evals.git
RUN git clone ${EXERCISES_REPO} /exercises

ENTRYPOINT ["node", "/app/cli/index.js"]
```

#### docker-compose.yml - Full Stack

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: password
      POSTGRES_DB: evals_development
    ports:
      - "5433:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    ports:
      - "6380:6379"
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data

  web:
    build:
      context: .
      dockerfile: Dockerfile.web
    ports:
      - "3446:3000"
    environment:
      DATABASE_URL: postgres://postgres:password@postgres:5432/evals_development
      REDIS_URL: redis://redis:6379
      OPENROUTER_API_KEY: ${OPENROUTER_API_KEY}
    depends_on:
      - postgres
      - redis
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock

volumes:
  postgres_data:
  redis_data:
```

---

## 🎯 KEY WORKFLOWS

### 1. Create Evaluation Run

```
User (Web UI)
    ↓
[POST /api/runs]
    ↓
Create run in DB
    ↓
Create tasks for exercises
    ↓
Spawn controller container
    ↓
Return run ID to user
```

### 2. Controller Orchestration

```
Controller starts
    ↓
Load run configuration from DB
    ↓
Setup git workspace
    ↓
Initialize p-queue (concurrency limit)
    ↓
Start Redis heartbeat
    ↓
For each task:
    ↓
    Add to queue → Spawn runner container
    ↓
Wait for all tasks (queue.onIdle())
    ↓
Aggregate results
    ↓
Finalize run in DB
    ↓
Exit
```

### 3. Task Execution in Runner

```
Runner container starts
    ↓
Launch VS Code + Roo extension
    ↓
Connect via IPC
    ↓
Load exercise prompt
    ↓
Send prompt to AI agent
    ↓
Stream events to Redis:
    - Token usage
    - Tool calls
    - File changes
    ↓
Wait for AI completion (30min timeout)
    ↓
Run unit tests
    ↓
Save metrics to DB
    ↓
Publish completion event
    ↓
Container exits
```

---

## 🦀 RUST TRANSLATION CHALLENGES

### Challenge 1: VS Code Integration

**TypeScript**: IPC via Unix sockets to VS Code extension
**Rust**: Need equivalent IPC mechanism

```rust
// Option 1: Keep using VS Code with IPC
use tokio::net::UnixStream;

pub struct IpcClient {
    stream: UnixStream,
}

impl IpcClient {
    pub async fn connect(socket_path: &str) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        Ok(Self { stream })
    }
    
    pub async fn start_task(&mut self, params: TaskParams) -> Result<TaskHandle> {
        // Send JSON-RPC request
        // Wait for response
        // Return handle
    }
}
```

**Alternative**: Drop VS Code dependency, run Roo Code as standalone Rust binary

### Challenge 2: Docker-in-Docker

**TypeScript**: Uses Docker SDK to spawn containers
**Rust**: Use `bollard` crate

```rust
use bollard::Docker;
use bollard::container::{CreateContainerOptions, Config};

pub async fn spawn_runner(config: RunnerConfig) -> Result<String> {
    let docker = Docker::connect_with_local_defaults()?;
    
    let container = docker.create_container(
        Some(CreateContainerOptions {
            name: format!("runner-{}", config.task_id),
        }),
        Config {
            image: Some("evals-runner:latest"),
            env: Some(vec![
                format!("TASK_ID={}", config.task_id),
                format!("RUN_ID={}", config.run_id),
            ]),
            cmd: Some(vec!["run-task", &config.task_id]),
            ..Default::default()
        },
    ).await?;
    
    docker.start_container(&container.id, None).await?;
    
    Ok(container.id)
}
```

### Challenge 3: p-queue Equivalent

**TypeScript**: `p-queue` package
**Rust**: Custom implementation or `async-std`

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct TaskQueue {
    semaphore: Arc<Semaphore>,
    tasks: Vec<Task>,
}

impl TaskQueue {
    pub fn new(concurrency: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(concurrency)),
            tasks: Vec::new(),
        }
    }
    
    pub async fn add<F, Fut>(&self, task: F) 
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        
        tokio::spawn(async move {
            let _permit = permit; // Hold until task completes
            task().await.ok();
        });
    }
    
    pub async fn wait_all(&self) {
        // Wait for all permits to be available
        let _guards: Vec<_> = (0..self.semaphore.available_permits())
            .map(|_| self.semaphore.try_acquire())
            .collect();
    }
}
```

---

## 📊 TRANSLATION COMPLEXITY

| Component | Lines | Complexity | Effort | Notes |
|-----------|-------|------------|--------|-------|
| CLI system | ~2,000 | High | 15-20h | Docker, IPC, orchestration |
| Database layer | ~1,500 | Medium | 8-10h | Use diesel or sqlx |
| Docker config | ~200 | Low | 2-3h | Dockerfile mostly same |
| Test runners | ~500 | Low | 3-4h | Shell command execution |
| **TOTAL** | **~4,200** | **High** | **30-40 hours** | VS Code dependency is key blocker |

---

## 🎯 KEY TAKEAWAYS

✅ **Not Core to Lapce IDE** - This is a separate evaluation system

✅ **Translation Priority: LOW** - Only needed if building evaluation infrastructure

✅ **Key Blocker**: VS Code dependency
- Option A: Keep VS Code + IPC (hybrid approach)
- Option B: Standalone Roo Code in Rust (major refactor)

✅ **Docker orchestration** translates cleanly with `bollard` crate

✅ **Database schema** works with Diesel/SQLx

✅ **If Translating**: Start with database + Docker, defer VS Code integration

---

**Status**: ✅ Deep analysis complete for packages/evals
**Recommendation**: **SKIP** for initial Lapce translation (not core IDE functionality)
**Next**: CHUNK-37-39 (packages/types - deep analysis)

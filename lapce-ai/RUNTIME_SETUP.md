# Runtime Setup Guide

**Date**: 2025-10-18  
**Status**: ✅ Configuration Fixed - Ready for API Key Setup

---

## ✅ Configuration Fixed

The `lapce-ipc.toml` configuration file has been corrected to match all required fields in `IpcConfig`:

### Updated Config Structure

```toml
[server]
- socket_path = "/tmp/lapce-ai.sock"
- max_connections = 1000
- idle_timeout_secs = 300          ✅ ADDED
- max_message_size = 10485760       ✅ MOVED from [performance]
- buffer_pool_size = 100            ✅ MOVED from [performance]
- enable_auto_reconnect = true      ✅ ADDED
- reconnect_delay_ms = 50           ✅ ADDED

[providers]                         ✅ NEW SECTION
- enabled_providers = ["openai", "anthropic", "gemini", "xai"]
- default_provider = "openai"
- fallback_enabled = true
- fallback_order = ["openai", "anthropic", "gemini"]
- load_balance = false
- circuit_breaker_enabled = true
- circuit_breaker_threshold = 5

[performance]
- enable_compression = false
- compression_threshold = 1024      ✅ ADDED
- enable_binary_protocol = true     ✅ ADDED
- worker_threads = 4
- max_concurrent_requests = 100     ✅ ADDED
- request_timeout_secs = 30         ✅ ADDED

[security]
- enable_tls = false                ✅ ADDED
- tls_cert_path = ""               ✅ ADDED
- tls_key_path = ""                ✅ ADDED
- allowed_origins = ["*"]          ✅ ADDED
- rate_limit_per_second = 1000     ✅ ADDED
- max_request_size = 10485760      ✅ ADDED

[monitoring]
- enable_metrics = true
- metrics_port = 9090
- metrics_endpoint = "/metrics"     ✅ ADDED
- enable_tracing = false            ✅ ADDED
- log_level = "info"                ✅ ADDED
- health_check_interval_secs = 5    ✅ ADDED
```

---

## Current Status

```bash
$ ./target/debug/lapce_ipc_server
✅ Starting Lapce IPC Server
✅ Configuration loaded from: lapce-ipc.toml
❌ Failed to validate provider configuration
   → No AI providers configured
```

**Next Step**: Set up API keys for AI providers

---

## API Key Setup

The server requires at least **one AI provider** to be configured via environment variables.

### Supported Providers

#### 1. OpenAI (Recommended)
```bash
export OPENAI_API_KEY="sk-..."
```
- Get API key: https://platform.openai.com/api-keys
- Models: GPT-4, GPT-3.5, etc.

#### 2. Anthropic
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```
- Get API key: https://console.anthropic.com/
- Models: Claude 3.5 Sonnet, Claude 3 Opus, etc.

#### 3. Google Gemini
```bash
export GEMINI_API_KEY="..."
```
- Get API key: https://ai.google.dev/
- Models: Gemini 1.5 Pro, Gemini 1.5 Flash, etc.

#### 4. xAI (Grok)
```bash
export XAI_API_KEY="xai-..."
```
- Get API key: https://x.ai/
- Models: Grok, etc.

#### 5. Azure OpenAI
```bash
export AZURE_OPENAI_API_KEY="..."
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com/"
export AZURE_OPENAI_DEPLOYMENT_NAME="gpt-4"  # Optional
```

#### 6. AWS Bedrock
```bash
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"  # Optional
```

#### 7. Google Vertex AI
```bash
export VERTEX_PROJECT_ID="your-project-id"
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/credentials.json"
```

#### 8. OpenRouter
```bash
export OPENROUTER_API_KEY="sk-or-..."
```
- Get API key: https://openrouter.ai/keys
- Access to 100+ models via one API

---

## Quick Start

### Option 1: Use OpenAI (Simplest)

```bash
# Set your OpenAI API key
export OPENAI_API_KEY="sk-your-key-here"

# Start the server
cd /home/verma/lapce/lapce-ai
./target/debug/lapce_ipc_server
```

### Option 2: Use Multiple Providers (Fallback)

```bash
# Configure multiple providers for redundancy
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="..."

# Start the server (will use OpenAI by default, fallback to others)
./target/debug/lapce_ipc_server
```

### Option 3: Create Environment File

```bash
# Create .env file in lapce-ai directory
cat > /home/verma/lapce/lapce-ai/.env << 'EOF'
OPENAI_API_KEY=sk-your-key-here
ANTHROPIC_API_KEY=sk-ant-your-key-here
GEMINI_API_KEY=your-key-here
EOF

# Load environment variables
source .env

# Start the server
./target/debug/lapce_ipc_server
```

---

## Verification Steps

### 1. Test Server Startup

```bash
cd /home/verma/lapce/lapce-ai
export OPENAI_API_KEY="sk-..."
./target/debug/lapce_ipc_server
```

**Expected Output**:
```
✅ Starting Lapce IPC Server
✅ Configuration loaded from: lapce-ipc.toml
✅ Initializing provider manager...
✅ Registered provider: openai
✅ IPC server listening on: /tmp/lapce-ai.sock
✅ Provider streaming handler registered
✅ Server started successfully
```

### 2. Check Socket File

```bash
ls -l /tmp/lapce-ai.sock
# Should show: srwxrwxr-x ... /tmp/lapce-ai.sock
```

### 3. Test IPC Connection (Optional)

If you have `ipc_test_server` binary:
```bash
# In another terminal
./target/debug/ipc_test_server
```

---

## Server Configuration Reference

### Socket Path
Default: `/tmp/lapce-ai.sock`

To change:
```toml
[server]
socket_path = "/your/custom/path.sock"
```

### Performance Tuning

```toml
[performance]
worker_threads = 8              # More threads for heavy load
max_concurrent_requests = 200   # More concurrent requests
request_timeout_secs = 60       # Longer timeout
```

### Provider Selection

```toml
[providers]
default_provider = "anthropic"  # Use Claude by default
fallback_enabled = true         # Auto-fallback on errors
fallback_order = ["anthropic", "openai", "gemini"]
```

---

## Troubleshooting

### Error: "No such file or directory"
**Cause**: Binary not found or wrong directory  
**Solution**: 
```bash
cd /home/verma/lapce/lapce-ai
./target/debug/lapce_ipc_server
```

### Error: "missing field `idle_timeout_secs`"
**Cause**: Old config file format  
**Solution**: ✅ FIXED - Config file has been updated

### Error: "No AI providers configured"
**Cause**: Missing API keys  
**Solution**: Set at least one provider's API key (see API Key Setup above)

### Error: "Permission denied" on socket
**Cause**: Socket path not writable  
**Solution**: Change socket_path to `/tmp/` or user-writable directory

### Error: "Address already in use"
**Cause**: Another instance is running  
**Solution**: 
```bash
# Kill existing instance
pkill lapce_ipc_server

# Remove old socket
rm /tmp/lapce-ai.sock

# Restart
./target/debug/lapce_ipc_server
```

---

## Production Deployment

### Systemd Service (Linux)

Create `/etc/systemd/system/lapce-ai.service`:

```ini
[Unit]
Description=Lapce AI IPC Server
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/home/verma/lapce/lapce-ai
Environment="OPENAI_API_KEY=sk-your-key"
Environment="ANTHROPIC_API_KEY=sk-ant-your-key"
ExecStart=/home/verma/lapce/lapce-ai/target/release/lapce_ipc_server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable lapce-ai
sudo systemctl start lapce-ai
sudo systemctl status lapce-ai
```

### Docker Deployment

```dockerfile
FROM rust:latest
WORKDIR /app
COPY . .
RUN cargo build --release --bin lapce_ipc_server
CMD ["./target/release/lapce_ipc_server"]
```

```bash
docker build -t lapce-ai .
docker run -e OPENAI_API_KEY=sk-... -v /tmp:/tmp lapce-ai
```

---

## Next Steps

1. ✅ Configuration fixed
2. ⏭️ Set API key(s) for your preferred provider(s)
3. ⏭️ Start the server
4. ⏭️ Connect Lapce UI to IPC server
5. ⏭️ Test streaming chat functionality

---

## Support

- Configuration issues: Check this guide
- API key issues: Verify on provider's website
- Runtime errors: Check server logs (stdout)
- Performance issues: Adjust `[performance]` settings in config

**Status**: 🟢 Ready to run with API keys

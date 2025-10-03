#!/bin/bash
# Production deployment script

set -e

echo "🚀 Deploying Semantic Search..."

# Build release binaries
echo "📦 Building release binaries..."
cargo build --release --all

# Run tests
echo "🧪 Running tests..."
cargo test --release

# Build Docker image
echo "🐳 Building Docker image..."
docker build -f Dockerfile.semantic -t semantic-search:latest .

# Push to registry
if [ "$1" == "push" ]; then
    echo "📤 Pushing to registry..."
    docker tag semantic-search:latest ghcr.io/lapce/semantic-search:latest
    docker push ghcr.io/lapce/semantic-search:latest
fi

# Deploy to Kubernetes
if [ "$1" == "deploy" ]; then
    echo "☸️ Deploying to Kubernetes..."
    kubectl apply -f k8s/deployment.yaml
    kubectl rollout status deployment/semantic-search -n lapce
fi

echo "✅ Deployment complete!"

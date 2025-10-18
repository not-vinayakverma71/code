# AWS Credential Rotation Runbook - SEM-014-B

## Overview
This runbook covers rotating AWS credentials for the semantic search system.

## Pre-Rotation Checklist

- [ ] New AWS credentials generated
- [ ] Backup current configuration
- [ ] Maintenance window scheduled
- [ ] Monitoring alerts configured

## Rotation Steps

### 1. Generate New Credentials

```bash
# Via AWS CLI
aws iam create-access-key --user-name semantic-search-user

# Save output:
# AccessKeyId: AKIA...
# SecretAccessKey: ...
```

### 2. Test New Credentials

```bash
# Test locally first
export AWS_ACCESS_KEY_ID_NEW=<new-key>
export AWS_SECRET_ACCESS_KEY_NEW=<new-secret>

AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID_NEW \
AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY_NEW \
aws bedrock-runtime invoke-model \
  --model-id amazon.titan-embed-text-v2:0 \
  --body '{"inputText": "test"}' \
  --region us-east-1 \
  test-output.json
```

### 3. Update Configuration

```bash
# Backup current config
sudo cp /etc/semantic-search/config.env /etc/semantic-search/config.env.bak

# Update credentials
sudo vim /etc/semantic-search/config.env
# Update AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY
```

### 4. Reload Service

```bash
# Reload without downtime
sudo systemctl reload semantic-search

# Or restart if reload not supported
sudo systemctl restart semantic-search
```

### 5. Verify Operation

```bash
# Check service health
curl http://localhost:8080/health

# Test search functionality
curl -X POST http://localhost:8080/search \
  -H "Content-Type: application/json" \
  -d '{"query": "test query", "limit": 10}'

# Check metrics for errors
curl http://localhost:9090/metrics | grep aws_titan_errors
```

### 6. Deactivate Old Credentials

```bash
# After verification period (e.g., 24 hours)
aws iam delete-access-key \
  --user-name semantic-search-user \
  --access-key-id <old-key>
```

## Rollback Procedure

```bash
# If issues occur, restore old credentials
sudo cp /etc/semantic-search/config.env.bak /etc/semantic-search/config.env
sudo systemctl restart semantic-search
```

## Automation Script

```bash
#!/bin/bash
# rotate-credentials.sh

set -e

# Configuration
USER="semantic-search-user"
CONFIG_FILE="/etc/semantic-search/config.env"
HEALTH_URL="http://localhost:8080/health"

# Generate new credentials
echo "Generating new credentials..."
NEW_CREDS=$(aws iam create-access-key --user-name $USER --output json)
NEW_KEY=$(echo $NEW_CREDS | jq -r '.AccessKey.AccessKeyId')
NEW_SECRET=$(echo $NEW_CREDS | jq -r '.AccessKey.SecretAccessKey')

# Backup current config
echo "Backing up configuration..."
sudo cp $CONFIG_FILE ${CONFIG_FILE}.bak

# Update configuration
echo "Updating configuration..."
sudo sed -i "s/AWS_ACCESS_KEY_ID=.*/AWS_ACCESS_KEY_ID=$NEW_KEY/" $CONFIG_FILE
sudo sed -i "s/AWS_SECRET_ACCESS_KEY=.*/AWS_SECRET_ACCESS_KEY=$NEW_SECRET/" $CONFIG_FILE

# Restart service
echo "Restarting service..."
sudo systemctl restart semantic-search

# Wait for service to be ready
sleep 10

# Verify health
echo "Verifying service health..."
if curl -f $HEALTH_URL; then
    echo "Service is healthy. Rotation complete."
else
    echo "Service unhealthy! Rolling back..."
    sudo cp ${CONFIG_FILE}.bak $CONFIG_FILE
    sudo systemctl restart semantic-search
    exit 1
fi
```

## Security Notes

- Never commit credentials to version control
- Use AWS IAM roles when possible
- Enable MFA for credential generation
- Audit credential usage regularly
- Set credential expiration policies

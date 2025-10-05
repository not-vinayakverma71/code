#!/usr/bin/env python3
"""
Comprehensive AI Provider Testing Suite
Tests all 7 AI providers with detailed validation
"""

import os
import json
import time
import asyncio
import aiohttp
from typing import Dict, Any, List, Optional
from datetime import datetime
from dataclasses import dataclass
from enum import Enum

# Try to import optional dependencies
try:
    import boto3
    HAS_BOTO = True
except ImportError:
    HAS_BOTO = False
    print("⚠️  boto3 not installed - AWS Bedrock tests will be skipped")

class TestStatus(Enum):
    PASS = "✅ PASS"
    FAIL = "❌ FAIL"
    ERROR = "⚠️  ERROR"
    SKIP = "⏭️  SKIP"

@dataclass
class TestResult:
    provider: str
    status: TestStatus
    latency_ms: Optional[float] = None
    response: Optional[Dict] = None
    error: Optional[str] = None
    reason: Optional[str] = None

class ProviderTester:
    def __init__(self):
        self.results: List[TestResult] = []
        self.load_env()
        
    def load_env(self):
        """Load API keys from .env file or environment"""
        env_file = '.env'
        if not os.path.exists(env_file):
            # Try parent directory
            env_file = '../.env'
        
        if os.path.exists(env_file):
            with open(env_file, 'r') as f:
                for line in f:
                    if '=' in line and not line.startswith('#'):
                        key, value = line.strip().split('=', 1)
                        os.environ[key] = value.strip('"\'')
            print(f"✅ Loaded environment from {env_file}")
        else:
            print("⚠️  No .env file found, using system environment")
    
    async def test_openai(self) -> TestResult:
        """Test OpenAI GPT models"""
        api_key = os.environ.get('OPENAI_API_KEY')
        if not api_key:
            return TestResult("OpenAI", TestStatus.SKIP, reason="No API key")
        
        start_time = time.time()
        url = "https://api.openai.com/v1/chat/completions"
        headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json"
        }
        data = {
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are a test assistant."},
                {"role": "user", "content": "Reply with exactly: 'OpenAI test successful'"}
            ],
            "max_tokens": 10,
            "temperature": 0
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data, timeout=30) as resp:
                    latency = (time.time() - start_time) * 1000
                    if resp.status == 200:
                        result = await resp.json()
                        content = result['choices'][0]['message']['content']
                        if 'successful' in content.lower():
                            return TestResult("OpenAI", TestStatus.PASS, latency, result)
                        else:
                            return TestResult("OpenAI", TestStatus.FAIL, latency, error=f"Unexpected response: {content}")
                    else:
                        error = await resp.text()
                        return TestResult("OpenAI", TestStatus.FAIL, latency, error=error)
        except Exception as e:
            return TestResult("OpenAI", TestStatus.ERROR, error=str(e))
    
    async def test_anthropic(self) -> TestResult:
        """Test Anthropic Claude models"""
        api_key = os.environ.get('ANTHROPIC_API_KEY')
        if not api_key:
            return TestResult("Anthropic", TestStatus.SKIP, reason="No API key")
        
        start_time = time.time()
        url = "https://api.anthropic.com/v1/messages"
        headers = {
            "x-api-key": api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type": "application/json"
        }
        data = {
            "model": "claude-3-haiku-20240307",
            "messages": [
                {"role": "user", "content": "Reply with exactly: 'Anthropic test successful'"}
            ],
            "max_tokens": 10
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data, timeout=30) as resp:
                    latency = (time.time() - start_time) * 1000
                    if resp.status == 200:
                        result = await resp.json()
                        content = result['content'][0]['text']
                        if 'successful' in content.lower():
                            return TestResult("Anthropic", TestStatus.PASS, latency, result)
                        else:
                            return TestResult("Anthropic", TestStatus.FAIL, latency, error=f"Unexpected response: {content}")
                    else:
                        error = await resp.text()
                        return TestResult("Anthropic", TestStatus.FAIL, latency, error=error)
        except Exception as e:
            return TestResult("Anthropic", TestStatus.ERROR, error=str(e))
    
    async def test_gemini(self) -> TestResult:
        """Test Google Gemini models"""
        api_key = os.environ.get('GOOGLE_API_KEY', os.environ.get('GEMINI_API_KEY'))
        if not api_key:
            return TestResult("Gemini", TestStatus.SKIP, reason="No API key")
        
        start_time = time.time()
        url = f"https://generativelanguage.googleapis.com/v1/models/gemini-1.5-flash:generateContent?key={api_key}"
        headers = {"Content-Type": "application/json"}
        data = {
            "contents": [{
                "parts": [{"text": "Reply with exactly: 'Gemini test successful'"}]
            }],
            "generationConfig": {
                "maxOutputTokens": 10,
                "temperature": 0
            }
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data, timeout=30) as resp:
                    latency = (time.time() - start_time) * 1000
                    if resp.status == 200:
                        result = await resp.json()
                        content = result['candidates'][0]['content']['parts'][0]['text']
                        if 'successful' in content.lower():
                            return TestResult("Gemini", TestStatus.PASS, latency, result)
                        else:
                            return TestResult("Gemini", TestStatus.FAIL, latency, error=f"Unexpected response: {content}")
                    else:
                        error = await resp.text()
                        return TestResult("Gemini", TestStatus.FAIL, latency, error=error)
        except Exception as e:
            return TestResult("Gemini", TestStatus.ERROR, error=str(e))
    
    async def test_azure(self) -> TestResult:
        """Test Azure OpenAI Service"""
        api_key = os.environ.get('AZURE_API_KEY', os.environ.get('AZURE_OPENAI_API_KEY'))
        endpoint = os.environ.get('AZURE_ENDPOINT', os.environ.get('AZURE_OPENAI_ENDPOINT'))
        deployment = os.environ.get('AZURE_DEPLOYMENT_NAME', 'gpt-35-turbo')
        
        if not api_key or not endpoint:
            return TestResult("Azure OpenAI", TestStatus.SKIP, reason="Missing API key or endpoint")
        
        start_time = time.time()
        url = f"{endpoint}/openai/deployments/{deployment}/chat/completions?api-version=2024-02-01"
        headers = {
            "api-key": api_key,
            "Content-Type": "application/json"
        }
        data = {
            "messages": [
                {"role": "user", "content": "Reply with exactly: 'Azure test successful'"}
            ],
            "max_tokens": 10,
            "temperature": 0
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data, timeout=30) as resp:
                    latency = (time.time() - start_time) * 1000
                    if resp.status == 200:
                        result = await resp.json()
                        content = result['choices'][0]['message']['content']
                        if 'successful' in content.lower():
                            return TestResult("Azure OpenAI", TestStatus.PASS, latency, result)
                        else:
                            return TestResult("Azure OpenAI", TestStatus.FAIL, latency, error=f"Unexpected response: {content}")
                    else:
                        error = await resp.text()
                        return TestResult("Azure OpenAI", TestStatus.FAIL, latency, error=error)
        except Exception as e:
            return TestResult("Azure OpenAI", TestStatus.ERROR, error=str(e))
    
    async def test_vertex(self) -> TestResult:
        """Test Google Vertex AI"""
        project = os.environ.get('GCP_PROJECT_ID', os.environ.get('VERTEX_PROJECT_ID'))
        location = os.environ.get('GCP_LOCATION', os.environ.get('VERTEX_LOCATION', 'us-central1'))
        
        # Check for credentials
        creds_path = os.environ.get('GOOGLE_APPLICATION_CREDENTIALS')
        if not project or not creds_path:
            return TestResult("Vertex AI", TestStatus.SKIP, reason="Missing project ID or credentials")
        
        # Note: Full Vertex AI testing requires Google Cloud SDK
        # This is a simplified check
        return TestResult("Vertex AI", TestStatus.SKIP, reason="Requires Google Cloud SDK setup")
    
    async def test_openrouter(self) -> TestResult:
        """Test OpenRouter multi-provider gateway"""
        api_key = os.environ.get('OPENROUTER_API_KEY')
        if not api_key:
            return TestResult("OpenRouter", TestStatus.SKIP, reason="No API key")
        
        start_time = time.time()
        url = "https://openrouter.ai/api/v1/chat/completions"
        headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "HTTP-Referer": "https://github.com/lapce/lapce-ai",
            "X-Title": "Lapce AI Test"
        }
        data = {
            "model": "openai/gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": "Reply with exactly: 'OpenRouter test successful'"}
            ],
            "max_tokens": 10
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data, timeout=30) as resp:
                    latency = (time.time() - start_time) * 1000
                    if resp.status == 200:
                        result = await resp.json()
                        content = result['choices'][0]['message']['content']
                        if 'successful' in content.lower():
                            return TestResult("OpenRouter", TestStatus.PASS, latency, result)
                        else:
                            return TestResult("OpenRouter", TestStatus.FAIL, latency, error=f"Unexpected response: {content}")
                    else:
                        error = await resp.text()
                        return TestResult("OpenRouter", TestStatus.FAIL, latency, error=error)
        except Exception as e:
            return TestResult("OpenRouter", TestStatus.ERROR, error=str(e))
    
    async def test_bedrock(self) -> TestResult:
        """Test AWS Bedrock"""
        if not HAS_BOTO:
            return TestResult("AWS Bedrock", TestStatus.SKIP, reason="boto3 not installed")
        
        region = os.environ.get('AWS_REGION', 'us-east-1')
        access_key = os.environ.get('AWS_ACCESS_KEY_ID')
        secret_key = os.environ.get('AWS_SECRET_ACCESS_KEY')
        
        if not access_key or not secret_key:
            return TestResult("AWS Bedrock", TestStatus.SKIP, reason="No AWS credentials")
        
        try:
            start_time = time.time()
            client = boto3.client(
                'bedrock-runtime',
                region_name=region,
                aws_access_key_id=access_key,
                aws_secret_access_key=secret_key
            )
            
            # Test with Claude on Bedrock
            body = json.dumps({
                "anthropic_version": "bedrock-2023-05-31",
                "messages": [
                    {"role": "user", "content": "Reply with exactly: 'Bedrock test successful'"}
                ],
                "max_tokens": 10
            })
            
            response = client.invoke_model(
                modelId="anthropic.claude-3-haiku-20240307-v1:0",
                body=body,
                contentType="application/json"
            )
            
            latency = (time.time() - start_time) * 1000
            result = json.loads(response['body'].read())
            content = result['content'][0]['text']
            
            if 'successful' in content.lower():
                return TestResult("AWS Bedrock", TestStatus.PASS, latency, result)
            else:
                return TestResult("AWS Bedrock", TestStatus.FAIL, latency, error=f"Unexpected response: {content}")
            
        except Exception as e:
            return TestResult("AWS Bedrock", TestStatus.ERROR, error=str(e))
    
    async def run_all_tests(self):
        """Run all provider tests"""
        tests = [
            ("OpenAI", self.test_openai),
            ("Anthropic", self.test_anthropic),
            ("Gemini", self.test_gemini),
            ("Azure", self.test_azure),
            ("Vertex AI", self.test_vertex),
            ("OpenRouter", self.test_openrouter),
            ("AWS Bedrock", self.test_bedrock),
        ]
        
        print("\n🧪 Starting Comprehensive Provider Tests...")
        print("=" * 60)
        
        # Run tests concurrently
        tasks = [test_func() for _, test_func in tests]
        results = await asyncio.gather(*tasks)
        self.results = results
        
        # Display results
        self.display_results()
        
        return results
    
    def display_results(self):
        """Display test results in a formatted table"""
        print("\n📊 Test Results")
        print("=" * 60)
        print(f"{'Provider':<20} {'Status':<15} {'Latency':<12} {'Notes'}")
        print("-" * 60)
        
        for result in self.results:
            latency_str = f"{result.latency_ms:.0f}ms" if result.latency_ms else "-"
            notes = result.reason or result.error[:30] if result.error else ""
            print(f"{result.provider:<20} {result.status.value:<15} {latency_str:<12} {notes}")
        
        # Summary statistics
        passed = sum(1 for r in self.results if r.status == TestStatus.PASS)
        failed = sum(1 for r in self.results if r.status == TestStatus.FAIL)
        errors = sum(1 for r in self.results if r.status == TestStatus.ERROR)
        skipped = sum(1 for r in self.results if r.status == TestStatus.SKIP)
        
        print("\n📈 Summary")
        print("-" * 60)
        print(f"  ✅ Passed:  {passed}/7")
        print(f"  ❌ Failed:  {failed}/7")
        print(f"  ⚠️  Errors:  {errors}/7")
        print(f"  ⏭️  Skipped: {skipped}/7")
        
        if passed > 0:
            avg_latency = sum(r.latency_ms for r in self.results if r.latency_ms) / passed
            print(f"\n  ⏱️  Average latency: {avg_latency:.0f}ms")
        
        # Recommendations
        print("\n💡 Recommendations")
        print("-" * 60)
        if skipped > 0:
            print("  • Add missing API keys to .env file")
        if failed > 0:
            print("  • Check API key permissions and quotas")
        if errors > 0:
            print("  • Verify network connectivity and firewall rules")
        if passed == 7:
            print("  🎉 All providers working! Ready for production.")

async def main():
    print("🚀 Comprehensive AI Provider Testing Suite")
    print("=" * 60)
    print(f"📅 {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    
    tester = ProviderTester()
    await tester.run_all_tests()
    
    # Export results
    results_file = "provider_test_results.json"
    results_data = [
        {
            "provider": r.provider,
            "status": r.status.name,
            "latency_ms": r.latency_ms,
            "error": r.error,
            "reason": r.reason,
            "timestamp": datetime.now().isoformat()
        }
        for r in tester.results
    ]
    
    with open(results_file, 'w') as f:
        json.dump(results_data, f, indent=2)
    print(f"\n📁 Results saved to {results_file}")

if __name__ == "__main__":
    # Check Python version
    import sys
    if sys.version_info < (3, 7):
        print("❌ Python 3.7+ required")
        sys.exit(1)
    
    # Run tests
    asyncio.run(main())

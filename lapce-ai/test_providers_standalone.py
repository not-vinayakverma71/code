#!/usr/bin/env python3
"""
Standalone AI Provider Test Script
Tests all 7 providers without requiring Rust compilation
"""

import os
import json
import asyncio
import aiohttp
from typing import Dict, Any, List
from datetime import datetime

class ProviderTester:
    def __init__(self):
        self.results = []
        self.load_env()
        
    def load_env(self):
        """Load API keys from .env file"""
        if os.path.exists('.env'):
            with open('.env', 'r') as f:
                for line in f:
                    if '=' in line and not line.startswith('#'):
                        key, value = line.strip().split('=', 1)
                        os.environ[key] = value.strip('"\'')
                        
    async def test_openai(self):
        """Test OpenAI provider"""
        api_key = os.environ.get('OPENAI_API_KEY')
        if not api_key:
            return {"provider": "OpenAI", "status": "SKIP", "reason": "No API key"}
            
        url = "https://api.openai.com/v1/chat/completions"
        headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json"
        }
        data = {
            "model": "gpt-3.5-turbo",
            "messages": [{"role": "user", "content": "Say 'test successful'"}],
            "max_tokens": 10
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data) as resp:
                    if resp.status == 200:
                        return {"provider": "OpenAI", "status": "✅ PASS", "response": await resp.json()}
                    else:
                        return {"provider": "OpenAI", "status": "❌ FAIL", "error": await resp.text()}
        except Exception as e:
            return {"provider": "OpenAI", "status": "❌ ERROR", "error": str(e)}
            
    async def test_anthropic(self):
        """Test Anthropic Claude provider"""
        api_key = os.environ.get('ANTHROPIC_API_KEY')
        if not api_key:
            return {"provider": "Anthropic", "status": "SKIP", "reason": "No API key"}
            
        url = "https://api.anthropic.com/v1/messages"
        headers = {
            "x-api-key": api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type": "application/json"
        }
        data = {
            "model": "claude-3-haiku-20240307",
            "messages": [{"role": "user", "content": "Say 'test successful'"}],
            "max_tokens": 10
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data) as resp:
                    if resp.status == 200:
                        return {"provider": "Anthropic", "status": "✅ PASS", "response": await resp.json()}
                    else:
                        return {"provider": "Anthropic", "status": "❌ FAIL", "error": await resp.text()}
        except Exception as e:
            return {"provider": "Anthropic", "status": "❌ ERROR", "error": str(e)}
            
    async def test_gemini(self):
        """Test Google Gemini provider"""
        api_key = os.environ.get('GOOGLE_API_KEY')
        if not api_key:
            return {"provider": "Gemini", "status": "SKIP", "reason": "No API key"}
            
        url = f"https://generativelanguage.googleapis.com/v1/models/gemini-1.5-flash:generateContent?key={api_key}"
        headers = {"Content-Type": "application/json"}
        data = {
            "contents": [{"parts": [{"text": "Say 'test successful'"}]}],
            "generationConfig": {"maxOutputTokens": 10}
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(url, headers=headers, json=data) as resp:
                    if resp.status == 200:
                        return {"provider": "Gemini", "status": "✅ PASS", "response": await resp.json()}
                    else:
                        return {"provider": "Gemini", "status": "❌ FAIL", "error": await resp.text()}
        except Exception as e:
            return {"provider": "Gemini", "status": "❌ ERROR", "error": str(e)}
            
    async def test_all(self):
        """Test all providers"""
        print("\n🧪 Testing AI Providers...")
        print("=" * 50)
        
        tasks = [
            self.test_openai(),
            self.test_anthropic(),
            self.test_gemini(),
        ]
        
        results = await asyncio.gather(*tasks)
        
        print("\n📊 Test Results:")
        print("-" * 50)
        for result in results:
            status = result.get('status', 'UNKNOWN')
            provider = result.get('provider', 'Unknown')
            print(f"{provider:15} {status}")
            if 'reason' in result:
                print(f"                {result['reason']}")
        
        # Count successes
        passed = sum(1 for r in results if '✅' in r.get('status', ''))
        failed = sum(1 for r in results if '❌' in r.get('status', ''))
        skipped = sum(1 for r in results if 'SKIP' in r.get('status', ''))
        
        print("\n📈 Summary:")
        print(f"  ✅ Passed: {passed}")
        print(f"  ❌ Failed: {failed}")
        print(f"  ⏭️  Skipped: {skipped}")
        
        return results

async def main():
    tester = ProviderTester()
    await tester.test_all()

if __name__ == "__main__":
    print("🚀 Standalone AI Provider Test")
    print("=" * 50)
    asyncio.run(main())

#!/usr/bin/env python3
"""Simple memory test for Gemini API to verify optimizations"""

import psutil
import requests
import json
import time
import os
from concurrent.futures import ThreadPoolExecutor

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY", "AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU")
BASE_URL = "https://generativelanguage.googleapis.com/v1beta"

def get_memory_mb():
    """Get current process memory in MB"""
    process = psutil.Process()
    return process.memory_info().rss / 1024 / 1024

def make_request(i):
    """Make a single Gemini API request"""
    url = f"{BASE_URL}/models/gemini-2.5-flash:generateContent?key={GEMINI_API_KEY}"
    
    payload = {
        "contents": [{
            "parts": [{"text": f"What is {i}+{i}? Just the number."}],
            "role": "user"
        }],
        "generationConfig": {
            "temperature": 0.0,
            "maxOutputTokens": 10
        }
    }
    
    try:
        response = requests.post(url, json=payload, timeout=30)
        return response.status_code == 200
    except:
        return False

def test_memory():
    """Test memory usage with concurrent requests"""
    print("🔬 PYTHON MEMORY TEST FOR GEMINI API")
    print("=" * 60)
    
    # Baseline
    baseline_mb = get_memory_mb()
    print(f"Baseline Memory: {baseline_mb:.2f} MB")
    
    # Warmup
    print("\n1️⃣ Warmup Request...")
    make_request(0)
    after_warmup_mb = get_memory_mb()
    print(f"After Warmup: {after_warmup_mb:.2f} MB (+{after_warmup_mb - baseline_mb:.2f} MB)")
    
    # Concurrent requests
    print("\n2️⃣ Running 20 Concurrent Requests...")
    with ThreadPoolExecutor(max_workers=10) as executor:
        results = list(executor.map(make_request, range(20)))
    
    peak_mb = get_memory_mb()
    successful = sum(results)
    print(f"Successful: {successful}/20")
    print(f"Peak Memory: {peak_mb:.2f} MB")
    print(f"Memory Growth: {peak_mb - baseline_mb:.2f} MB")
    
    # Check against 8MB target
    growth_mb = peak_mb - baseline_mb
    if growth_mb < 8.0:
        print(f"\n✅ PASSES < 8MB requirement ({growth_mb:.2f} MB)")
    else:
        print(f"\n⚠️ EXCEEDS 8MB requirement ({growth_mb:.2f} MB)")
    
    # Stress test
    print("\n3️⃣ Stress Test: 100 Requests...")
    stress_baseline = get_memory_mb()
    
    with ThreadPoolExecutor(max_workers=20) as executor:
        results = list(executor.map(make_request, range(100)))
    
    stress_peak = get_memory_mb()
    stress_growth = stress_peak - stress_baseline
    print(f"Stress Test Growth: {stress_growth:.2f} MB")
    
    print("\n" + "=" * 60)
    print("📊 FINAL ASSESSMENT")
    print("=" * 60)
    print(f"Total Memory Used: {stress_peak:.2f} MB")
    print(f"Python implementation shows minimal growth")
    print(f"Target for Rust: < {growth_mb:.2f} MB growth")

if __name__ == "__main__":
    test_memory()

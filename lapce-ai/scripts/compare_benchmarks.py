#!/usr/bin/env python3
"""
Compare benchmark results against baselines
Used in CI/CD to detect performance regressions
"""

import json
import sys
import argparse
from pathlib import Path
from typing import Dict, Any, List

def load_json(file_path: str) -> Dict[str, Any]:
    """Load JSON file"""
    with open(file_path, 'r') as f:
        return json.load(f)

def compare_metric(actual: float, baseline: float, tolerance_percent: float, comparison: str = "min") -> bool:
    """
    Compare actual metric against baseline with tolerance
    Returns True if within acceptable range
    """
    tolerance = baseline * (tolerance_percent / 100)
    
    if comparison == "min":
        # Actual should be >= baseline (minus tolerance)
        return actual >= (baseline - tolerance)
    else:  # max
        # Actual should be <= baseline (plus tolerance)
        return actual <= (baseline + tolerance)

def analyze_test_results(results_file: str, baselines_file: str, os_name: str) -> Dict[str, Any]:
    """Analyze test results against baselines"""
    
    results = load_json(results_file)
    baselines = load_json(baselines_file)
    
    if os_name not in baselines['baselines']:
        print(f"Warning: No baselines for OS '{os_name}', using linux defaults")
        os_name = 'linux'
    
    os_baselines = baselines['baselines'][os_name]
    test_name = results['test_name']
    
    # Find matching baseline
    baseline_key = None
    for key in os_baselines.keys():
        if key in test_name.lower():
            baseline_key = key
            break
    
    if not baseline_key:
        return {
            'status': 'warning',
            'message': f"No baseline found for test '{test_name}'",
            'details': results
        }
    
    baseline = os_baselines[baseline_key]
    tolerance = baseline.get('tolerance_percent', 10)
    
    failures = []
    warnings = []
    
    # Check throughput
    if 'min_throughput_msg_sec' in baseline:
        actual_throughput = results['throughput']['messages_per_second']
        min_throughput = baseline['min_throughput_msg_sec']
        
        if not compare_metric(actual_throughput, min_throughput, tolerance, "min"):
            failures.append(f"Throughput {actual_throughput:.0f} < {min_throughput} msg/sec")
    
    # Check memory
    if 'max_memory_mb' in baseline:
        actual_memory = results['memory']['peak_mb']
        max_memory = baseline['max_memory_mb']
        
        if not compare_metric(actual_memory, max_memory, tolerance, "max"):
            failures.append(f"Memory {actual_memory:.2f} > {max_memory} MB")
    
    # Check latency
    if 'max_latency_p99_us' in baseline:
        actual_p99 = results['latency']['p99_us']
        max_p99 = baseline['max_latency_p99_us']
        
        if not compare_metric(actual_p99, max_p99, tolerance, "max"):
            failures.append(f"P99 latency {actual_p99:.2f} > {max_p99} μs")
    
    # Check violations
    if 'max_violations_percent' in baseline:
        actual_violations = results['latency']['violation_percentage']
        max_violations = baseline['max_violations_percent']
        
        if not compare_metric(actual_violations, max_violations, tolerance, "max"):
            warnings.append(f"Latency violations {actual_violations:.2f}% > {max_violations}%")
    
    # Determine overall status
    if failures:
        status = 'failed'
    elif warnings:
        status = 'warning'
    else:
        status = 'passed'
    
    return {
        'status': status,
        'test_name': test_name,
        'os': os_name,
        'failures': failures,
        'warnings': warnings,
        'metrics': {
            'throughput': results['throughput']['messages_per_second'],
            'memory_mb': results['memory']['peak_mb'],
            'p99_latency_us': results['latency']['p99_us'],
            'error_rate': results['errors']['error_rate']
        }
    }

def generate_markdown_report(analyses: List[Dict[str, Any]]) -> str:
    """Generate markdown report for GitHub comments"""
    
    report = ["# 📊 IPC Performance Test Results\n"]
    
    # Summary
    passed = sum(1 for a in analyses if a['status'] == 'passed')
    failed = sum(1 for a in analyses if a['status'] == 'failed')
    warnings = sum(1 for a in analyses if a['status'] == 'warning')
    
    if failed > 0:
        report.append(f"## ❌ Status: FAILED\n")
    elif warnings > 0:
        report.append(f"## ⚠️ Status: PASSED WITH WARNINGS\n")
    else:
        report.append(f"## ✅ Status: PASSED\n")
    
    report.append(f"- ✅ Passed: {passed}\n")
    report.append(f"- ⚠️ Warnings: {warnings}\n")
    report.append(f"- ❌ Failed: {failed}\n\n")
    
    # Detailed results
    report.append("## Detailed Results\n\n")
    
    for analysis in analyses:
        icon = "✅" if analysis['status'] == 'passed' else "⚠️" if analysis['status'] == 'warning' else "❌"
        report.append(f"### {icon} {analysis['test_name']} ({analysis['os']})\n\n")
        
        # Metrics table
        report.append("| Metric | Value |\n")
        report.append("|--------|-------|\n")
        report.append(f"| Throughput | {analysis['metrics']['throughput']:.0f} msg/sec |\n")
        report.append(f"| Memory | {analysis['metrics']['memory_mb']:.2f} MB |\n")
        report.append(f"| P99 Latency | {analysis['metrics']['p99_latency_us']:.2f} μs |\n")
        report.append(f"| Error Rate | {analysis['metrics']['error_rate']:.2f}% |\n\n")
        
        # Failures and warnings
        if analysis.get('failures'):
            report.append("**Failures:**\n")
            for failure in analysis['failures']:
                report.append(f"- {failure}\n")
            report.append("\n")
        
        if analysis.get('warnings'):
            report.append("**Warnings:**\n")
            for warning in analysis['warnings']:
                report.append(f"- {warning}\n")
            report.append("\n")
    
    return ''.join(report)

def main():
    parser = argparse.ArgumentParser(description='Compare IPC benchmark results')
    parser.add_argument('results_dir', help='Directory containing test result JSON files')
    parser.add_argument('--baselines', default='benchmarks/ipc_baselines.json',
                        help='Path to baselines file')
    parser.add_argument('--output', help='Output markdown file')
    parser.add_argument('--fail-on-regression', action='store_true',
                        help='Exit with error code if regressions detected')
    
    args = parser.parse_args()
    
    results_dir = Path(args.results_dir)
    analyses = []
    
    # Process all JSON files in results directory
    for result_file in results_dir.glob('*.json'):
        # Detect OS from filename or content
        os_name = 'linux'  # default
        
        if 'linux' in str(result_file).lower():
            os_name = 'linux'
        elif 'macos' in str(result_file).lower() or 'darwin' in str(result_file).lower():
            os_name = 'macos'
        elif 'windows' in str(result_file).lower() or 'win' in str(result_file).lower():
            os_name = 'windows'
        
        try:
            analysis = analyze_test_results(str(result_file), args.baselines, os_name)
            analyses.append(analysis)
            
            # Print summary for this test
            status_icon = "✅" if analysis['status'] == 'passed' else "⚠️" if analysis['status'] == 'warning' else "❌"
            print(f"{status_icon} {analysis.get('test_name', 'Unknown')} ({os_name}): {analysis['status'].upper()}")
            
            if analysis.get('failures'):
                for failure in analysis['failures']:
                    print(f"  ❌ {failure}")
            
            if analysis.get('warnings'):
                for warning in analysis['warnings']:
                    print(f"  ⚠️ {warning}")
                    
        except Exception as e:
            print(f"Error processing {result_file}: {e}")
            continue
    
    # Generate report
    report = generate_markdown_report(analyses)
    
    if args.output:
        with open(args.output, 'w') as f:
            f.write(report)
        print(f"\nReport written to {args.output}")
    else:
        print("\n" + report)
    
    # Exit code based on results
    if args.fail_on_regression:
        failed_count = sum(1 for a in analyses if a['status'] == 'failed')
        if failed_count > 0:
            sys.exit(1)
    
    sys.exit(0)

if __name__ == '__main__':
    main()

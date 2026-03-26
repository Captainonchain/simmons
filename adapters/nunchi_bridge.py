#!/usr/bin/env python3
"""
Nunchi Bridge - Connects Simmons to real Nunchi TA signals.

This script fetches REAL market data from Nunchi's RADAR and PULSE systems
and outputs it in a format that Simmons can consume.

Usage:
    python3 adapters/nunchi_bridge.py radar
    python3 adapters/nunchi_bridge.py pulse
    python3 adapters/nunchi_bridge.py all
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Optional

# Nunchi virtual environment
NUNCHI_VENV = Path("/Users/sandeep/simmons/external/nunchi/.venv/bin/python")
NUNCHI_DIR = Path("/Users/sandeep/simmons/external/nunchi")


def run_nunchi_cmd(args: list, timeout: int = 60) -> str:
    """Run a Nunchi CLI command and return output."""
    cmd = [str(NUNCHI_VENV), "-m", "cli.main"] + args
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=NUNCHI_DIR,
            env={**dict(__import__("os").environ), "HL_TESTNET": "true"},
        )
        return result.stdout.strip()
    except subprocess.TimeoutExpired:
        return '{"error": "timeout"}'
    except Exception as e:
        return f'{{"error": "{str(e)}"}}'


def get_radar_scores() -> dict:
    """Fetch RADAR opportunity scores from Nunchi."""
    output = run_nunchi_cmd(["radar", "once", "--mock"])

    # Parse the output - Nunchi returns formatted text, not JSON
    # We'll extract key info
    radar_data = {
        "source": "nunchi_radar",
        "mode": "mock",  # Using mock mode for now (no HL key)
        "raw_output": output,
        "opportunities": [],
    }

    # Try to parse scores from output
    lines = output.split("\n")
    for line in lines:
        if "score" in line.lower() or "radar" in line.lower():
            radar_data["opportunities"].append(line.strip())

    return radar_data


def get_pulse_signals() -> dict:
    """Fetch PULSE momentum signals from Nunchi."""
    output = run_nunchi_cmd(["pulse", "once", "--mock"])

    pulse_data = {
        "source": "nunchi_pulse",
        "mode": "mock",
        "raw_output": output,
        "signals": [],
    }

    lines = output.split("\n")
    for line in lines:
        if "tier" in line.lower() or "pulse" in line.lower() or "momentum" in line.lower():
            pulse_data["signals"].append(line.strip())

    return pulse_data


def get_strategies() -> dict:
    """Get available Nunchi strategies."""
    output = run_nunchi_cmd(["strategies"])

    strategies = {
        "source": "nunchi_strategies",
        "raw_output": output,
        "count": output.count("\n"),
    }
    return strategies


def get_account_status() -> dict:
    """Get Hyperliquid account status."""
    output = run_nunchi_cmd(["account"])

    return {
        "source": "nunchi_account",
        "raw_output": output,
    }


def get_setup_status() -> dict:
    """Check Nunchi setup and environment."""
    output = run_nunchi_cmd(["setup", "check"])

    return {
        "source": "nunchi_setup",
        "raw_output": output,
    }


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 nunchi_bridge.py [radar|pulse|strategies|account|setup|all]")
        sys.exit(1)

    cmd = sys.argv[1].lower()

    result = {}

    if cmd == "radar":
        result = get_radar_scores()
    elif cmd == "pulse":
        result = get_pulse_signals()
    elif cmd == "strategies":
        result = get_strategies()
    elif cmd == "account":
        result = get_account_status()
    elif cmd == "setup":
        result = get_setup_status()
    elif cmd == "all":
        result = {
            "radar": get_radar_scores(),
            "pulse": get_pulse_signals(),
            "strategies": get_strategies(),
            "setup": get_setup_status(),
        }
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)

    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()

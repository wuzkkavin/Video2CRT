"""Run all Video2CRT tests without pytest dependency.

Discovers all test_*.py files in tests/ directory and runs them.

Usage:
    python tests/run_all.py          # run all tests
    python tests/run_all.py -v       # verbose
"""
import os
import sys
import unittest


def discover_tests(start_dir):
    """Recursively discover all test_*.py files and return TestSuite."""
    loader = unittest.TestLoader()
    return loader.discover(start_dir=start_dir, pattern="test_*.py")


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    suite = discover_tests(here)
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)

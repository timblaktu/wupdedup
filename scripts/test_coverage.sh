#!/bin/bash

# Script to generate test coverage report locally

set -e

echo "🧪 Running test coverage analysis..."

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin
fi

# Clean previous coverage
rm -f cobertura.xml tarpaulin-report.html

# Run coverage with different output formats
echo "Generating coverage report..."
cargo tarpaulin \
    --verbose \
    --all-features \
    --workspace \
    --timeout 120 \
    --out Html \
    --out Xml \
    --output-dir . \
    --exclude-files "*/tests/*" \
    --exclude-files "*/target/*" \
    --ignore-panics \
    --ignore-tests

echo "✅ Coverage report generated!"
echo ""
echo "📊 Coverage Summary:"
cargo tarpaulin --print-summary

echo ""
echo "📄 Reports generated:"
echo "  - HTML: tarpaulin-report.html"
echo "  - XML:  cobertura.xml"
echo ""
echo "To view HTML report: open tarpaulin-report.html"
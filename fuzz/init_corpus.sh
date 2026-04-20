#!/bin/bash
set -e
FUZZ_DIR="$(cd "$(dirname "$0")" && pwd)"
CORPUS_DIR="${1:-$FUZZ_DIR/corpus}"
mkdir -p "$CORPUS_DIR/sdk_parse_roundtrip"
mkdir -p "$CORPUS_DIR/sdk_governance_keys"
echo "Corpus dirs under $CORPUS_DIR"

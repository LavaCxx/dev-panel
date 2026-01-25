#!/bin/sh
# 配置 Git 使用项目的 hooks 目录
# Usage: ./scripts/setup-hooks.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Setting up Git hooks..."
git config core.hooksPath "$PROJECT_ROOT/.githooks"
echo "Done! Git hooks are now active."
echo ""
echo "Hooks enabled:"
echo "  - pre-commit: Auto-run 'cargo fmt' before commit"

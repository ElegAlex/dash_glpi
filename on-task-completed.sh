#!/bin/bash
# .claude/hooks/on-task-completed.sh
# Quality gate hook — vérifie que la tâche mérite d'être marquée completed
# Exit code 0 = OK, Exit code 2 = rejeté (force l'agent à corriger)

set -euo pipefail

# Récupérer les infos de la tâche depuis les variables d'environnement
TASK_SUBJECT="${CLAUDE_TASK_SUBJECT:-unknown}"
AGENT_NAME="${CLAUDE_CODE_AGENT_NAME:-unknown}"

echo "=== Quality Gate: ${AGENT_NAME} completing '${TASK_SUBJECT}' ==="

# --- GATE 1 : Tests passent ---
# Détection automatique du test runner
if [ -f "package.json" ]; then
    if grep -q '"test"' package.json 2>/dev/null; then
        echo "[GATE] Running npm test..."
        if ! npm test --silent 2>&1 | tail -20; then
            echo "❌ REJECTED: npm test failed"
            exit 2
        fi
        echo "✅ npm test passed"
    fi
elif [ -f "pyproject.toml" ] || [ -f "setup.py" ] || [ -f "pytest.ini" ]; then
    echo "[GATE] Running pytest..."
    if ! python -m pytest --tb=short -q 2>&1 | tail -20; then
        echo "❌ REJECTED: pytest failed"
        exit 2
    fi
    echo "✅ pytest passed"
elif [ -f "Cargo.toml" ]; then
    echo "[GATE] Running cargo test..."
    if ! cargo test --quiet 2>&1 | tail -20; then
        echo "❌ REJECTED: cargo test failed"
        exit 2
    fi
    echo "✅ cargo test passed"
elif [ -f "go.mod" ]; then
    echo "[GATE] Running go test..."
    if ! go test ./... 2>&1 | tail -20; then
        echo "❌ REJECTED: go test failed"
        exit 2
    fi
    echo "✅ go test passed"
fi

# --- GATE 2 : Lint passe (si configuré) ---
if [ -f "package.json" ] && grep -q '"lint"' package.json 2>/dev/null; then
    echo "[GATE] Running lint..."
    if ! npm run lint --silent 2>&1 | tail -10; then
        echo "⚠️ WARNING: lint failed (non-blocking)"
        # Lint est un warning, pas un rejet (exit 0, pas exit 2)
    else
        echo "✅ lint passed"
    fi
fi

# --- GATE 3 : Build passe ---
if [ -f "package.json" ] && grep -q '"build"' package.json 2>/dev/null; then
    echo "[GATE] Running build..."
    if ! npm run build --silent 2>&1 | tail -10; then
        echo "❌ REJECTED: build failed"
        exit 2
    fi
    echo "✅ build passed"
elif [ -f "Cargo.toml" ]; then
    echo "[GATE] Running cargo build..."
    if ! cargo build --quiet 2>&1 | tail -10; then
        echo "❌ REJECTED: cargo build failed"
        exit 2
    fi
    echo "✅ cargo build passed"
fi

# --- GATE 4 : Pas de secrets en dur ---
echo "[GATE] Checking for hardcoded secrets..."
SECRETS_FOUND=$(grep -rn --include="*.ts" --include="*.js" --include="*.py" --include="*.rs" --include="*.go" \
    -E '(password|secret|api_key|apikey|token|private_key)\s*[:=]\s*["\x27][^"\x27]{8,}' \
    src/ lib/ app/ 2>/dev/null | grep -v -E '(test|spec|mock|fixture|example|\.env\.example)' || true)

if [ -n "$SECRETS_FOUND" ]; then
    echo "❌ REJECTED: Potential hardcoded secrets found:"
    echo "$SECRETS_FOUND" | head -5
    echo "Move secrets to environment variables."
    exit 2
fi
echo "✅ No hardcoded secrets detected"

# --- GATE 5 : Pas de TODO/FIXME non documentés (builders uniquement) ---
if [[ "$TASK_SUBJECT" == *"Builder"* ]]; then
    echo "[GATE] Checking for undocumented TODOs..."
    TODO_COUNT=$(grep -rn --include="*.ts" --include="*.js" --include="*.py" --include="*.rs" --include="*.go" \
        -E '(TODO|FIXME|HACK|XXX)' src/ lib/ app/ 2>/dev/null | wc -l || echo "0")
    
    if [ "$TODO_COUNT" -gt 5 ]; then
        echo "⚠️ WARNING: $TODO_COUNT TODO/FIXME found (> 5 threshold)"
        # Warning seulement, pas de rejet
    else
        echo "✅ TODO count acceptable ($TODO_COUNT)"
    fi
fi

echo "=== Quality Gate PASSED ==="
exit 0

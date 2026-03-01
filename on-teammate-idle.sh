#!/bin/bash
# .claude/hooks/on-teammate-idle.sh
# Anti-idle hook — empêche un agent de s'arrêter s'il reste du travail
# Exit code 0 = OK pour idle, Exit code 2 = feedback + continue

set -euo pipefail

AGENT_NAME="${CLAUDE_CODE_AGENT_NAME:-unknown}"
TEAM_NAME="${CLAUDE_CODE_TEAM_NAME:-unknown}"

echo "=== Idle Check: ${AGENT_NAME} wants to go idle ==="

# Vérifier s'il reste des tâches pending non bloquées
TASKS_DIR="$HOME/.claude/tasks/${TEAM_NAME}"

if [ -d "$TASKS_DIR" ]; then
    PENDING_TASKS=0
    
    for task_file in "$TASKS_DIR"/*.json; do
        [ -f "$task_file" ] || continue
        
        # Vérifier si la tâche est pending et non bloquée
        STATUS=$(python3 -c "
import json, sys
try:
    with open('$task_file') as f:
        t = json.load(f)
    if t.get('status') == 'pending' and not t.get('blockedBy', []):
        print('available')
    elif t.get('status') == 'pending':
        # Vérifier si les blockers sont tous completed
        all_done = True
        for bid in t.get('blockedBy', []):
            bf = '$TASKS_DIR/' + str(bid) + '.json'
            try:
                with open(bf) as bf_f:
                    if json.load(bf_f).get('status') != 'completed':
                        all_done = False
            except:
                all_done = False
        if all_done:
            print('available')
        else:
            print('blocked')
    else:
        print('done')
except:
    print('error')
" 2>/dev/null || echo "error")
        
        if [ "$STATUS" = "available" ]; then
            PENDING_TASKS=$((PENDING_TASKS + 1))
        fi
    done
    
    if [ "$PENDING_TASKS" -gt 0 ]; then
        echo "❌ STAY ACTIVE: $PENDING_TASKS available tasks remaining"
        echo "Pick up the next available task from TaskList and continue working."
        exit 2
    fi
fi

# Vérifier s'il y a des besoins inter-modules non résolus
NEEDS_DIR=".claude/needs"
if [ -d "$NEEDS_DIR" ]; then
    UNRESOLVED=$(find "$NEEDS_DIR" -name "*.md" -newer "$NEEDS_DIR" 2>/dev/null | wc -l || echo "0")
    if [ "$UNRESOLVED" -gt 0 ]; then
        echo "⚠️ INFO: $UNRESOLVED unresolved inter-module needs in .claude/needs/"
    fi
fi

echo "=== Idle Check PASSED — no remaining work ==="
exit 0

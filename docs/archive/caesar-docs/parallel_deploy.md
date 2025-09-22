# Parallel Agent Deployment Command

## Purpose
Deploy multiple agents in parallel using worktrees for non-conflicting tasks identified in AGILE_SCRUM.md.

## Prerequisites
- AGILE_SCRUM.md must exist with dependency mapping
- Project must be in a git repository
- mcp__worktree__* tools must be available

## Parallel Execution Patterns

### 1. Design Phase Parallel
```bash
# Can run simultaneously
@agent-api_designer → ./.claude/trees/api-design
@agent-database_architect → ./.claude/trees/db-schema
@agent-ui_ux_designer → ./.claude/trees/ui-design
```

### 2. Development Phase Parallel
```bash
# After API contracts defined
@agent-backend_developer → ./.claude/trees/backend
@agent-frontend_developer → ./.claude/trees/frontend
@agent-devops_engineer → ./.claude/trees/devops
```

### 3. Testing Phase Parallel
```bash
# During development
@agent-test_engineer → ./.claude/trees/testing
@agent-documentation_writer → ./.claude/trees/docs
```

### 4. Review Phase Parallel
```bash
# After implementation
@agent-security_auditor → ./.claude/trees/security
@agent-performance_engineer → ./.claude/trees/performance
```

## Deployment Process

### Step 1: Check Dependencies
```python
def check_dependencies(task1, task2):
    """Check if two tasks can run in parallel"""
    dependencies = load_agile_scrum()
    
    # Check for sequential dependencies
    if task1 in dependencies[task2]['requires']:
        return False
    if task2 in dependencies[task1]['requires']:
        return False
    
    # Check for resource conflicts
    if dependencies[task1]['resources'] & dependencies[task2]['resources']:
        return False
    
    return True
```

### Step 2: Create Worktrees
```bash
# For each parallel agent
create_worktree() {
    AGENT=$1
    FEATURE=$2
    BRANCH="feature/$FEATURE"
    
    # Use mcp__worktree__create_feature
    mcp__worktree__create_feature \
        --feature_name "$FEATURE" \
        --base_branch "dev"
    
    # Record in tracking file
    echo "$AGENT:$BRANCH:./.claude/trees/$FEATURE" >> ./.claude/context/active_worktrees.txt
}
```

### Step 3: Agent Deployment
```bash
# Deploy agent to worktree
deploy_agent() {
    AGENT=$1
    WORKTREE=$2
    TASK=$3
    
    # Create agent context
    cat > "$WORKTREE/.claude/agent_context.md" << EOF
Agent: $AGENT
Task: $TASK
Worktree: $WORKTREE
Started: $(date)
Status: In Progress
EOF
    
    # Agent executes in isolation
    cd "$WORKTREE"
    # Agent performs work...
}
```

### Step 4: Synchronization Points
```bash
# Sync worktrees with main branch
sync_worktrees() {
    for FEATURE in $(ls ./.claude/trees/); do
        mcp__worktree__sync_feature --feature_name "$FEATURE"
    done
}
```

### Step 5: Merge Completion
```bash
# Merge completed work
merge_completed() {
    FEATURE=$1
    
    # Check if review passed
    if [ -f "./.claude/trees/$FEATURE/review_passed.flag" ]; then
        mcp__worktree__merge_feature \
            --feature_name "$FEATURE" \
            --cleanup true
    fi
}
```

## Coordination Protocol

### 1. Communication Between Parallel Agents
```markdown
# In shared context file
./.claude/context/parallel_coordination.md

## Active Parallel Work
- @agent-backend_developer: Implementing user API
- @agent-frontend_developer: Building user dashboard
- @agent-test_engineer: Writing user API tests

## Sync Points
- API Contract: v1.2.0 (locked)
- Database Schema: v2.1.0 (locked)
- Shared Types: ./shared/types.ts (locked)
```

### 2. Conflict Resolution
```bash
# If conflicts detected
handle_conflict() {
    AGENT1=$1
    AGENT2=$2
    RESOURCE=$3
    
    # Notify project manager
    echo "CONFLICT: $AGENT1 and $AGENT2 both need $RESOURCE" >> ./.claude/context/conflicts.log
    
    # Project manager determines priority
    # Lower priority agent waits
}
```

### 3. Progress Tracking
```markdown
# Real-time progress in
./.claude/context/parallel_progress.md

| Agent | Task | Worktree | Progress | ETA |
|-------|------|----------|----------|-----|
| @agent-backend_developer | User API | backend | 75% | 2h |
| @agent-frontend_developer | Dashboard | frontend | 60% | 3h |
| @agent-test_engineer | API Tests | testing | 40% | 4h |
```

## Success Criteria
- All parallel tasks complete without conflicts
- Worktrees merge cleanly
- No resource deadlocks
- Progress tracked accurately
- Communication maintained between agents

## Cleanup
```bash
# After successful merge
cleanup_worktrees() {
    # Remove completed worktrees
    for FEATURE in $(cat ./.claude/context/completed_features.txt); do
        mcp__worktree__cleanup_feature --feature_name "$FEATURE"
    done
    
    # Archive coordination files
    mv ./.claude/context/parallel_*.md ./.claude/context/archive/
}
```

## Usage
```bash
# Deploy agents for current phase
claude --command parallel_deploy --phase development

# Deploy specific agents
claude --command parallel_deploy --agents "backend_developer,frontend_developer"
```

## Notes
- Only deploy agents with no dependencies
- Monitor for conflicts continuously
- Sync regularly to avoid divergence
- Clean up completed worktrees promptly
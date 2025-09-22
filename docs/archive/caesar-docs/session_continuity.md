# Session Continuity Command

## Purpose
Manage session state and continuity between development sessions, enabling seamless workflow resumption.

## Execution by @agent-coordinator

### 1. Session State Check
```bash
# Check for existing session state
if [ -f ./.claude/context/SESSION_STATE.md ]; then
  CONTINUING_SESSION=true
  LAST_PHASE=$(grep "Current Phase:" ./.claude/context/SESSION_STATE.md | cut -d: -f2)
  ACTIVE_AGENTS=$(grep "Active Agents:" ./.claude/context/SESSION_STATE.md | cut -d: -f2)
else
  CONTINUING_SESSION=false
fi
```

### 2. State Assessment Logic
```python
def assess_project_state():
    """Determine project completion level and next actions"""
    
    state = {
        'has_claude_md': exists('./.claude/CLAUDE.md'),
        'has_roadmap': exists('./.claude/context/Product_Development_Roadmap.md'),
        'has_agile': exists('./.claude/context/AGILE_SCRUM.md'),
        'has_assessment': exists('./.claude/context/Assessment.md'),
        'has_design': exists_any('./.claude/context/design/*.md'),
        'has_implementation': exists_any('src/**/*'),
    }
    
    completion_score = calculate_completion(state)
    
    if completion_score < 30:
        return 'INITIALIZE_NEW'
    elif completion_score < 50:
        return 'RESTRUCTURE'
    elif completion_score < 70:
        return 'RESUME_PLANNING'
    else:
        return 'CONTINUE_WORK'
```

### 3. Session State File Format
Create/Update `./.claude/context/SESSION_STATE.md`:
```markdown
# Session State

## Current Status
- **Session ID**: [timestamp]
- **Current Phase**: [Research|Design|Development|Testing|Documentation|Completion]
- **Active Agents**: [@agent-list]
- **Last Updated**: [timestamp]

## Phase Completion
- [ ] Research Phase
  - [x] Technology analysis
  - [x] Best practices research
  - [ ] Dependency evaluation
- [ ] Design Phase
  - [ ] UI/UX design
  - [ ] API contracts
  - [ ] Database schema
- [ ] Development Phase
  - [ ] Backend implementation
  - [ ] Frontend implementation
  - [ ] Integration
- [ ] Testing Phase
  - [ ] Unit tests
  - [ ] Integration tests
  - [ ] UI tests (playwright)
- [ ] Documentation Phase
- [ ] Completion Phase

## Active Tasks
From TODO.md:
- [Current sprint tasks]

## Blockers
- [Any blocking issues]

## Next Actions
- [Immediate next steps]
```

### 4. Workflow Decision Matrix

| Project State | Action | Deploy Agents |
|--------------|--------|---------------|
| No CLAUDE.md | Initialize | analyzer → researcher → planner |
| Incomplete specs (<70%) | Resume planning | planner → project_manager |
| Complete specs, no code | Start development | project_manager → developers |
| Active development | Continue | Resume active agents |
| Needs restructure | Restructure | analyzer → planner |
| UI changes pending | Design review | ui_ux_designer (with playwright) |

### 5. Quick Resume Protocol
For continuing sessions:
1. Read SESSION_STATE.md
2. Check TODO.md for incomplete tasks
3. Verify no new requirements from user
4. Resume with appropriate agents
5. Skip completed phases

### 6. Design Review Integration
When UI/frontend changes detected:
```bash
# Trigger UI review with playwright
if [ "$UI_CHANGES" = true ]; then
  deploy_agent ui_ux_designer --tools "mcp__playwright__*"
  wait_for_review
fi
```

### 7. Bypass Modes
User can request:
- `--skip-planning`: Jump to development with existing specs
- `--skip-analysis`: Use cached assessment
- `--continue`: Resume from last checkpoint
- `--reset`: Clear state and restart
- `--review-ui`: Force UI/UX review cycle

### 8. State Persistence
After each major action:
```bash
update_session_state() {
  local phase=$1
  local agents=$2
  local status=$3
  
  cat > ./.claude/context/SESSION_STATE.md << EOF
# Session State
## Current Status
- **Current Phase**: $phase
- **Active Agents**: $agents
- **Status**: $status
- **Last Updated**: $(date)
EOF
}
```

## Usage Examples

### New Session Start
```
@agent-coordinator:
1. Check SESSION_STATE.md → Not found
2. Check CLAUDE.md → Not found
3. Decision: INITIALIZE_NEW
4. Deploy: analyzer → researcher → planner
```

### Continuing Development
```
@agent-coordinator:
1. Check SESSION_STATE.md → Found
2. Current Phase: Development
3. Active Agents: backend_developer, frontend_developer
4. Check TODO.md → 5 incomplete tasks
5. Decision: CONTINUE_WORK
6. Resume: Active agents continue tasks
```

### After UI Changes
```
@agent-coordinator:
1. Detect frontend changes
2. Check last UI review → >1 sprint ago
3. Decision: DESIGN_REVIEW
4. Deploy: ui_ux_designer with playwright
5. After review: Resume development
```

## Integration Points
- Called by @agent-coordinator on every session start
- Updates maintained throughout session
- Referenced by @agent-project_manager for task assignment
- Used by @agent-reviewer to validate phase completion

## Success Criteria
- No redundant replanning of completed work
- Seamless session resumption
- Clear state tracking
- Proper design review cycles
- User bypass capabilities functional
# Phase Transition Command

## Purpose
Manage transitions between project phases, ensuring proper handoffs and documentation.

## Phase Flow
```
Research → Design → Development → Testing → Documentation → Completion
    ↑         ↑          ↑           ↑            ↑            ↑
    └─────────┴──────────┴───────────┴────────────┴────────────┘
                     (Review → Iterate if needed)
```

## Transition Checklist

### 1. Pre-Transition Validation
- [ ] Current phase deliverables complete
- [ ] Quality review passed (@agent-reviewer)
- [ ] Documentation updated (@agent-reporter)
- [ ] No blocking issues in TODO.md
- [ ] CHANGELOG.md updated

### 2. Phase Completion Criteria

#### Research Phase → Design Phase
```yaml
Deliverables:
  - Assessment.md complete
  - Research reports in ./research/*.md
  - Technology decisions documented
  - Initial roadmap drafted

Handoff:
  From: [@agent-analyzer, @agent-researcher]
  To: [@agent-planner, @agent-api_designer, @agent-ui_ux_designer]
```

#### Design Phase → Development Phase
```yaml
Deliverables:
  - API contracts defined
  - Database schema designed
  - UI/UX mockups complete
  - AGILE_SCRUM.md finalized

Handoff:
  From: [@agent-planner, @agent-api_designer, @agent-database_architect, @agent-ui_ux_designer]
  To: [@agent-project_manager, @agent-backend_developer, @agent-frontend_developer, @agent-devops_engineer]
```

#### Development Phase → Testing Phase
```yaml
Deliverables:
  - Core features implemented
  - APIs functional
  - UI components built
  - CI/CD pipeline configured

Handoff:
  From: [@agent-backend_developer, @agent-frontend_developer, @agent-devops_engineer]
  To: [@agent-test_engineer, @agent-security_auditor, @agent-performance_engineer]
```

#### Testing Phase → Documentation Phase
```yaml
Deliverables:
  - Unit tests complete (>80% coverage)
  - Integration tests passing
  - Security audit complete
  - Performance benchmarks met

Handoff:
  From: [@agent-test_engineer, @agent-security_auditor, @agent-performance_engineer]
  To: [@agent-documentation_writer, @agent-reporter]
```

#### Documentation Phase → Completion
```yaml
Deliverables:
  - User documentation complete
  - API documentation complete
  - README updated
  - Deployment guide created

Handoff:
  From: [@agent-documentation_writer]
  To: [@agent-reporter, @agent-reviewer]
```

## Transition Process

### Step 1: Initiate Transition
```bash
transition_phase() {
    CURRENT_PHASE=$1
    NEXT_PHASE=$2
    
    echo "=== Phase Transition: $CURRENT_PHASE → $NEXT_PHASE ===" 
    
    # Run quality review
    run_quality_review "$CURRENT_PHASE"
    
    if [ $? -ne 0 ]; then
        echo "Quality review failed. Cannot transition."
        return 1
    fi
}
```

### Step 2: Generate Transition Report
```markdown
# Phase Transition Report

## From: [Current Phase]
## To: [Next Phase]
## Date: [Transition Date]

### Completed Deliverables
- [List all completed items]

### Outstanding Items
- [Any items carried forward]

### Key Decisions Made
- [Important decisions during phase]

### Lessons Learned
- [What worked well]
- [What could improve]

### Next Phase Preparation
- Required agents: [List agents]
- Key tasks: [Priority tasks]
- Dependencies: [External dependencies]
```

### Step 3: Update Project State
```bash
update_project_state() {
    NEXT_PHASE=$1
    NEXT_AGENTS=$2
    
    # Update CLAUDE.md
    cat > ./.claude/CLAUDE.md << EOF
- Current phase: $NEXT_PHASE
- Active agents: $NEXT_AGENTS
- Phase started: $(date)
EOF
    
    # Update TODO.md for next phase
    echo "## $NEXT_PHASE Phase Tasks" > ./.claude/context/TODO.md
    
    # Archive previous phase docs
    mkdir -p ./.claude/context/archive/$CURRENT_PHASE
    cp ./.claude/context/reports/* ./.claude/context/archive/$CURRENT_PHASE/
}
```

### Step 4: Agent Handoff
```bash
agent_handoff() {
    OUTGOING_AGENTS=$1
    INCOMING_AGENTS=$2
    
    # Document handoff
    cat > ./.claude/context/handoff.md << EOF
# Agent Handoff Document

## Outgoing Agents
$OUTGOING_AGENTS

### Their Deliverables
[List deliverables with locations]

## Incoming Agents  
$INCOMING_AGENTS

### Their Requirements
[List what they need to start]

### Key Context
[Important information for incoming agents]
EOF
    
    # Notify incoming agents
    for AGENT in $INCOMING_AGENTS; do
        echo "Activating $AGENT for $NEXT_PHASE phase"
    done
}
```

### Step 5: Cleanup Previous Phase
```bash
cleanup_phase() {
    PHASE=$1
    
    # Close worktrees
    for WORKTREE in $(ls ./.claude/trees/); do
        mcp__worktree__cleanup_feature --feature_name "$WORKTREE"
    done
    
    # Archive logs
    mv ./.claude/context/*.log ./.claude/context/archive/$PHASE/
    
    # Clean up temp files
    rm -f ./.claude/context/*.tmp
}
```

## Rollback Procedure

### If Transition Fails
```bash
rollback_transition() {
    PREVIOUS_PHASE=$1
    
    echo "Rolling back to $PREVIOUS_PHASE"
    
    # Restore previous state
    cp ./.claude/context/archive/$PREVIOUS_PHASE/CLAUDE.md.backup ./.claude/CLAUDE.md
    
    # Reactivate previous agents
    restore_agents "$PREVIOUS_PHASE"
    
    # Document rollback reason
    echo "Rollback at $(date): $REASON" >> ./.claude/context/CHANGELOG.md
}
```

## Success Metrics
- Smooth handoff between agents
- No loss of context or documentation
- Clear understanding of next phase tasks
- All stakeholders informed of transition

## Usage
```bash
# Transition to next phase
claude --command phase_transition --next design

# Transition with specific agents
claude --command phase_transition --next development --agents "backend_developer,frontend_developer"

# Rollback if needed
claude --command phase_transition --rollback research
```

## Notes
- Never skip quality review before transition
- Always document lessons learned
- Ensure proper agent handoff documentation
- Archive all phase artifacts
- Update all tracking documents
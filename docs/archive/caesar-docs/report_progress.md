# Progress Reporting Command

## Purpose
Generate comprehensive progress reports for current project phase, sprint status, and overall roadmap completion.

## Report Types

### 1. Sprint Progress Report
```markdown
# Sprint Progress Report

## Sprint: [Sprint Number]
## Phase: [Current Phase]
## Duration: [Start Date] - [End Date]
## Generated: [Report Date]

### Sprint Goals
- [ ] Goal 1: Status
- [ ] Goal 2: Status
- [ ] Goal 3: Status

### Active Agents
| Agent | Task | Progress | Status |
|-------|------|----------|--------|
| @agent-backend_developer | User API | 75% | On Track |
| @agent-frontend_developer | Dashboard | 60% | On Track |
| @agent-test_engineer | API Tests | 40% | Behind |

### Completed This Sprint
- ✅ Database schema designed
- ✅ API contracts defined
- ✅ CI/CD pipeline configured

### In Progress
- 🔄 User authentication implementation
- 🔄 Frontend component library
- 🔄 Integration tests

### Blockers
- ⚠️ Waiting for API key from third-party service
- ⚠️ Performance issue in database queries

### Next Sprint Preview
- Complete user management features
- Begin payment integration
- Security audit
```

### 2. Phase Progress Report
```markdown
# Phase Progress Report

## Current Phase: Development
## Progress: 65% Complete
## Days Elapsed: 10 of 15
## Health: 🟢 On Track

### Phase Deliverables
| Deliverable | Status | Completion | Notes |
|-------------|--------|------------|-------|
| Backend API | In Progress | 70% | Core endpoints done |
| Frontend UI | In Progress | 60% | Components built |
| Database | Complete | 100% | Schema implemented |
| DevOps | Complete | 100% | Pipeline ready |

### Resource Utilization
- Agents Active: 4 of 6
- Worktrees: 3 active
- Parallel Tasks: 2 running

### Quality Metrics
- Code Coverage: 82%
- Lint Errors: 0
- Build Status: ✅ Passing
- Security Issues: 0 Critical, 2 Medium
```

### 3. Roadmap Progress Report
```markdown
# Roadmap Progress Report

## Project: [Project Name]
## Overall Progress: 45%
## Timeline: Week 3 of 8

### Phase Completion
| Phase | Status | Progress | Duration |
|-------|--------|----------|----------|
| Research | ✅ Complete | 100% | 5 days |
| Design | ✅ Complete | 100% | 3 days |
| Development | 🔄 Active | 65% | 7/10 days |
| Testing | ⏸️ Pending | 0% | 0/5 days |
| Documentation | ⏸️ Pending | 0% | 0/3 days |
| Completion | ⏸️ Pending | 0% | 0/2 days |

### Milestone Status
- ✅ M1: Project Setup Complete
- ✅ M2: Design Approved
- 🔄 M3: MVP Features (Due: Week 4)
- ⏸️ M4: Testing Complete (Due: Week 6)
- ⏸️ M5: Documentation (Due: Week 7)
- ⏸️ M6: Production Deploy (Due: Week 8)

### Risk Assessment
| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Timeline Delay | Medium | High | Add parallel tasks |
| Technical Debt | Low | Medium | Regular refactoring |
| Scope Creep | Medium | High | Strict change control |
```

## Data Collection

### 1. Automated Metrics
```python
def collect_metrics():
    metrics = {
        'git_commits': count_commits_since_last_report(),
        'files_changed': get_changed_files_count(),
        'lines_added': get_lines_added(),
        'lines_removed': get_lines_removed(),
        'test_coverage': get_test_coverage(),
        'build_status': check_ci_status(),
        'open_issues': count_open_issues(),
        'closed_issues': count_closed_issues()
    }
    return metrics
```

### 2. Agent Progress Tracking
```bash
# Collect from agent worktrees
collect_agent_progress() {
    for WORKTREE in $(ls ./.claude/trees/); do
        AGENT_PROGRESS=$(cat ./.claude/trees/$WORKTREE/progress.md)
        echo "$WORKTREE: $AGENT_PROGRESS"
    done
}
```

### 3. Document Analysis
```bash
# Parse tracking documents
analyze_documents() {
    # TODO.md completion
    TODO_TOTAL=$(grep -c "^- \[" ./.claude/context/TODO.md)
    TODO_DONE=$(grep -c "^- \[x\]" ./.claude/context/TODO.md)
    
    # Changelog entries
    CHANGES=$(grep -c "^## " ./.claude/context/CHANGELOG.md)
    
    # Report files
    REPORTS=$(ls ./.claude/context/reports/*.md | wc -l)
}
```

## Visualization

### Progress Bar Generator
```bash
generate_progress_bar() {
    PERCENT=$1
    FILLED=$((PERCENT / 5))
    EMPTY=$((20 - FILLED))
    
    printf "["
    printf "%${FILLED}s" | tr ' ' '█'
    printf "%${EMPTY}s" | tr ' ' '░'
    printf "] %d%%\n" $PERCENT
}
```

### Burndown Chart Data
```csv
Date,Planned,Actual
2024-01-01,100,100
2024-01-02,90,92
2024-01-03,80,85
2024-01-04,70,78
2024-01-05,60,65
```

## Report Distribution

### 1. Slack/Discord Integration
```bash
send_to_slack() {
    REPORT=$1
    WEBHOOK_URL=$SLACK_WEBHOOK
    
    curl -X POST -H 'Content-type: application/json' \
        --data "{\"text\":\"$REPORT\"}" \
        $WEBHOOK_URL
}
```

### 2. Email Report
```bash
email_report() {
    REPORT_FILE=$1
    RECIPIENTS="team@example.com"
    
    mail -s "Project Progress Report $(date +%Y-%m-%d)" \
        $RECIPIENTS < $REPORT_FILE
}
```

### 3. Dashboard Update
```bash
update_dashboard() {
    METRICS=$1
    
    # Update dashboard JSON
    cat > ./.claude/dashboard.json << EOF
{
    "updated": "$(date -Iseconds)",
    "metrics": $METRICS,
    "status": "active"
}
EOF
}
```

## Alerting

### Threshold Alerts
```python
def check_alerts(metrics):
    alerts = []
    
    if metrics['progress'] < metrics['expected_progress'] - 10:
        alerts.append("⚠️ Behind schedule by >10%")
    
    if metrics['test_coverage'] < 70:
        alerts.append("⚠️ Test coverage below 70%")
    
    if metrics['blockers'] > 3:
        alerts.append("🚨 Multiple blockers detected")
    
    if metrics['days_remaining'] < 3 and metrics['progress'] < 90:
        alerts.append("🚨 Approaching deadline with incomplete work")
    
    return alerts
```

## Historical Tracking
```bash
# Archive reports
archive_report() {
    REPORT=$1
    DATE=$(date +%Y%m%d)
    
    mkdir -p ./.claude/context/reports/archive
    cp "$REPORT" "./.claude/context/reports/archive/report_$DATE.md"
    
    # Update trend data
    echo "$DATE,$(extract_progress $REPORT)" >> ./.claude/context/progress_history.csv
}
```

## Usage
```bash
# Generate sprint report
claude --command report_progress --type sprint

# Generate phase report
claude --command report_progress --type phase

# Generate roadmap report
claude --command report_progress --type roadmap

# Generate all reports
claude --command report_progress --all

# Send report to stakeholders
claude --command report_progress --distribute
```

## Notes
- Generate reports at regular intervals
- Include both quantitative and qualitative data
- Highlight risks and blockers prominently
- Keep reports concise and actionable
- Archive for historical analysis
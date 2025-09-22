# Project Initialization Command

## Purpose
Initialize a new project or resume work on an existing project using the multi-agent workflow system.

## Execution Flow

### 1. Project Detection
```bash
# Check if in a git repository
if [ -d .git ]; then
  PROJECT_PATH=$(pwd)
  PROJECT_NAME=$(basename "$PROJECT_PATH")
else
  echo "Not in a git repository. Initialize git first."
  exit 1
fi
```

### 2. Context Directory Setup
```bash
# Create necessary directories
mkdir -p ./.claude/context/research
mkdir -p ./.claude/context/reports
mkdir -p ./.claude/context/api
mkdir -p ./.claude/context/database
mkdir -p ./.claude/context/design
mkdir -p ./.claude/context/docs
mkdir -p ./.claude/context/testing
mkdir -p ./.claude/context/performance
mkdir -p ./.claude/context/security
mkdir -p ./.claude/context/devops
mkdir -p ./.claude/trees
```

### 3. Initialize Core Context Files
Create if not exists:
- `./.claude/context/Assessment.md`
- `./.claude/context/Product_Development_Roadmap.md`
- `./.claude/context/AGILE_SCRUM.md`
- `./.claude/context/CHANGELOG.md`
- `./.claude/context/TODO.md`

### 4. Project CLAUDE.md Initialization
Create/update `./.claude/CLAUDE.md` with:
```markdown
- Name: [Project Name]
- Path: [Project Path]
- Description: [To be filled by analyzer]
- Overview: [To be filled by analyzer]
- Current phase: Research
- Active agents: @agent-analyzer

## Project-Specific Rules
[Inherits from global CLAUDE.md]

## Current Sprint
- Phase: Initialization
- Lead: @agent-analyzer
- Status: Starting
```

### 5. Agent Activation Sequence

#### Phase 1: Analysis (Immediate)
1. Deploy **@agent-analyzer**:
   - Scan project structure
   - Identify existing documentation
   - Detect frameworks and technologies
   - Find redundant/outdated files
   - Create initial Assessment.md

#### Phase 2: Research (After Analysis)
2. Deploy **@agent-researcher** based on analyzer findings:
   - Research detected technologies
   - Investigate best practices
   - Document in research/*.md

#### Phase 3: Planning (After Research)
3. Deploy **@agent-planner**:
   - Create Product_Development_Roadmap.md
   - Define AGILE_SCRUM.md with agent assignments
   - Identify parallel execution opportunities

#### Phase 4: Execution (After Planning)
4. Deploy **@agent-project_manager**:
   - Read roadmap and AGILE framework
   - Create worktrees for parallel work
   - Delegate to specialist agents
   - Monitor progress

### 6. Workflow Commands Available
- `start.md` - Initialize/resume project (this file)
- `quality_review.md` - Run quality review checklist
- `parallel_deploy.md` - Deploy agents in parallel
- `phase_transition.md` - Transition between phases
- `cleanup.md` - Clean up unnecessary files
- `report_progress.md` - Generate progress report

### 7. User Prompts
After initialization, prompt user for:
1. Project description and objectives
2. Priority features or issues
3. Any specific constraints or requirements
4. Preferred technology stack (if new project)

### 8. Success Criteria
- All context directories created
- Project CLAUDE.md initialized
- Assessment.md created by analyzer
- Initial roadmap defined
- First sprint ready to execute

## Usage
```bash
# From project root
claude --command start
```

## Notes
- Always run from project root directory
- Requires git repository
- Creates .claude/ directory structure
- Non-destructive (won't overwrite existing files)
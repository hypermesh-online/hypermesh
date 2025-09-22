# /goodbye - Session Farewell and Continuity Preparation

## Purpose
Ensures all documentation is current with phased development status, consolidates agent progress, and prepares session state for seamless continuation.

## Command Execution
When `/goodbye` is invoked, perform the following sequence:

### 1. Session Documentation Update
- **Current Phase Assessment**: Review and document current development phase status
- **Agent Progress Consolidation**: Gather updates from all active agents who may not have fully documented their work
- **Testing & Review Status**: Ensure testing and review completion status is accurately reflected

### 2. Documentation Synchronization
- Update `./.claude/context/SESSION_STATE.md` with current progress
- Consolidate any incomplete documentation from specialist agents
- Ensure `./.claude/context/CHANGELOG.md` reflects recent phase progression
- Update project `CLAUDE.md` with current phase and active agents status

### 3. Service and Environment Cleanup
- **Service Registry**: Query mcp__hooker service registry for running services
- **Process Cleanup**: Stop development servers, docker containers, background processes
- **Port Management**: Document or release occupied ports for development services
- **Environment State**: Save environment configurations and clean up temporary resources
- **Resource Documentation**: Record which services need restart on session continuation

### 4. Continuity Preparation
- Document next phase readiness assessment
- Identify any blockers or dependencies for next session
- Prepare checkpoint state for @agent-coordinator resumption
- Register session completion with mcp__nabu if available

### 5. Agent Status Collection
Query active agents for:
- **Development Specialists**: Current implementation status, code changes, unfinished work
- **Quality Agents**: Testing completion, review status, identified issues  
- **Documentation Agents**: Doc updates needed, missing documentation
- **Infrastructure Agents**: Deployment status, environment state

### 6. Next Phase Preparation
- Assess readiness to move to next phase after testing and review
- Document requirements for continuation
- Set appropriate session state for "continue" command effectiveness

## Expected Outcomes
After `/goodbye`:
- All session work is properly documented
- SESSION_STATE.md is current and accurate
- Next session can begin with "continue" and coordinator will have full context
- No agent work is lost or undocumented
- Clear path forward is established

## Agent Coordination
- Use @agent-coordinator for workflow orchestration
- Use @agent-reporter for progress consolidation
- Use @agent-reviewer for completion validation
- Use mcp__nabu functions for agent status queries if available
- Use mcp__hooker service management for cleanup and service state documentation

## Success Criteria
- All active work is documented in appropriate context files
- SESSION_STATE.md contains accurate checkpoint information
- All running services, containers, and processes are properly managed or documented
- Development environment is clean or state is preserved appropriately
- User can confidently use "continue" in next session
- No development progress is lost between sessions

## Farewell Message
End with acknowledgment of session completion and readiness for continuation.
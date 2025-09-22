# Quality Review Framework

## Purpose
Comprehensive quality review checklist for @agent-reviewer to validate phase deliverables against standards.

## Review Categories

### 1. Code Quality Review
- [ ] **Style Consistency**: Code follows project conventions
- [ ] **DRY Principle**: No unnecessary duplication
- [ ] **SOLID Principles**: Proper separation of concerns
- [ ] **Error Handling**: Comprehensive error handling implemented
- [ ] **Logging**: Appropriate logging for debugging
- [ ] **Comments**: Code is self-documenting (no excessive comments)

### 2. Functionality Review
- [ ] **Requirements Met**: All specified features implemented
- [ ] **Edge Cases**: Handles edge cases and boundary conditions
- [ ] **Data Validation**: Input validation and sanitization
- [ ] **Business Logic**: Correctly implements business rules
- [ ] **Integration**: Components integrate properly

### 3. Security Review
- [ ] **Authentication**: Proper authentication implemented
- [ ] **Authorization**: Role-based access control working
- [ ] **Data Protection**: Sensitive data encrypted
- [ ] **Input Sanitization**: Protection against injection attacks
- [ ] **OWASP Top 10**: Checked for common vulnerabilities
- [ ] **Secrets Management**: No hardcoded credentials

### 4. Performance Review
- [ ] **Response Times**: Meets performance requirements
- [ ] **Database Queries**: Optimized and indexed properly
- [ ] **Caching**: Appropriate caching strategies
- [ ] **Bundle Size**: Frontend assets optimized
- [ ] **Memory Usage**: No memory leaks detected
- [ ] **Scalability**: Can handle expected load

### 5. Testing Review
- [ ] **Unit Tests**: Adequate unit test coverage (>80%)
- [ ] **Integration Tests**: API endpoints tested
- [ ] **E2E Tests**: Critical user paths tested
- [ ] **Test Quality**: Tests are meaningful, not just coverage
- [ ] **Test Documentation**: Clear test descriptions
- [ ] **CI/CD**: Tests integrated in pipeline

### 6. Documentation Review
- [ ] **Code Documentation**: Functions/classes documented
- [ ] **API Documentation**: Endpoints documented with examples
- [ ] **README**: Updated with setup instructions
- [ ] **Architecture Docs**: System design documented
- [ ] **User Guides**: End-user documentation complete
- [ ] **Changelog**: Updates recorded in CHANGELOG.md

### 7. Accessibility Review
- [ ] **WCAG Compliance**: Meets WCAG 2.1 standards
- [ ] **Keyboard Navigation**: Fully keyboard accessible
- [ ] **Screen Readers**: Compatible with screen readers
- [ ] **Color Contrast**: Proper contrast ratios
- [ ] **Alt Text**: Images have descriptive alt text
- [ ] **ARIA Labels**: Proper ARIA attributes

### 8. Infrastructure Review
- [ ] **Docker**: Containerization configured properly
- [ ] **CI/CD**: Pipeline configured and working
- [ ] **Environment Config**: Proper env variable usage
- [ ] **Monitoring**: Logging and monitoring setup
- [ ] **Backup Strategy**: Data backup procedures defined
- [ ] **Deployment Docs**: Deployment process documented

## Severity Levels

### Critical (Blocks Release)
- Security vulnerabilities
- Data loss risks
- Breaking functionality
- Legal/compliance issues

### High (Must Fix)
- Performance issues
- Missing core features
- Poor error handling
- Accessibility barriers

### Medium (Should Fix)
- Code quality issues
- Missing documentation
- Minor bugs
- UX improvements needed

### Low (Nice to Have)
- Code style inconsistencies
- Optional enhancements
- Minor optimizations

## Review Process

### 1. Initial Check
```bash
# Run automated checks
npm run lint
npm run typecheck
npm run test
npm run test:coverage
```

### 2. Manual Review
- Review code changes line by line
- Test functionality manually
- Verify against requirements
- Check documentation updates

### 3. Report Generation
Create review report with:
- Phase name and date
- Items reviewed
- Issues found (by severity)
- Recommendations
- Pass/Fail determination

### 4. Decision Tree
```
IF critical_issues > 0:
    RETURN "FAILED - Critical issues must be resolved"
ELIF high_issues > 3:
    RETURN "FAILED - Too many high priority issues"
ELIF test_coverage < 70%:
    RETURN "FAILED - Insufficient test coverage"
ELIF documentation_incomplete:
    RETURN "CONDITIONAL - Documentation must be completed"
ELSE:
    RETURN "PASSED - Ready for next phase"
```

## Output Format

### Review Report Template
```markdown
# Phase Review Report

## Phase: [Phase Name]
## Date: [Review Date]
## Reviewer: @agent-reviewer

### Summary
- Total Items Reviewed: X
- Critical Issues: X
- High Priority: X
- Medium Priority: X
- Low Priority: X

### Critical Issues
[List critical issues with details]

### Recommendations
[Specific actions needed]

### Decision
[PASSED/FAILED/CONDITIONAL]

### Next Steps
[What needs to happen next]
```

## Integration Points
- Triggered by @agent-project_manager at phase completion
- Results reported to @agent-reporter for documentation
- Failures return to @agent-project_manager for remediation
- Success proceeds to next phase

## Notes
- Review must be thorough but efficient
- Focus on objective criteria
- Provide constructive feedback
- Document specific examples of issues
# Escalations

Role-to-role escalations. Each file records:
- **From** the role that escalated (implementer, adversary, etc.)
- **To** the role that addresses it (architect, analyst)
- **Date** of the escalation
- **Status**: OPEN or RESOLVED
- **Finding**: what was escalated
- **Resolution**: what was done (if RESOLVED)

Format:
```
# Escalation: <short title>

**Date:** YYYY-MM-DD
**From:** <role>
**To:** <role>
**Status:** OPEN | RESOLVED
**Severity:** <Critical | High | Medium | Low>

## Finding

<what was found, why it can't be resolved at the current level>

## Recommended outcome

<what the escalated-to role should do>

## Resolution (if RESOLVED)

<what was decided, when, and what changed>
```

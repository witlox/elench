---
description: "elench project status assessment. Run at the start of any session."
---

Project status assessment. Run at the start of any session.

1. Read `AGENTS.md` — project state table, workflow router
2. Read `NEXT-STEPS.md` — what's done, what's next
3. Read `specs/fidelity/INDEX.md` — test depth per invariant
4. Check build state:
   - `cargo build --quiet 2>&1 | tail -5` — does it compile?
   - `make` — fmt-check + lint + Tier 1
5. Check for open escalations: `ls specs/escalations/*.md 2>/dev/null`
6. Check for open findings: `ls specs/findings/*.md 2>/dev/null`
7. Summarize: current phase, test count, coverage, any blockers

# Decision Risk Rubric

Use this rubric to decide whether an unknown must block implementation.

## High risk

Treat an unknown as `HIGH` when being wrong is likely to require one or more of:

- database migration or destructive data correction
- a different ownership model or system of record
- a different state machine or irreversible transition
- changed authorization boundaries
- refunds, billing corrections, quota reconciliation, or accounting changes
- legal, contractual, compliance, privacy, or safety remediation
- incompatible API changes
- a materially different primary user journey
- significant rework across multiple components or services

Default action: resolve it or get explicit acceptance of the risk before building.

## Medium risk

Treat an unknown as `MEDIUM` when being wrong causes localized rework but can be contained by an explicit design seam, configuration, feature flag, adapter, or reversible assumption.

Default action: continue only after recording the assumption, boundary, and review trigger.

## Low risk

Treat an unknown as `LOW` when it affects copy, minor ordering, a cosmetic choice, a local default, or another easily reversible detail without changing domain behavior.

Default action: choose a sensible default, record only when useful, and continue.

## Confidence language

- `High confidence`: supported by direct project evidence or authoritative primary sources with no meaningful conflict.
- `Medium confidence`: supported by common practice or indirect evidence, but project-specific validation is incomplete.
- `Low confidence`: mainly inference, analogy, stale evidence, or conflicting sources.

Confidence is not risk. A low-confidence cosmetic choice can be low risk; a medium-confidence billing rule can be high risk.

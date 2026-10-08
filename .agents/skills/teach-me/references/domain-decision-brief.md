# Domain Decision Brief Template

Use this template as the final decision artifact. Remove sections that truly do not apply, but never merge facts, assumptions, and decisions.

## 1. Decision scope

- Goal:
- Pending decision:
- Why this decision matters now:
- Expensive-to-reverse consequences:
- Out of scope:

## 2. Minimum domain model

### Key terms

| Term | Working definition | Common confusion |
|---|---|---|

### Actors

| Actor | Goal | Authority | Constraints |
|---|---|---|---|

### Core entities and ownership

| Entity | Meaning | Owner/source of truth | Important relationships |
|---|---|---|---|

### Lifecycle or state model

Describe states, allowed transitions, transition triggers, and forbidden transitions.

### Normal flow

1.
2.
3.

### Exceptions and recovery

| Situation | Expected behavior | Recovery path |
|---|---|---|

### Invariants

-

## 3. Claim ledger

| Claim | Classification | Evidence | Confidence | Impact if wrong | Validation |
|---|---|---|---|---|---|

Allowed classifications:

- `VERIFIED_DOMAIN_FACT`
- `VERIFIED_PROJECT_FACT`
- `CURRENT_IMPLEMENTATION`
- `PROJECT_CONSTRAINT`
- `USER_PREFERENCE`
- `WORKING_ASSUMPTION`
- `RECOMMENDATION`
- `UNRESOLVED`
- `LIKELY_MISUNDERSTANDING`

## 4. Decision options

### Decision: [name]

| Option | User impact | Operational impact | Engineering impact | Edge cases | Reversibility |
|---|---|---|---|---|---|

Recommendation:

Confidence:

Recommendation changes if:

## 5. Confirmed decisions

-

## 6. Explicit assumptions

| Assumption | Risk | Why acceptable now | Review trigger |
|---|---|---|---|

## 7. Rejected options

| Option | Reason rejected | Reconsider when |
|---|---|---|

## 8. Unresolved questions

| Question | Risk | Validation method | Owner/source | Blocks build? |
|---|---|---|---|---|

## 9. Implementation constraints

Translate confirmed decisions into boundaries for later product shaping and coding. Do not design the implementation here unless necessary to prevent a known violation.

- Must:
- Must not:
- May assume:
- Must remain reversible:

## 10. Readiness

Status: `READY_TO_DECIDE` | `READY_TO_BUILD` | `NOT_READY`

Reason:

Next decision or validation action:

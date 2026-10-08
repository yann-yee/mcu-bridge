---
name: teach-me
description: 当用户对待决业务/专业领域不熟，且错误判断会造成明显返工时使用；先教到足以做决定。需求已明确的普通编码不要用。
---

# Teach Me

## Purpose

Teach the user enough about an unfamiliar domain to make a reliable product or engineering decision before Codex implements it.

The primary learner and decision-maker is the user. Your job is to help the user become sufficiently informed to make the current decision, not to silently replace the user's judgment and not to start coding early.

## Core principle

Treat every initial domain statement from the user as a claim to classify, not as an established business fact.

Always distinguish:

- verified domain fact
- verified project fact
- current implementation behavior
- project constraint
- user preference
- working assumption
- recommendation
- unresolved question

Never collapse these categories into a single list of "requirements".

## When to use

Use this skill when one or more of these are true:

- The user says the business or domain is unfamiliar.
- The user is about to choose business rules, states, permissions, pricing, workflows, data ownership, terminology, compliance boundaries, or exception handling.
- The user proposes a rule based mainly on intuition, analogy, or a simplified understanding.
- A wrong assumption could cause meaningful rework in the product, schema, API, state machine, permissions, billing, or user journey.
- The user asks to learn just enough about a domain to make a product or engineering decision.

Do not use this skill for:

- routine implementation with already confirmed requirements
- isolated bug fixes whose expected behavior is known
- general programming education
- purely aesthetic UI review
- broad, curiosity-driven domain research with no pending decision

## Operating rules

1. Do not write or modify production code until the decision gate is passed, unless the user explicitly asks for a disposable probe or experiment.
2. Do not assume the existing code is the domain truth. It is evidence of the current implementation only.
3. Prefer decision-relevant depth over encyclopedic coverage.
4. Challenge high-impact assumptions respectfully and explicitly.
5. Do not block progress on low-impact uncertainty. Record a reversible assumption and continue.
6. Do not decide silently for the user. Recommendations must remain recommendations until confirmed.
7. When external research is required, delegate evidence discovery and verification to `deepnote` when available instead of doing ad-hoc browsing inside Teach-Me.
8. Prefer authoritative and primary sources. Cite claims and state confidence.
9. When evidence conflicts, show the conflict instead of averaging it away.
10. Teach-Me owns the learning goal and user decision; DeepNote owns external evidence collection, authority assessment, conflict detection and evidence gaps.

## Workflow

### 1. Frame the pending decision

Extract:

- what the user is trying to achieve
- what decision they are about to give Codex
- what implementation choices would follow from that decision
- what could be expensive to reverse
- what is explicitly out of scope

Rewrite the user's current position as a short list of testable claims.

Example:

- "A paid order cannot be changed."
- "Only administrators need override permissions."
- "A plan downgrade should take effect immediately."

Do not yet endorse these claims.

### 2. Build the minimum useful domain model

Teach only the concepts required to evaluate the pending decision. Cover, where relevant:

- key terms and distinctions
- actors and incentives
- core entities and ownership
- lifecycle and state transitions
- normal flow
- exception and recovery flows
- invariants that must remain true
- common industry patterns
- common misconceptions

Use one concrete end-to-end example whenever it improves understanding.

The model should be concise enough for the user to hold in working memory.

### 3. Inspect available evidence

Use evidence in this order when available:

1. explicit product or business documentation
2. real user workflows, support cases, contracts, policies, or operational procedures
3. production behavior and data
4. authoritative external sources and standards
5. code, tests, and database schema as evidence of current behavior
6. team convention or historical precedent
7. intuition or analogy

For each important claim, state the strongest evidence found and what remains uncertain.

### 3A. Use DeepNote for external evidence

When a material claim depends on external domain facts and the `deepnote` skill is available, use it as the Evidence Provider.

Teach-Me should formulate focused research questions such as:

- What rule does the authoritative source actually require?
- Is this industry pattern mandatory or merely common?
- Under what conditions does this technical behavior hold?
- Do multiple authoritative sources agree?

Choose DeepNote mode by need:

- `EVIDENCE_BRIEF`: one decision-critical fact;
- `TOPIC_RESEARCH`: several related facts or competing approaches;
- `SOURCE_NOTE`: one important primary source must be understood in depth.

Expected handoff:

```text
Teach-Me
  → research question / pending decision
DeepNote
  → evidence + authority + claim status + applicability + conflicts + gaps
Teach-Me
  → compact domain model + options + recommendation + user decision
```

Do not let DeepNote silently make the product/business decision. It provides evidence; Teach-Me interprets that evidence for the user.

If the user asks to preserve what was learned in their personal knowledge base after the decision is understood, DeepNote may additionally produce a `KNOWLEDGE_NOTE` suitable for Obsidian.

Agent-only exception:

If no user education or decision is required and another Skill only needs a trusted external fact, that Skill may call DeepNote directly without Teach-Me.

### 4. Classify the user's claims

Create a claim ledger. Each claim must be labeled as one of:

- `VERIFIED_DOMAIN_FACT`
- `VERIFIED_PROJECT_FACT`
- `CURRENT_IMPLEMENTATION`
- `PROJECT_CONSTRAINT`
- `USER_PREFERENCE`
- `WORKING_ASSUMPTION`
- `RECOMMENDATION`
- `UNRESOLVED`
- `LIKELY_MISUNDERSTANDING`

For every non-verified claim, include:

- why it is uncertain
- what would falsify it
- impact if wrong
- cheapest useful validation method

### 5. Prioritize uncertainty by rework risk

Evaluate unresolved items against these dimensions:

- changes the domain model or entity ownership
- changes a lifecycle or state machine
- changes permissions or authorization
- changes money, billing, quotas, refunds, or accounting
- changes compliance, policy, safety, or contractual obligations
- changes API or persistence shape
- changes the primary user journey
- creates irreversible migration or data loss risk

Classify each item:

- `HIGH`: must be resolved or explicitly accepted before building
- `MEDIUM`: may proceed with a visible, reversible assumption
- `LOW`: record and continue

Read `references/decision-risk-rubric.md` when the risk is not obvious.

### 6. Compare decision options

For each material decision, present realistic options. Include:

- rule or behavior
- user impact
- operational impact
- engineering impact
- edge cases introduced
- reversibility
- evidence or precedent
- major advantage
- major risk

Then provide a recommendation with:

- rationale
- confidence level
- conditions under which the recommendation would change

Do not present a common industry pattern as mandatory unless it truly is mandatory.

### 7. Teach back and correct the model

Ask the user to confirm or correct the critical understanding in plain language when interaction is available.

A good checkpoint is not "Do you agree?". It is a concrete decision statement such as:

> Confirm: upgrades take effect immediately with prorated billing; downgrades take effect next cycle; failed payment leaves the current plan unchanged.

When interaction is not available, produce a provisional contract and clearly mark all decisions that still require user confirmation.

### 8. Produce the decision contract

Use `references/domain-decision-brief.md`.

The final artifact must contain:

- decision scope
- compact domain model
- claim ledger
- confirmed decisions
- explicit assumptions
- rejected options and reasons
- unresolved questions and owners or validation methods
- implementation constraints
- readiness status

## Readiness gate

Use exactly one status:

### `READY_TO_DECIDE`

The user now understands the relevant domain and options, but one or more product or business decisions still need confirmation.

### `READY_TO_BUILD`

High-impact domain rules and product decisions are confirmed. Remaining uncertainty is low-risk, reversible, and documented.

### `NOT_READY`

A high-impact unknown could materially change the state model, permissions, money flow, compliance boundary, persistence model, or primary journey.

Do not use `NOT_READY` merely because perfect knowledge is unavailable.

## Output style

Lead with the most consequential correction to the user's current model.

Prefer:

- compact explanations
- explicit labels
- decision tables
- concrete examples
- visible uncertainty

Avoid:

- long generic primers
- pretending certainty
- silently turning recommendations into requirements
- implementation plans before the decision contract
- asking broad questions that do not affect a decision

## Completion checklist

Before finishing, verify that:

- the user's initial claims were extracted and classified
- at least one relevant exception or failure path was considered
- high-impact unknowns were separated from low-impact unknowns
- facts, assumptions, preferences, and recommendations are visibly distinct
- external decision-critical claims used DeepNote evidence when available and appropriate
- the user owns the final decision
- the output includes a readiness status
- `READY_TO_BUILD` is not declared while a high-risk decision remains implicit

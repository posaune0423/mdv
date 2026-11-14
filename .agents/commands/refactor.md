# Refactor Command

You are a refactoring agent for large Bun + TypeScript codebases. Incrementally and reliably remove duplicate or similar code, extract shared logic, and improve maintainability. Keep conversation to a minimum and output only a summary of each cycle.

---

Project Conventions (Mandatory)

- Target Bun + TypeScript. Keep code in src/ and prefer co-location. Avoid proliferating files.
- Preserve public API compatibility. If a breaking change is unavoidable, provide a migration shim and deprecation notes together.
- Use only logger.debug for logging (console.\* is prohibited). Isolate side effects at boundaries.
- Do not edit outside the repository (such as the home directory). Do not manipulate .git directly; use normal git commands only.
- Do not use GraphQL.

---

Coding Rules (Guidelines)

FP: Prefer pure functions, immutable updates, isolated side effects, and type safety.
DDD: Distinguish value objects from entities, enforce consistency through aggregates, abstract persistence through repositories, and respect bounded contexts.
TDD: Use Red-Green-Refactor in small iterations and treat tests as specifications.

Types and Patterns

type Branded<T, B> = T & { \_brand: B };
type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };
// Value objects are immutable, validate themselves, and provide domain operations.

Repositories/Adapters: Work only with domain concepts and abstract external dependencies. Provide in-memory implementations for tests.

---

Preparation (Health Checks)

- Types: bun run typecheck
- Lint: bun run lint
- Tests: bun run test --coverage
- Dead code (optional): bunx ts-prune -p tsconfig.json

---

Analysis Steps (similarity)

Goal: Detect duplicate or similar code and prioritize it by "impact = lines × similarity".

Run

similarity-ts ./src --threshold 0.80 --min-lines 8 --cross-file --print

# Investigate partial overlap further as needed

similarity-ts ./src --experimental-overlap \
 --threshold 0.75 --overlap-min-window 8 --overlap-max-window 25 \
 --overlap-size-tolerance 0.25 --print

Analyze

- Aggregate the output by duplicate group and calculate similarity(%)/lines/priority=lines×similarity.
- Create an action plan in descending priority order.

---

Design Principles (Extraction and Consolidation)

- Extract identical logic into a reusable utility / service / strategy / hook.
- Accommodate differences in arguments or pre/post-processing through callback injection or template methods.
- Unify similar types using shared interfaces, type aliases, or branded types.
- Make exceptions and errors explicit with Result<T,E> and consistently use early returns.
- Migrate public API changes gradually through thin wrappers that delegate from old to new, with deprecation notes.

---

Implementation Cycle (Apply Safely with serena)

Iterate with one duplicate group per cycle. Each cycle follows "Explore → Design → Edit → Verify → Summarize". 1. Explore
Use serena search to list the affected locations and callers, and establish the scope. 2. Design (Document Explicitly)
Define the target modules/functions, argument and return types, exception/Result policy, side-effect locations, and logger.debug usage.
If public APIs are affected, design migration shims (old signature → new signature) and deprecation notes together. 3. Edit (Minimal Diff)
Open the affected files with serena and extract/consolidate while preserving co-location. Avoid excessive new files. 4. Verify (Immediately)

bun run typecheck && bun run lint && bun run test --coverage

If verification fails, make the smallest corrective changes and rerun it. 5. Output a summary (strictly follow the format below).

---

Output Format (Each Cycle)

[Target Group] <list of files and symbols>
[Detection Metrics] similarity=<%> / lines=<n> / priority=<score>
[Approach] Key extraction/consolidation/generalization decisions (1–3 lines)
[Changes] Affected files and main changes (function names, arguments, return values, exceptions/Result, logging)
[Verification] Summary of tsc/eslint/test status
[Follow-up] Remaining tasks/next candidates/migration guide (old API → new API)

---

Improvement Phase (Continuous Refactoring)

- Run bun run typecheck && bun run lint && bun run test --coverage in every cycle.
- Remove dead code (optional): bunx ts-prune -p tsconfig.json
- Continue introducing value objects and types for domain vocabulary. Avoid excessive abstraction and adjust to the complexity.

---

Completion Criteria

- Propose completion when the remaining combined impact of the highest-priority groups is ≤20% of the total.
- Finish with a final summary listing applied changes, deprecated APIs, and a migration guide.

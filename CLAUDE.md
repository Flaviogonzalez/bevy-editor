# Development principles

## Technical ownership

The developer must maintain a clear understanding of the entire codebase. Generating code is not the primary objective; producing understandable, maintainable software is.

### Rules

1. Never introduce abstractions without explaining the problem they solve.
2. Before implementing a feature, identify the existing modules and responsibilities involved.
3. Prefer modifying existing code over creating new files or abstractions when appropriate.
4. Explain architectural decisions before implementing them.
5. Implement incrementally. Avoid large, unrelated changes in a single operation.
6. Never assume that code is acceptable simply because it compiles or passes tests.
7. Every new module must have a clearly defined responsibility, inputs, outputs and dependencies.
8. When modifying existing functionality, explain how the change affects the rest of the system.

### Communication

For every significant implementation, explain:

- What was changed.
- Why it was necessary.
- How the implementation works.
- Which existing components interact with it.
- What alternatives were considered.

Prefer concrete explanations over generic architectural terminology.

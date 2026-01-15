# Plan process (implementation planning)

## Setup

Create a new plan directory at `<workspace>/plans/<YYYY-MM-dd>-<name>` where `<workspace>` is
`lp-app` or `lp-glsl` depending on scope of plan.

Use `date +%Y-%m-%d` to get the current date.

This will be the directory used to keep the plan files.

## Analysis

Analyze the current scope of work, and populate a `00-plans-notes.md` with the questions we need to
answer to plan the implementation in a `# Questions` section with a subsection for each question.

Each question should include context and a suggested answer.

## Question Iteration

Ask me the questions from the file ONE AT A TIME with:

- a summary of the current state
- your suggested course forward

I will answer or ask more questions.
You will record the answers in the `00-plans-notes.md` file.
If my answers imply additional questions, add them to the file.
If my answers include other notes, add them to the file in a `# Notes` section.

Move on to the next question.

## Plan Structure Decision

Once questions are answered, decide whether to use a single plan file or separate phase files:

- **Single file**: For simpler plans with 3-5 phases that can fit comfortably in one file
- **Separate files**: For complex plans with many phases or when phases need detailed documentation

When in doubt, use separate files. The structure is:

- `00-overview.md` - Plan overview and all phases listed
- `01-phase-title.md`, `02-phase-title.md`, etc. - Individual phase files

## Plan Creation

Once questions are answered, create the plan based on the analysis in `00-plans-notes.md`.

### Overview Section

Create `00-overview.md` (or include as first section if using single file) with:

- Title and brief overview of what the plan accomplishes
- List of phases with brief descriptions
- Success criteria for the overall plan

Example:

```markdown
# Plan: Project Commands

## Overview

Implement project management commands in the client-server system, enabling clients to load,
unload, and interact with projects running on the server.

## Phases

1. Extend message types for project commands
2. Implement project loading and unloading
3. Implement project request routing
4. Add client API methods
5. Add integration tests
6. Cleanup and finalization
```

### Phase Definition

Present phase suggestions based on the analysis and answered questions, like this example:

```
1. Extend ElfLoadInfo
2. Create Object Submodule Structure
3. Implement Object Layout Calculation
4. Implement Object Section Loading
5. Implement Object Symbol Map Building
6. Implement Symbol Map Merging
```

I will then make suggestions to change the phases, or add more phases.

Once I tell you that we're ready to start, save the phases to the plan directory.

### Single File Structure

If using a single file, create `00-plan.md` with:

- Overview section (as above)
- Phase sections, each with:
  - Phase title
  - Description
  - Success criteria
  - Implementation notes (if needed)

### Separate Files Structure

If using separate files:

- `00-overview.md` - Overview and phase list
- `01-phase-title.md`, `02-phase-title.md`, etc. - Individual phase files

Each phase file should include:

- Phase title
- Description
- Success criteria
- Implementation notes (if needed)

## Phase File Requirements

Every phase file (whether in single file or separate files) must include these style notes:

### Code Organization

- Place helper utility functions **at the bottom** of files
- Place more abstract things, entry points, and tests **first**
- Keep related functionality grouped together

### Formatting

- Run `cargo +nightly fmt` on all changes before committing
- Ensure consistent formatting across modified files

### Language and Tone

- Keep language professional and restrained
- Avoid overly optimistic language like "comprehensive", "fully production ready", "complete solution"
- Avoid emoticons
- Code is never done, never perfect, never fully ready, never fully complete
- Use measured, factual descriptions of what was implemented

## Implementation

### Phase Execution

For each phase:

1. Read the phase requirements
2. Implement the changes
3. Ensure code compiles
4. Fix warnings (except unused code that will be used in later phases)
5. Run relevant tests and ensure they pass
6. Run `cargo +nightly fmt` on changes
7. Commit with message: `lpc: <plan-name> - phase <N>: <phase-title>`

### Final Phase

The final phase should be a cleanup phase:

- Remove any temporary code, TODOs, debug prints, etc.
- Fix all warnings
- Ensure all tests pass
- Ensure all code is clean and readable
- Run `cargo +nightly fmt` on the entire workspace
- Move the plan directory to `<workspace>/plans/_done/`

Then commit the changes with a message like `lpc: complete plan <name>`. Include details of the
effect of the plan in the commit message (but not the implementation details).

## Success Criteria

Each phase should include:

- Specific, measurable success criteria
- Code compiles without errors
- Relevant tests pass
- Warnings addressed (except unused code for future phases)
- Code formatted with `cargo +nightly fmt`

The overall plan success criteria should be documented in the overview section.

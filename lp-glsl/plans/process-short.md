## Plan process (short plans)

Create a new plan file at `lightplayer/plans/<YYYY-MM-dd>-<name>.md`.

This will be the file used to keep the plan.

Analyze the current scope of work, and populate the plan with questions we need to answer to build this.

Once created, ask me the questions ONE AT A TIME with:

- a summary of the current state
- your suggested course forward

I will answer or ask more questions. We will then move on.

We must ensure that we have a clear acceptance criteria for the plan. There should be a phase
at the end for verifying the acceptance criteria and debugging.

Once you're done with the questions, you will present me with a suggestion of plan phases,
like this example:

```
1. Extend ElfLoadInfo
2. Create Object Submodule Structure
3. Implement Object Layout Calculation
4. Implement Object Section Loading
5. Implement Object Symbol Map Building
6. Implement Symbol Map Merging
```

I will then make suggestions to change the phases, or add more phases.

Once I tell you that we're ready to start, you write out the plan phases to the plan file

Each phase should end with:

- a success criteria section
- all code should compile
- Any warnings that aren't unused code (that will be used later) should be fixed.
- Tests relevant to the phase should be run and pass.
- Do not commit changes between phases

The final phase should be a cleanup phase:

- remove any temporary code or TODOs, debug prints, etc.
- fix all warnings
- ensure all tests pass
- ensure all code is clean and readable
- move the plan file to `lightplayer/plans/_done/`
- run 'cargo +nightly fmt' to format the code on lightplayer/ directory

# Plan process

Create a new plan directory at `<workspace>/plans/<YY-MM-dd>-<name>`. This will be the base
directory for all the plan files.
`<workspace>` is either `lp-app` or `lp-glsl` depending on the scope.

## Questions

Analyze the current scope of work, and create a plan file 00-questions.md with any questions we need
to answer to build this.

Once created, ask me the questions ONE AT A TIME with:

- a summary of the current state
- your suggested course forward

I will answer or ask more questions. We will then move on.

## Design

With the questions answered, you will present me with a design overview.
The main components of this should be the file structure and code structure.

Present this to me as a code block (do not put it in a file, yet) like

```
module/
└── src/
    ├── file.rs                 # NEW: File with things in it
    └── updater.rs              # File with the updater function
```

And separately a summary of new functions and types.

We will iterate on the structure and naming until we are happy with it.

At that point, write a 00-design.md file with the design overview.

## Phases

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

Once I tell you that we're ready to start, you will save these phases to the
plan directory as 00-phases.md.

You will then create a new file called 00-overview.md with the overview of the plan.

Then you will create a new file for each phase, named like 01-phase-title.md.

Then commit the plan to git with the message "lpc: create plan <name>"

Each phase should end with:

- a success criteria section
- all code should compile
- Any warnings that aren't unused code (that will be used later) should be fixed.
- Tests relevant to the phase should be run and pass.
- Commit changes between phases with a reference to the plan and phase in the commit message.

The final phase should be a cleanup phase:

- remove any temporary code or TODOs, debug prints, etc.
- fix all warnings
- ensure all tests pass
- ensure all code is clean and readable
- move the plan directory to `lightplayer/plans/_done/`
- run 'cargo +nightly fmt' to format the code on lightplayer/ directory

Then commit the changes with a message like "lpc: complete plan <name>". Include details of the
effect of the plan in the commit message (but not the implementation details).

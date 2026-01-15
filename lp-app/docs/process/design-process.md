# Plan process (architecture design)

## Setup

Create a new plan directory at `<workspace>/plans/<YYYY-MM-dd>-<name>.md` where `<workspace>` is
`lp-glsl` or `lp-app` depending on scope of plan.

Use `date +%Y-%m-%d` to get the current date.

This will be the directory used to keep the plan.

## Analysis

Analyze the current scope of work, and populate a 00-design-notes.md with the questions we need to
answer to design the architecture in a `# Questions` with a subsection for each question.

Each question should include context and a suggested answer.

## Question iteration

Ask me the questions from the file ONE AT A TIME with:

- a summary of the current state
- your suggested course forward

I will answer or ask more questions.
You will record the answers in the 00-design-notes.md file.
If my answers imply additional questions, add them to the file.
If my answers include other notes, add them to the file in a `# Notes` section.

Move on to the next question.

## Design iteration

Once questions are answered, you will present me with a suggestion of an
architecture design with two main elements:

The file sturcture as a bare-bones ascii file tree of the relevant directories and files.
Do NOT create a file to show the file tree, and types, print it to the user in a code block, like this:

```
module/
└── src/
    ├── file.ts                 # NEW: File with things in it
    └── updater.ts              # UPDATE: File with the updater function
```

A summary of the types and functions in a similar format to the file tree:

```
NewThing - # NEW: Trait for the new thing
├── method1() - # NEW: Method 1
└── method2() - # NEW: Method 2
```

If I want to make changes, I will tell you and you will note the changes in the 00-design-notes.md file
and show me relevant updates to the file tree and type summary.

## Design completion

Once the design is agreed, you will create a new file called 00-design.md with the design.

The design file should be structured as shown in design-example.md

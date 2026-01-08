## Plan process (architecture design)

Create a new plan directory at `lightplayer/plans/<YYYY-MM-dd>-<name>.md`.

This will be the directory used to keep the plan.

Analyze the current scope of work, and populate a 00-design-questions.md fileith questions we need to
answer to design the architecture.

Once created, ask me the questions ONE AT A TIME with:

- a summary of the current state
- your suggested course forward

I will answer or ask more questions. We will then move on.

---

Once questions are answered, you will present me with a suggestion of an
architecture design with two main elements:

The file sturcture as a bare-bones ascii file tree of the relevant directories and files.
A summary of the types and functions in a similar format to the file tree.

Do NOT create a file to show the file tree, and types, print it to the user in a code block.

We will then discuss the design and make changes as needed. Note the changes in the plan file.

---

Once the design is agreed, you will create a new file called 00-design.md with the design.

The design file should include the file tree and type tree near the top as code blocks for easy viewing.
It can go into some detail about the types but should be fairly high level.

We may then move on to an implementation plan, which will follow the process in process-long.md,
but reuse this plan directory.

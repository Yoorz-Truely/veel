# Changing Veel

Veel changes after v0 through written proposals, as Null's and Scowr's do (`~/Projects/Null/docs/rfcs/`, `../Scowr/docs/rfcs/`) and as Rust's RFCs do. Once real projects exist, a change in what a file means can cost every project a migration, so each one is argued on paper first. Until v0 freezes (Annilay's task 1d-01), the draft spec changes with the slice and the schema; from then on, this process applies.

## When a proposal is needed

- Any new node, attribute, value form, unit word or syntax that a `.veel` file can hold.
- Any change of meaning: what a word converts to, a default, a form of Veel's own over Kdenlive's assets, how identity and references resolve.
- Any change to the canonical form, to the rules of the smallest write-back edit, or to which diagnostics are errors.

No proposal is needed for:

- a fix to the spec's wording that changes no meaning;
- implementation work that follows the spec;
- new parameters that arrive with a new Kdenlive release through the generated schema (`schema/`), which need no change to the language.

## How

1. Copy [0000-template.md](0000-template.md) to `NNNN-short-name.md`, with the next free number.
2. Fill it in: the problem with a real example, the design with files before and after, what breaks, the migration, and the alternatives.
3. Add a row to Annilay's task board (`../Annilay/docs/TASKS.md`) naming the proposal, so it is decided rather than forgotten.
4. **The decision.** RL decides, or the lead under RL's standing instruction (Annilay's `AGENTS.md`, "Decisions and gates"). The decision is recorded in Annilay's `docs/DECISIONS.md` with its V-number, and in the proposal's status line.
5. **If accepted:**
   - the change is written into the spec and `CHANGELOG.md`, naming the proposal;
   - the code follows, with tests;
   - if it changes what an existing file means or whether it reads, the Veel format (`veel-format` in the project's manifest) goes up by one and a migration rewrites older files (below).
6. **If rejected:** it stays in this folder marked rejected, with the reason, so the question is not reopened without new arguments.

## Versions and migrations

Every project names the Veel format it is written for (`veel-format 1` in its manifest, Veelindy's spec). A change that breaks old files raises the format. Veelindy migrates an older project when it reads it, one step per format, each with a test that turns a file of the old format into the new one (`veel_migrate`, task 1d-02). A project in a format newer than the reader knows is an error, never guessed at.

## Proposals

| Number | Title | Status |
| --- | --- | --- |

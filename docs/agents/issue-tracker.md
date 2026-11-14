# Issue tracker: GitHub

The issues and specifications of OpenCraft are GitHub issues in
`shantanugoel/OpenCraft`. Use the `gh` CLI for all operations. `gh` finds the
repository from `git remote -v` when you run it in a clone.

[CONTRIBUTING.md](../CONTRIBUTING.md#how-the-issues-are-organized) gives how
the issues are organized: the roadmap issue, the phase issues, the milestones,
and the labels. This file gives only the commands.

## Commands

- **Make an issue.** `gh issue create --title "..." --body "..."`. Use a
  heredoc for a body with many lines.
- **Read an issue.** `gh issue view <number> --json number,title,body,labels,comments`.
- **List issues.** `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'`.
  Add `--label` and `--state` filters as necessary.
- **Make an issue a sub-issue of a parent.** `gh issue create --parent <parent> ...`,
  or `gh issue edit <parent> --add-sub-issue <child>` after you make it
  (`gh` 2.94 or later). With an older `gh`, use
  `gh api --method POST repos/<owner>/<repo>/issues/<parent>/sub_issues -F sub_issue_id=<child-db-id>`.
  The value is the database id, as in **Blocking** below. If sub-issues are
  not available, write `Part of #<parent>` at the top of the child body.
- **Write a comment.** `gh issue comment <number> --body "..."`
- **Add or remove a label.** `gh issue edit <number> --add-label "..."` or
  `--remove-label "..."`
- **Close an issue.** `gh issue close <number> --comment "..."`

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repository uses
external pull requests as feature requests. `/triage` reads this flag.)_

When the flag is `yes`, pull requests use the same labels and states as
issues. Use the `gh pr` commands:

- **Read a pull request.** `gh pr view <number> --comments`, and
  `gh pr diff <number>` for the diff.
- **List external pull requests for triage.** `gh api --paginate 'repos/{owner}/{repo}/pulls?state=open' --jq '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") | not) | {number, title, author: .user.login, author_association, labels: [.labels[].name]}'`.
- **Comment, label, close.** `gh pr comment`, `gh pr edit --add-label` or
  `--remove-label`, `gh pr close`.

Issues and pull requests use one set of numbers on GitHub. Thus `#42` can be
an issue or a pull request. Try `gh pr view 42` first. If it fails, use
`gh issue view 42`.

## When a skill says "publish to the issue tracker"

Make a GitHub issue. Obey the labels and the milestone rules in
[CONTRIBUTING.md](../CONTRIBUTING.md#how-the-issues-are-organized).

## When a skill says "fetch the relevant ticket"

Read the issue as in **Read an issue** above.

## Wayfinding operations

`/wayfinder` uses these operations. The **map** is one issue. Its **child**
issues are the tickets.

- **Map.** One issue with the label `wayfinder:map`. Its body holds the
  sections Notes, Decisions so far, and Fog.
  `gh issue create --label wayfinder:map`.
- **Child ticket.** An issue that is a GitHub sub-issue of the map (see
  **Make an issue a sub-issue of a parent**). If sub-issues are not available,
  add the child to a task list in the map body, and write `Part of #<map>` at
  the top of the child body. Labels: `wayfinder:<type>`, where the type is
  `research`, `prototype`, `grilling`, or `task`. When a person or an agent
  claims the ticket, assign it to the person who drives the work.
- **Blocking.** Use the GitHub issue dependencies. To add an edge, use
  `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`.
  `<blocker-db-id>` is the numeric database id of the blocker
  (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`). It is not the
  `#number` and not the `node_id`. GitHub gives the open blockers in
  `issue_dependencies_summary.blocked_by`. If dependencies are not available,
  write a line `Blocked by: #<n>, #<n>` at the top of the child body. A ticket
  is not blocked when all its blockers are closed.
- **Frontier query.** List the open children of the map
  (`gh issue list --state open`, only the sub-issues or the task list of the
  map). Remove each child that has an open blocker or an assignee. The first
  child in the order of the map is the next ticket.
- **Claim.** `gh issue edit <n> --add-assignee @me`. This is the first write of
  the session.
- **Resolve.** `gh issue comment <n> --body "<answer>"`, then
  `gh issue close <n>`. Then add a short summary and a link to the
  Decisions so far section of the map.

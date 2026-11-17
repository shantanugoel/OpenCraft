# 0041: The cask of OpenCraft is in the shared tap of the owner

- Status: Accepted
- Date: 2026-11-17

## Context

[Decision 0027](0027-homebrew-cask.md) put the cask in its own tap,
`shantanugoel/homebrew-opencraft`. The owner has more projects than this one.
A tap for each project means one more repository, one more `brew tap` name,
and one more README for each project.

The owner now has one tap for all projects, `shantanugoel/homebrew-tap`. It
holds a formula and the cask of Ganit. A tap name has the form
`user/repository` without the prefix `homebrew-`, thus the name of this tap is
`shantanugoel/tap` ([kb/homebrew.md](../kb/homebrew.md#a-tap-of-the-owner)).
OpenCraft has no external users when this record is written, thus no
installation uses the old tap.

## Decision

- **The repository.** The cask is in `shantanugoel/homebrew-tap`, in the file
  `Casks/opencraft.rb`. The command is
  `brew install --cask shantanugoel/tap/opencraft`. The repository
  `shantanugoel/homebrew-opencraft` is deleted. No redirect is necessary.
- **The README.** The README of the tap lists all casks and formulae. It
  lives in the tap. This repository holds no copy of it.
- **The rest of decision 0027 stays.** The cask text and its stanzas, the
  script `scripts/homebrew-cask.sh`, the manual release, and the reasons for
  them do not change. Only the place of the file changes.

## Results

- The steps to publish a release are in
  [scripts/README.md](../../scripts/README.md#publish-the-homebrew-cask).
  They write the file into a checkout of the shared tap.
- A change in the shared tap by a different project does not touch
  `Casks/opencraft.rb`. A push to the tap can fail if a different project
  pushed first. Pull and push again.
- The tests of
  [kb/homebrew.md](../kb/homebrew.md#the-published-cask) used the old tap
  name. The cask text did not change, but the command with the new tap name is
  not measured.

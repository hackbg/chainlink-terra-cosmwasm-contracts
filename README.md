# Chainlink Terra

TODO: add more documentation here ...

## Contributing

For commit message naming convention we use [conventional commits](https://www.conventionalcommits.org/). While this is not enforced, please try to stick to this as it eases the reviewers and also allows us to build automated `changelog` directly out of the commit messages if compliant to the format.

We use the [gitflow](https://danielkummer.github.io/git-flow-cheatsheet/) workflow [this is also helpful](https://gist.github.com/JamesMGreene/cdd0ac49f90c987e45ac).
* Development of features happens in branches made from `develop` called `feature/<the-feature>` like `feature/human-address`.
* When development is finished a pull request to `develop` is created. At least one person has to review the PR and when everything is fine the PR gets merged.
* To make a new release create a release branch called `release/X.X.X`, also bump the version number in this branch.
* Create a PR to `main` which then also has to be accepted.
* Create a tag for this version and push the tag.
* Also merge back the changes (like the version bump) into `develop`.
* The `main` branch has to be deployed to the [production environment]() automatically after PR merge.
  
### Rules
- Use `rebase` instead of `merge` to update your codebase, except when a PR gets included in a branch.
- Use meaningfull descriptions and titles in commit messages.
- Explain what you did in your PRs, add images whenever possible for showing the status before/after the change visually. 

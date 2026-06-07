The workhorse metadata chip that carries object ownership everywhere — branches, worktrees, models, SHAs, ticket IDs, counts.

```jsx
<MetaChip tone="branch" icon={<GitBranch/>}>agent/eng-221-oauth</MetaChip>
<MetaChip tone="worktree" icon={<FolderGit2/>}>~/wt/eng-221</MetaChip>
<MetaChip tone="pr" icon={<GitPullRequest/>}>#84</MetaChip>
<MetaChip tone="linear" mono={false}>ENG-221</MetaChip>
```

`tone` tints the chip to its owning domain (branch, worktree, pr, linear, github, brain, accent). Keep values in mono so paths/SHAs align and copy cleanly.

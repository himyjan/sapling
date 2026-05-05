  $ setconfig diff.git=True
  $ setconfig subtree.allow-any-source-commit=True
  $ setconfig subtree.min-path-depth=1

test backout of subtree commit is aborted
  $ newclientrepo
  $ drawdag <<'EOS'
  > B  # B/foo/x = 1a\n2\n3\n
  > |
  > A  # A/foo/x = 1\n2\n3\n
  > EOS
  $ sl go -q $B
  $ sl subtree copy -r $A --from-path foo --to-path foo2 -m "subtree copy foo to foo2"
  copying foo to foo2
  $ sl backout -r .
  abort: cannot backout subtree copy/merge commits
  (use 'sl subtree copy -r <good-commit>' to overwrite the path, then graft changes on top if needed)
  [255]

test backout of subtree commit shows configurable hint
  $ sl backout -r . --config subtree.backout-hint="see https://example.com for details"
  abort: cannot backout subtree copy/merge commits
  (use 'sl subtree copy -r <good-commit>' to overwrite the path, then graft changes on top if needed. see https://example.com for details)
  [255]

test backout subtree merge: backout fails, subtree copy overwrites, future merge works
  $ newclientrepo
  $ drawdag <<'EOS'
  > B  # B/foo/x = 1a\n2\n3\n
  > |
  > A  # A/foo/x = 1\n2\n3\n
  > EOS
  $ sl go -q $B
  $ sl subtree copy -r $A --from-path foo --to-path bar -m "subtree copy foo to bar"
  copying foo to bar

step 1: create a subtree merge commit
  $ echo "change1" >> foo/x && sl ci -m "update foo with change1"
  $ sl subtree merge --from-path foo --to-path bar | grep "merge base:"
  merge base: b4cb27eee4e2
  $ sl ci -m "subtree merge foo to bar"
  $ cat bar/x
  1a
  2
  3
  change1

step 2: backout the subtree merge commit does not work
  $ sl backout -r .
  abort: cannot backout subtree copy/merge commits
  (use 'sl subtree copy -r <good-commit>' to overwrite the path, then graft changes on top if needed)
  [255]

step 3: subtree copy overwrites the dest path to a good state
  $ sl log -G -T '{node|short} {desc|firstline}'
  @  26d65d502329 subtree merge foo to bar
  │
  o  49d9ca68c22d update foo with change1
  │
  o  ee6785824a72 subtree copy foo to bar
  │
  o  c4fbbcdf676b B
  │
  o  b4cb27eee4e2 A
  $ sl subtree copy -r .^ --from-path bar --to-path bar --force -m "overwrite bar to pre-merge state"
  removing bar/x
  copying bar to bar
  $ cat bar/x
  1
  2
  3

step 4: future subtree merge finds correct merge base and merges the changes back
  $ echo "change2" >> foo/x && sl ci -m "update foo with change2"
  $ sl subtree merge --from-path foo --to-path bar | grep "merge base:"
  merge base: b4cb27eee4e2
  $ sl ci -m "subtree merge foo to bar again"
  $ cat bar/x
  1a
  2
  3
  change1
  change2

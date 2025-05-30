#modern-config-incompatible

#require no-eden no-symlink

TODO(debugruntest): this test fails on Mac
#chg-compatible

  $ setconfig experimental.windows-symlinks=false

# The following (test) script was used to create the bundle:
#
# configure modernclient
# newclientrepo symlinks
# echo a > a
# mkdir d
# echo b > d/b
# ln -s a a.lnk
# ln -s d/b d/b.lnk
# hg ci -Am t
# hg bundle --base null $TESTDIR/bundles/test-no-symlinks.hg

Extract a symlink on a platform not supporting them

  $ hg init t
  $ cd t
  $ hg unbundle -q "$TESTDIR/bundles/test-no-symlinks.hg"
  $ hg goto tip
  4 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ cat a.lnk && echo
  a
  $ cat d/b.lnk && echo
  d/b

Copy a symlink and move another

  $ hg copy a.lnk d/a2.lnk
  $ hg mv d/b.lnk b2.lnk
  $ hg ci -Am copy
  $ cat d/a2.lnk && echo
  a
  $ cat b2.lnk && echo
  d/b

Bundle and extract again

  $ hg bundle --base null ../symlinks.hg
  2 changesets found
  $ cd ..
  $ hg init t2
  $ cd t2
  $ hg unbundle -q ../symlinks.hg
  $ hg goto tip
  5 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ cat a.lnk && echo
  a
  $ cat d/a2.lnk && echo
  a
  $ cat b2.lnk && echo
  d/b

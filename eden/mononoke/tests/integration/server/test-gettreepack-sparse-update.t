# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

  $ export LOG=revisionstore::scmstore::tree=info

Setup repo, and create test repo

  $ BLOB_TYPE="blob_files" EMIT_OBSMARKERS=1 quiet default_setup_drawdag
  $ mkdir sparse
  $ cat > sparse/profile <<EOF
  > path:sparse/
  > EOF
  $ hg commit -Aqm 'initial'

  $ mkdir -p foo/foo/{foo1,foo2,foo3} bar/bar/{bar1,bar2,bar3}
  $ touch foo/foo/{foo1,foo2,foo3}/123 bar/bar/{bar1,bar2,bar3}/456
  $ hg commit -Aqm 'add files'

  $ cat >> sparse/profile <<EOF
  > # some comment
  > EOF
  $ hg commit -Aqm 'modify sparse profile'

  $ touch foo/456
  $ hg commit -Aqm 'add more files'

  $ hg push -q -r . --to master_bookmark --force

Setup a client repo that doesn't have any of the manifests in its local store.

  $ hg clone -q mono:repo test_repo --noupdate
  $ cd test_repo
  $ hg pull -q -B master_bookmark

Set up some config to enable sparse profiles, get logging from fetches, and
also disable ondemand fetch to check this is overriden by sparse profiles.

  $ cat >> ".hg/hgrc" << EOF
  > [ui]
  > interactive=True
  > [remotefilelog]
  > debug=True
  > cachepath=$TESTTMP/test_repo.cache
  > [treemanifest]
  > ondemandfetch=False
  > [extensions]
  > sparse=
  > EOF

Checkout commits. Expect BFS prefetch to fill our tree

  $ hg up 'master_bookmark~3'
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ hg sparse enable sparse/profile

  $ hg up 'master_bookmark~2'
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
  0 files updated, 0 files merged, 0 files removed, 0 files unresolved

# Now, force load the root tree for the commit we have, which simulates a hg
# cache that has the data we care about but not entire, not-changed-recently
# trees that are outside our sparse profile. We expect to see BFS
# fetching for the rest of the tree.

  $ rm -r "$TESTTMP/test_repo.cache"
  $ hg debuggetroottree "$(hg log -r '.' -T '{manifest}')"
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)

  $ hg up 'master_bookmark' --config sparse.force_full_prefetch_on_sparse_profile_change=True
  2 files fetched over 2 fetches - (2 misses, 0.00% hit ratio) over * (glob) (?)
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *:scmstore::tree::fetch: exit (glob)
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

# Now, force load the root tree for the commit again, and do update to master_bookmark
# without force_full_prefetch_on_sparse_profile_change set. Note that we fetch less trees

  $ hg up 'master_bookmark~2' -q
  $ rm -r "$TESTTMP/test_repo.cache"
  $ hg debuggetroottree "$(hg log -r '.' -T '{manifest}')"
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
  $ hg up 'master_bookmark'
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
   INFO fetch_edenapi: revisionstore::scmstore::tree::fetch: enter
   INFO fetch_edenapi{* requests=1 *}: revisionstore::scmstore::tree::fetch: exit (glob)
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

Check that we can create some commits, and that nothing breaks even if the
server does not know about our root manifest.

  $ hg book client

  $ cat >> sparse/profile <<EOF
  > # more comment
  > EOF
  $ hg commit -Aqm 'modify sparse profile again'

  $ hg up 'client~1'
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  (leaving bookmark client)

  $ hg up 'client'
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved
  (activating bookmark client)

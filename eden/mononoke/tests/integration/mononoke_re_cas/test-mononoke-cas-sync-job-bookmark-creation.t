# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"
  $ setup_common_config
  $ export CAS_STORE_PATH="$TESTTMP"
  $ setconfig drawdag.defaultfiles=false

  $ start_and_wait_for_mononoke_server
  $ hg clone -q mono:repo repo
  $ cd repo
  $ drawdag << EOS
  > D # D/qux = random:30
  > |
  > C # C/baz = random:30
  > |
  > B # B/bar = random:30
  > |
  > A # A/foo = random:30
  > EOS

  $ hg goto D -q
  $ hg push -r . --to master_bookmark -q --create


Validate that blobs and trees were uploaded for _all_ 4 commits (this should include 4 files and 4 trees)
  $ mononoke_cas_sync repo 0
  [INFO] [execute{repo=repo}] Initiating mononoke RE CAS sync command execution
  [INFO] [execute{repo=repo}] using repo "repo" repoid RepositoryId(0)
  [INFO] [execute{repo=repo}] syncing log entries [1] ...
  [INFO] [execute{repo=repo}] log entry BookmarkUpdateLogEntry { id: 1, repo_id: * } is a creation of bookmark (glob)
  [INFO] [execute{repo=repo}] log entries [1] synced (4 commits uploaded, upload stats: uploaded digests: 8, already present digests: 0, uploaded bytes: 2.0 KiB, the largest uploaded blob: 717 B), took overall * sec (glob)
  [INFO] [execute{repo=repo}] queue size after processing: 0
  [INFO] [execute{repo=repo}] successful sync of entries [1]
  [INFO] [execute{repo=repo}] Finished mononoke RE CAS sync command execution for repo repo

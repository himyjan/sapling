# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ export COMMIT_DATE="1985-09-04T00:00:00.00Z"
  $ export MASTER_BOOKMARK_NAME="master_bookmark"

-- Use large repo as the default repo
  $ export REPONAME="$LARGE_REPO_NAME"

  $ cat >> $HGRCPATH <<EOF
  > [extensions]
  > rebase=
  > pushrebase=
  > EOF



  $ . "${TEST_FIXTURES}/library.sh"
  $ . "${TEST_FIXTURES}/cross_repo/library-git-submodules-config-setup.sh"
  $ . "${TEST_FIXTURES}/cross_repo/library-git-submodules-helpers.sh"

  $ quiet hg config -g rebase.reproducible-commits=true


Setup configuration
  $ quiet run_common_xrepo_sync_with_gitsubmodules_setup

  $ testtool_drawdag -R "$LARGE_REPO_NAME" --no-default-files <<EOF
  > A
  > # modify: A "large_repo_root.txt" "file in large repo root"
  > # bookmark: A master_bookmark
  > EOF
  A=3f09a27a52abfe9117e13ec42027d0220ed88ce2b8ad976cf49e5b6d28fc2baa

# Simple integration test for the initial-import command in the forward syncer
Create small repo commits
  $ testtool_drawdag -R "$SUBMODULE_REPO_NAME" --no-default-files <<EOF
  > A-B-C
  > # modify: A "foo/a.txt" "creating foo directory"
  > # modify: A "bar/b.txt" "creating bar directory"
  > # modify: B "bar/c.txt" "random change"
  > # modify: B "foo/d" "another random change"
  > # copy: C "foo/b.txt" "copying file from bar into foo" B "bar/b.txt"
  > # bookmark: C master_bookmark
  > EOF
  A=7e97054c51a17ea2c03cd5184826b6a7556d141d57c5a1641bbd62c0854d1a36
  B=2999dcf517994fe94506b62e5a9c54f851abd4c4964f98fdd701c013abd9c0c3
  C=738630e43445144e9f5ddbe1869730cfbaf8ff6bf95b25b8410cb35ca92f25c7


  $ quiet mononoke_x_repo_sync "$SUBMODULE_REPO_ID"  "$LARGE_REPO_ID" \
  >   initial-import --no-progress-bar -i "$C" --version-name "$LATEST_CONFIG_VERSION_NAME"

  $ quiet mononoke_admin megarepo gradual-merge \
  >   --repo-id "$LARGE_REPO_ID" -a test_user -m "gradual merge" \
  >   --last-deletion-commit -i ca175120dfe7fb7fcb0d872e26ce331cb24c7d9ec457d599a40684527c65d63a \
  >   --pre-deletion-commit -i ca175120dfe7fb7fcb0d872e26ce331cb24c7d9ec457d599a40684527c65d63a  \
  >   --target-bookmark "$MASTER_BOOKMARK_NAME" --limit 10 \
  >   --commit-date-rfc3339 "$COMMIT_DATE" 2>&1 | tee "$TESTTMP/gradual_merge.out"

  $ clone_and_log_large_repo "ca175120dfe7fb7fcb0d872e26ce331cb24c7d9ec457d599a40684527c65d63a"
  @    4d720f131197 [MEGAREPO GRADUAL MERGE] gradual merge (0)
  ├─╮   smallrepofolder1/bar/b.txt |  1 +
  │ │   smallrepofolder1/bar/c.txt |  1 +
  │ │   smallrepofolder1/foo/a.txt |  1 +
  │ │   smallrepofolder1/foo/b.txt |  1 +
  │ │   smallrepofolder1/foo/d     |  1 +
  │ │   5 files changed, 5 insertions(+), 0 deletions(-)
  │ │
  │ o  cbb9c8a988b5 C
  │ │   smallrepofolder1/foo/b.txt |  1 +
  │ │   1 files changed, 1 insertions(+), 0 deletions(-)
  │ │
  │ o  5e3f6798b6a3 B
  │ │   smallrepofolder1/bar/c.txt |  1 +
  │ │   smallrepofolder1/foo/d     |  1 +
  │ │   2 files changed, 2 insertions(+), 0 deletions(-)
  │ │
  │ o  e462fc947f26 A
  │     smallrepofolder1/bar/b.txt |  1 +
  │     smallrepofolder1/foo/a.txt |  1 +
  │     2 files changed, 2 insertions(+), 0 deletions(-)
  │
  o  20ceeabb70c6 A
      large_repo_root.txt |  1 +
      1 files changed, 1 insertions(+), 0 deletions(-)
  
  
  
  Running mononoke_admin to verify mapping
  
  RewrittenAs([(ChangesetId(Blake2(738630e43445144e9f5ddbe1869730cfbaf8ff6bf95b25b8410cb35ca92f25c7)), CommitSyncConfigVersion("INITIAL_IMPORT_SYNC_CONFIG"))])
  
  Deriving all the enabled derived data types



-- Prepare large repo
  $ cd "$TESTTMP/$LARGE_REPO_NAME"
  $ hg pull -q && hg co -q master_bookmark
  $ enable commitcloud infinitepush # to push commits to server

-- Create a large repo commit
  $ echo "file" > large_repo_file.txt
  $ hg commit -Aq -m "change large repo file" 

-- Create a commit in small repo folder to be backsynced
  $ echo "abc" > smallrepofolder1/new_file.txt
  $ hg commit -Aq -m "change small repo from large repo" --date "1 1"
  $ ORIGINAL_HG_COMMIT=$(hg whereami)

-- Go back and create another large repo commit
  $ hg co -q .^
  $ echo "change file" > large_repo_file.txt
  $ hg commit -Aq -m "change large repo file AGAIN" 
  $ REBASE_TARGET=$(hg whereami)
  $ hg push -q --to master_bookmark

  $ hg co -q $ORIGINAL_HG_COMMIT

-- Rebase small repo commit to new target
  $ hg rebase --keep -r $ORIGINAL_HG_COMMIT -d $REBASE_TARGET -q --config rebase.reproducible-commits=true
  $ REBASED_HG_COMMIT=$(hg whereami)

  $ hg log --graph -T '{node|short} {desc}\n' --stat -r "sort(all(), desc)"
  @  7b5f91f0c5b2 change small repo from large repo
  │   smallrepofolder1/new_file.txt |  1 +
  │   1 files changed, 1 insertions(+), 0 deletions(-)
  │
  │ o  7faf677d5367 change small repo from large repo
  │ │   smallrepofolder1/new_file.txt |  1 +
  │ │   1 files changed, 1 insertions(+), 0 deletions(-)
  │ │
  o │  5d44cb1dbffb change large repo file AGAIN
  ├─╯   large_repo_file.txt |  2 +-
  │     1 files changed, 1 insertions(+), 1 deletions(-)
  │
  o  72e1f4df4120 change large repo file
  │   large_repo_file.txt |  1 +
  │   1 files changed, 1 insertions(+), 0 deletions(-)
  │
  o    4d720f131197 [MEGAREPO GRADUAL MERGE] gradual merge (0)
  ├─╮   smallrepofolder1/bar/b.txt |  1 +
  │ │   smallrepofolder1/bar/c.txt |  1 +
  │ │   smallrepofolder1/foo/a.txt |  1 +
  │ │   smallrepofolder1/foo/b.txt |  1 +
  │ │   smallrepofolder1/foo/d     |  1 +
  │ │   5 files changed, 5 insertions(+), 0 deletions(-)
  │ │
  │ o  cbb9c8a988b5 C
  │ │   smallrepofolder1/foo/b.txt |  1 +
  │ │   1 files changed, 1 insertions(+), 0 deletions(-)
  │ │
  │ o  5e3f6798b6a3 B
  │ │   smallrepofolder1/bar/c.txt |  1 +
  │ │   smallrepofolder1/foo/d     |  1 +
  │ │   2 files changed, 2 insertions(+), 0 deletions(-)
  │ │
  │ o  e462fc947f26 A
  │     smallrepofolder1/bar/b.txt |  1 +
  │     smallrepofolder1/foo/a.txt |  1 +
  │     2 files changed, 2 insertions(+), 0 deletions(-)
  │
  o  20ceeabb70c6 A
      large_repo_root.txt |  1 +
      1 files changed, 1 insertions(+), 0 deletions(-)
  
-- Backup all commits to commit cloud
  $ hg cloud backup -q

  $ ORIG_BONSAI_HASH=$(mononoke_admin convert -R $LARGE_REPO_NAME -f hg -t bonsai $ORIGINAL_HG_COMMIT)
  $ echo "ORIG_BONSAI_HASH: $ORIG_BONSAI_HASH"
  ORIG_BONSAI_HASH: 45b0a006e9f7012884ec6d8799e45eeaac7583f4b4a0cd06eec06e839b7748a7

  $ REBASED_BONSAI_HASH=$(mononoke_admin convert -R $LARGE_REPO_NAME -f hg -t bonsai $REBASED_HG_COMMIT)
  $ echo "REBASED_BONSAI_HASH: $REBASED_BONSAI_HASH"
  REBASED_BONSAI_HASH: 3bac370e50ea100cc1eb8b0559209335d7069c7c57235bc9dad51fdf453d76a1


  $ mononoke_admin blobstore -R $LARGE_REPO_NAME fetch \
  >   "changeset.blake2.$ORIG_BONSAI_HASH" > $TESTTMP/large_repo_original_bonsai

  $ mononoke_admin blobstore -R $LARGE_REPO_NAME fetch \
  >   "changeset.blake2.$REBASED_BONSAI_HASH" > $TESTTMP/large_repo_rebased_bonsai


-- Sync both commits to small repo

  $ SMALL_REPO_COMMIT_A=$(hg debugapi --sort  -e committranslateids \
  >   -i "[{'Hg': '$ORIGINAL_HG_COMMIT'}]" -i "'Bonsai'" -i None -i "'$SUBMODULE_REPO_NAME'" | \
  >   rg '.+"translated": \{"Bonsai": bin\("(\w+)"\)\}\}\]' -or '$1')

  $ echo "SMALL_REPO_COMMIT_A: $SMALL_REPO_COMMIT_A"
  SMALL_REPO_COMMIT_A: 6405c11e11523c644532ffce53c95793f2051de3ecd52115e6a74a81e4cd4d7e

  $ SMALL_REPO_COMMIT_B=$(hg debugapi --sort -e committranslateids \
  >   -i "[{'Hg': '$REBASED_HG_COMMIT'}]" -i "'Bonsai'" -i None -i "'$SUBMODULE_REPO_NAME'" | \
  >   rg '.+"translated": \{"Bonsai": bin\("(\w+)"\)\}\}\]' -or '$1')

  $ echo "SMALL_REPO_COMMIT_B: $SMALL_REPO_COMMIT_B"
  SMALL_REPO_COMMIT_B: 6405c11e11523c644532ffce53c95793f2051de3ecd52115e6a74a81e4cd4d7e


-- Now fetch both changeset blobs

  $ mononoke_admin blobstore -R $SUBMODULE_REPO_NAME fetch \
  >   "changeset.blake2.$SMALL_REPO_COMMIT_A" > $TESTTMP/commit_a_bonsai

  $ mononoke_admin blobstore -R $SUBMODULE_REPO_NAME fetch \
  >   "changeset.blake2.$SMALL_REPO_COMMIT_B" > $TESTTMP/commit_b_bonsai

-- To debug the raw bonsais, uncomment the line below
# $ diff -y -t -T $TESTTMP/commit_a_bonsai $TESTTMP/commit_b_bonsai


# Examine committer and committer date, which are not set in the large repo, but
# should be set on the small git repo.
  $ mononoke_admin fetch -R $LARGE_REPO_NAME -i $REBASED_HG_COMMIT \
  > --json > $TESTTMP/commit_b_info_large_repo.json
  $ jq '{author: .author, author_date: .author_date, committer: .committer, committer_date: .committer_date}' \
  > $TESTTMP/commit_b_info_large_repo.json
  {
    "author": "test",
    "author_date": "1970-01-01T00:00:00-00:00",
    "committer": null,
    "committer_date": null
  }

  $ mononoke_admin fetch -R $SUBMODULE_REPO_NAME -i $SMALL_REPO_COMMIT_B \
  > --json > $TESTTMP/commit_b_info_small_repo.json
  $ jq '{author: .author, author_date: .author_date, committer: .committer, committer_date: .committer_date}' \
  > $TESTTMP/commit_b_info_small_repo.json
  {
    "author": "test",
    "author_date": "1970-01-01T00:00:00-00:00",
    "committer": "test",
    "committer_date": "1970-01-01T00:00:00-00:00"
  }

-- Derive git commit for commit A
  $ mononoke_admin derived-data -R $SUBMODULE_REPO_NAME \
  >   derive -T git_commits -i "$SMALL_REPO_COMMIT_A"

-- Derivation of commit B will succeed because hg extra is stripped
  $ mononoke_admin derived-data -R $SUBMODULE_REPO_NAME \
  >   derive -T git_commits -i "$SMALL_REPO_COMMIT_B"

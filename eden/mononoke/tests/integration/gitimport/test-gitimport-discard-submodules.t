# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"
  $ setup_common_config blob_files
  $ GIT_REPO_ORIGIN="${TESTTMP}/origin/repo-git"
  $ GIT_REPO_SUBMODULE="${TESTTMP}/origin/repo-submodule"
  $ GIT_REPO="${TESTTMP}/repo-git"
  $ HG_REPO="${TESTTMP}/repo"
  $ BUNDLE_PATH="${TESTTMP}/repo_bundle.bundle"


# Setup submodule git repository
  $ mkdir -p "$GIT_REPO_SUBMODULE"
  $ cd "$GIT_REPO_SUBMODULE"
  $ git init -q
  $ echo "this is submodule file1" > sub_file1
  $ git add sub_file1
  $ git commit -q -am "Add submodule file1"
  $ echo "this is submodule file2" > sub_file2
  $ git add sub_file2
  $ git commit -q -am "Add submodule file2"

# Setup git repository
  $ mkdir -p "$GIT_REPO_ORIGIN"
  $ cd "$GIT_REPO_ORIGIN"
  $ git init -q
  $ echo "this is file1" > file1
  $ git add file1
  $ git commit -q -am "Add file1"
  $ git tag -a -m"new tag" first_tag
  $ echo "this is file2" > file2
  $ git add file2
  $ git commit -q -am "Add file2"

# Add a submodule in this repository (use relative path as $TESTTMP in a commit makes the hash unstable)
  $ git submodule add "../repo-submodule"
  Cloning into '$TESTTMP/origin/repo-git/repo-submodule'...
  done.
  $ git add .
  $ git commit -q -am "Add a new submodule"
  $ git tag -a empty_tag -m ""

  $ cd "$TESTTMP"
  $ git clone "$GIT_REPO_ORIGIN"
  Cloning into 'repo-git'...
  done.
# Capture all the known Git objects from the repo
  $ cd $GIT_REPO
  $ git fetch "$GIT_REPO_ORIGIN" +refs/*:refs/* --prune -u
  From $TESTTMP/origin/repo-git
   - [deleted]         (none)     -> origin/master_bookmark
     (refs/remotes/origin/HEAD has become dangling)
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/object_list
  $ cat $TESTTMP/object_list
  433eb172726bc7b6d60e8d68efb0f0ef4e67a667 blob file1
  441e95750f7eb05137204a7684a4cafe7cc0da0f blob .gitmodules
  7327e6c9b533787eeb80877d557d50f39c480f54 tree 
  7565d37e20d5b551bee27c9676e4856d47bc1806 tree 
  7aa1d50cd2865dd8fd86444d7a8ff5b2a23fe3b2 tag empty_tag
  8963e1f55d1346a07c3aec8c8fc72bf87d0452b1 tag first_tag
  8ce3eae44760b500bf3f2c3922a95dcd3c908e9e commit 
  cb2ef838eb24e4667fee3a8b89c930234ae6e4bb tree 
  e8615d6f149b876be0a2f30a1c5bf0c42bf8e136 commit 
  f138820097c8ef62a012205db0b1701df516f6d5 blob file2
  fbae2e73cbaa3acf4d844c32bcbd5c79e722630d commit 

# Get the repository log
  $ git log --pretty=format:"%h %an %s %D" > $TESTTMP/repo_log
  $ cat $TESTTMP/repo_log
  fbae2e7 mononoke Add a new submodule HEAD -> master_bookmark, tag: empty_tag
  e8615d6 mononoke Add file2 
  8ce3eae mononoke Add file1 tag: first_tag (no-eol)

# Look at the commit that introduced the submodule:
# The .gitmodules file gets updated. a new blob of type: submodule gets added at repo-submodule that is the actual submodule
  $ git show fbae2e7
  commit fbae2e73cbaa3acf4d844c32bcbd5c79e722630d
  Author: mononoke <mononoke@mononoke>
  Date:   Sat Jan 1 00:00:00 2000 +0000
  
      Add a new submodule
  
  diff --git a/.gitmodules b/.gitmodules
  new file mode 100644
  index 0000000..441e957
  --- /dev/null
  +++ b/.gitmodules
  @@ -0,0 +1,3 @@
  +[submodule "repo-submodule"]
  +	path = repo-submodule
  +	url = ../repo-submodule
  diff --git a/repo-submodule b/repo-submodule
  new file mode 160000
  index 0000000..de0c53c
  --- /dev/null
  +++ b/repo-submodule
  @@ -0,0 +1 @@
  +Subproject commit de0c53cc213a98b1382aec1dcbcb01bf088273e4



# Import it into Mononoke
  $ cd "$TESTTMP"
  $ gitimport "$GIT_REPO" --generate-bookmarks --discard-submodules full-repo
  [INFO] using repo "repo" repoid RepositoryId(0)
  [INFO] GitRepo:$TESTTMP/repo-git commit 3 of 3 - Oid:fbae2e73 => Bid:4cd77220
  [INFO] Ref: "refs/heads/master_bookmark": Some(ChangesetId(Blake2(4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e)))
  [INFO] Ref: "refs/tags/empty_tag": Some(ChangesetId(Blake2(4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e)))
  [INFO] Ref: "refs/tags/first_tag": Some(ChangesetId(Blake2(032cd4dce0406f1c1dd1362b6c3c9f9bdfa82f2fc5615e237a890be4fe08b044)))
  [INFO] Initializing repo: repo
  [INFO] Initialized repo: repo
  [INFO] All repos initialized. It took: * seconds (glob)
  [INFO] Bookmark: "heads/master_bookmark": ChangesetId(Blake2(4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e)) (created)
  [INFO] Bookmark: "tags/empty_tag": ChangesetId(Blake2(4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e)) (created)
  [INFO] Bookmark: "tags/first_tag": ChangesetId(Blake2(032cd4dce0406f1c1dd1362b6c3c9f9bdfa82f2fc5615e237a890be4fe08b044)) (created)

# We can see that the bonsai changesets graph we created looks correct
  $ mononoke_admin changelog -R repo graph -i 4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e -M -I
  o  message: Add a new submodule
  │  , id: 4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e
  │
  o  message: Add file2
  │  , id: da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c
  │
  o  message: Add file1
     , id: 032cd4dce0406f1c1dd1362b6c3c9f9bdfa82f2fc5615e237a890be4fe08b044

# Look at the commit that introduced the submodule:
# While the edit to the normal file: `.gitmodules` is preserved, the addition of the submodule itself was removed
# from the commit at import time.
  $ mononoke_admin fetch -R repo -i 4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e
  BonsaiChangesetId: 4cd77220f6dcf9154b8cd4dc0f33b72b19a765d73a770cce612ee094191e7d9e
  Author: mononoke <mononoke@mononoke>
  Message: Add a new submodule
  
  FileChanges:
  	 ADDED/MODIFIED: .gitmodules 2b6c6b2889a7b56e04e646b89d236f31552baf7271957b807b84cb0b77fa9e1d
  


# Generate Git Bundle for the submodule discarded repo
  $ mononoke_admin git-bundle create from-repo -R repo --output-location "$BUNDLE_PATH"

# Clone the repo from the bundle
  $ cd $TESTTMP
  $ git clone "$BUNDLE_PATH" repo_from_bundle
  Cloning into 'repo_from_bundle'...

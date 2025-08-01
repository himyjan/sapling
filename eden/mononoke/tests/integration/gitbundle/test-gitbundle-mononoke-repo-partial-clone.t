# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"
  $ REPOTYPE="blob_files"
  $ setup_common_config $REPOTYPE
  $ GIT_REPO_ORIGIN="${TESTTMP}/origin/repo-git"
  $ GIT_REPO="${TESTTMP}/repo-git"
  $ HG_REPO="${TESTTMP}/repo"
  $ BUNDLE_PATH="${TESTTMP}/repo_bundle.bundle"

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
  $ git tag -a empty_tag -m ""
  $ cd "$TESTTMP"
  $ git clone "$GIT_REPO_ORIGIN"
  Cloning into 'repo-git'...
  done.
  $ cd repo-git
  $ git fetch "$GIT_REPO_ORIGIN" +refs/*:refs/* --prune -u
  From $TESTTMP/origin/repo-git
   - [deleted]         (none)     -> origin/master_bookmark
     (refs/remotes/origin/HEAD has become dangling)
  $ cd ..

# Import it into Mononoke
  $ cd "$TESTTMP"
  $ gitimport "$GIT_REPO" --derive-hg --generate-bookmarks full-repo
  [INFO] using repo "repo" repoid RepositoryId(0)
  [INFO] GitRepo:$TESTTMP/repo-git commit 2 of 2 - Oid:e8615d6f => Bid:da93dc81
  [INFO] Hg: Sha1(8ce3eae44760b500bf3f2c3922a95dcd3c908e9e): HgManifestId(HgNodeHash(Sha1(009adbc8d457927d2e1883c08b0692bc45089839)))
  [INFO] Hg: Sha1(e8615d6f149b876be0a2f30a1c5bf0c42bf8e136): HgManifestId(HgNodeHash(Sha1(d92f8d2d10e61e62f65acf25cdd638ea214f267f)))
  [INFO] Ref: "refs/heads/master_bookmark": Some(ChangesetId(Blake2(da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c)))
  [INFO] Ref: "refs/tags/empty_tag": Some(ChangesetId(Blake2(da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c)))
  [INFO] Ref: "refs/tags/first_tag": Some(ChangesetId(Blake2(032cd4dce0406f1c1dd1362b6c3c9f9bdfa82f2fc5615e237a890be4fe08b044)))
  [INFO] Initializing repo: repo
  [INFO] Initialized repo: repo
  [INFO] All repos initialized. It took: * seconds (glob)
  [INFO] Bookmark: "heads/master_bookmark": ChangesetId(Blake2(da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c)) (created)
  [INFO] Bookmark: "tags/empty_tag": ChangesetId(Blake2(da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c)) (created)
  [INFO] Bookmark: "tags/first_tag": ChangesetId(Blake2(032cd4dce0406f1c1dd1362b6c3c9f9bdfa82f2fc5615e237a890be4fe08b044)) (created)

# Regenerate the Git repo out of the Mononoke repo
  $ mononoke_admin git-bundle create from-repo -R repo --output-location "$BUNDLE_PATH"

# Create a new empty folder for containing the repo
  $ mkdir $TESTTMP/git_client_repo
  $ cd "$TESTTMP"
  $ git clone "$BUNDLE_PATH" git_client_repo
  Cloning into 'git_client_repo'...
  $ cd git_client_repo
  $ git fetch "$BUNDLE_PATH" +refs/*:refs/* --prune -u
  From $TESTTMP/repo_bundle.bundle
   - [deleted]         (none)     -> origin/master_bookmark
     (refs/remotes/origin/HEAD has become dangling)
  $ cd ..


# Add more commits to the Git repo
  $ cd "$GIT_REPO_ORIGIN"
  $ echo "this is file3" > file3
  $ git add .
  $ git commit -q -am "Add file3"
  $ git tag -a file3_tag -m "Tag for file 3"
  $ echo "this is file4" > file4
  $ git add .
  $ git commit -q -am "Add file4"
  $ git tag -a file4_tag -m "Tag for file 4"
  $ cd $GIT_REPO
  $ git pull -q --tags
  $ git fetch "$GIT_REPO_ORIGIN" +refs/*:refs/* --prune -u
  From $TESTTMP/origin/repo-git
   - [deleted]         (none)     -> origin/master_bookmark
     (refs/remotes/origin/HEAD has become dangling)

# Capture all the known Git objects from the repo
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/object_list

# Get the repository log
  $ git log --pretty=format:"%h %an %s %D"
  2fffba3 mononoke Add file4 HEAD -> master_bookmark, tag: file4_tag
  bfc616e mononoke Add file3 tag: file3_tag
  e8615d6 mononoke Add file2 tag: empty_tag
  8ce3eae mononoke Add file1 tag: first_tag (no-eol)

# Import the new commits into Mononoke
  $ cd "$TESTTMP"
  $ gitimport "$GIT_REPO" --derive-hg --generate-bookmarks full-repo
  [INFO] using repo "repo" repoid RepositoryId(0)
  [INFO] GitRepo:$TESTTMP/repo-git 2 of 4 commit(s) already exist
  [INFO] GitRepo:$TESTTMP/repo-git commit 4 of 4 - Oid:2fffba32 => Bid:9a3b8a37
  [INFO] Hg: Sha1(8ce3eae44760b500bf3f2c3922a95dcd3c908e9e): HgManifestId(HgNodeHash(Sha1(009adbc8d457927d2e1883c08b0692bc45089839)))
  [INFO] Hg: Sha1(e8615d6f149b876be0a2f30a1c5bf0c42bf8e136): HgManifestId(HgNodeHash(Sha1(d92f8d2d10e61e62f65acf25cdd638ea214f267f)))
  [INFO] Hg: Sha1(bfc616ee3980b461c7d38caa901fa591aa776631): HgManifestId(HgNodeHash(Sha1(8d973572ed46d7564f2e6d8e92772349ba328410)))
  [INFO] Hg: Sha1(2fffba32d46eadef1e064bce2f109f248da94cd1): HgManifestId(HgNodeHash(Sha1(598aee31ec54105c746ec33fdd19fc5980d04014)))
  [INFO] Ref: "refs/heads/master_bookmark": Some(ChangesetId(Blake2(9a3b8a37081bad8b5abdefbe01391b4960afcb329164cc03e3b52161912fd764)))
  [INFO] Ref: "refs/tags/empty_tag": Some(ChangesetId(Blake2(da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c)))
  [INFO] Ref: "refs/tags/file3_tag": Some(ChangesetId(Blake2(e70bae430caf70f469dd5517d77f396290a0cad67af436397849f01075413ed2)))
  [INFO] Ref: "refs/tags/file4_tag": Some(ChangesetId(Blake2(9a3b8a37081bad8b5abdefbe01391b4960afcb329164cc03e3b52161912fd764)))
  [INFO] Ref: "refs/tags/first_tag": Some(ChangesetId(Blake2(032cd4dce0406f1c1dd1362b6c3c9f9bdfa82f2fc5615e237a890be4fe08b044)))
  [INFO] Initializing repo: repo
  [INFO] Initialized repo: repo
  [INFO] All repos initialized. It took: * seconds (glob)
  [INFO] Bookmark: "heads/master_bookmark": ChangesetId(Blake2(9a3b8a37081bad8b5abdefbe01391b4960afcb329164cc03e3b52161912fd764)) (moved from ChangesetId(Blake2(da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c)))
  [INFO] Bookmark: "tags/empty_tag": ChangesetId(Blake2(da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c)) (already up-to-date)
  [INFO] Bookmark: "tags/file3_tag": ChangesetId(Blake2(e70bae430caf70f469dd5517d77f396290a0cad67af436397849f01075413ed2)) (created)
  [INFO] Bookmark: "tags/file4_tag": ChangesetId(Blake2(9a3b8a37081bad8b5abdefbe01391b4960afcb329164cc03e3b52161912fd764)) (created)
  [INFO] Bookmark: "tags/first_tag": ChangesetId(Blake2(032cd4dce0406f1c1dd1362b6c3c9f9bdfa82f2fc5615e237a890be4fe08b044)) (already up-to-date)

# We already have the repo generated uptill da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c from the earlier clone from bundle
# Let's generate a bundle of the partial state of the repo after the known head
  $ mononoke_admin git-bundle create from-repo -R repo --output-location "$BUNDLE_PATH" --have-heads da93dc81badd8d407db0f3219ec0ec78f1ef750ebfa95735bb483310371af80c
# Ensure that Git considers this a valid bundle
  $ cd $GIT_REPO
  $ git bundle verify -q $BUNDLE_PATH
  $TESTTMP/repo_bundle.bundle is okay

# Create a new empty folder for containing the repo
  $ cd $TESTTMP/git_client_repo
  $ git pull -q "$BUNDLE_PATH" --tags

# Get the repository log and verify if its the same as earlier. NOTE: The origin (remote) refs will not
# match because currently the bundle only updates local branches/tags. Future changes will (optionally) include remote
# branches as well
  $ git log --pretty=format:"%h %an %s %D"
  2fffba3 mononoke Add file4 HEAD -> master_bookmark, tag: file4_tag
  bfc616e mononoke Add file3 tag: file3_tag
  e8615d6 mononoke Add file2 tag: empty_tag
  8ce3eae mononoke Add file1 tag: first_tag (no-eol)

# Dump all the known Git objects into a file
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/new_object_list

# Ensure that there are no differences between the set of objects by diffing both object list files
  $ diff -w $TESTTMP/new_object_list $TESTTMP/object_list

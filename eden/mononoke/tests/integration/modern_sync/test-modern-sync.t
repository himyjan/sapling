# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

  $ cat >> "$ACL_FILE" << ACLS
  > {
  >   "repos": {
  >     "orig": {
  >       "actions": {
  >         "read": ["$CLIENT0_ID_TYPE:$CLIENT0_ID_DATA", "X509_SUBJECT_NAME:CN=localhost,O=Mononoke,C=US,ST=CA", "X509_SUBJECT_NAME:CN=client0,O=Mononoke,C=US,ST=CA"],
  >         "write": ["$CLIENT0_ID_TYPE:$CLIENT0_ID_DATA", "X509_SUBJECT_NAME:CN=localhost,O=Mononoke,C=US,ST=CA", "X509_SUBJECT_NAME:CN=client0,O=Mononoke,C=US,ST=CA"],
  >         "bypass_readonly": ["$CLIENT0_ID_TYPE:$CLIENT0_ID_DATA", "X509_SUBJECT_NAME:CN=localhost,O=Mononoke,C=US,ST=CA", "X509_SUBJECT_NAME:CN=client0,O=Mononoke,C=US,ST=CA"]
  >       }
  >     },
  >     "dest": {
  >       "actions": {
  >         "read": ["$CLIENT0_ID_TYPE:$CLIENT0_ID_DATA","SERVICE_IDENTITY:server", "X509_SUBJECT_NAME:CN=localhost,O=Mononoke,C=US,ST=CA", "X509_SUBJECT_NAME:CN=client0,O=Mononoke,C=US,ST=CA"],
  >         "write": ["$CLIENT0_ID_TYPE:$CLIENT0_ID_DATA","SERVICE_IDENTITY:server", "X509_SUBJECT_NAME:CN=localhost,O=Mononoke,C=US,ST=CA", "X509_SUBJECT_NAME:CN=client0,O=Mononoke,C=US,ST=CA"],
  >          "bypass_readonly": ["$CLIENT0_ID_TYPE:$CLIENT0_ID_DATA","SERVICE_IDENTITY:server", "X509_SUBJECT_NAME:CN=localhost,O=Mononoke,C=US,ST=CA", "X509_SUBJECT_NAME:CN=client0,O=Mononoke,C=US,ST=CA"]
  >       }
  >     }
  >   }
  > }
  > ACLS

  $ REPOID=0 REPONAME=orig ACL_NAME=orig setup_common_config 
  $ REPOID=1 REPONAME=dest ACL_NAME=dest setup_common_config 

  $ start_and_wait_for_mononoke_server

  $ hg clone -q mono:orig orig
  $ cd orig
  $ drawdag << EOS
  > E # E/dir1/dir2/fifth = abcdefg\n
  > |
  > D # D/dir1/dir2/forth = abcdef\n
  > |
  > C # C/dir1/dir2/third = abcde\n (copied from dir1/dir2/first)
  > |
  > B # B/dir1/dir2/second = abcd\n
  > |
  > A # A/dir1/dir2/first = abc\n
  > EOS


  $ hg goto A -q
  $ hg push -r . --to master_bookmark -q --create

  $ hg goto E -q
  $ hg push -r . --to master_bookmark -q

  $ hg log > $TESTTMP/hglog.out

Sync all bookmarks moves
  $ with_stripped_logs mononoke_modern_sync sync-once orig dest --start-id 0 
  Running sync-once loop
  Connecting to https://localhost:$LOCAL_PORT/edenapi/
  Health check outcome: Ok(ResponseMeta { version: HTTP/2.0, status: 200, server: Some("edenapi_server"), request_id: Some("*"), tw_task_handle: None, tw_task_version: None, tw_canary_id: None, server_load: Some(1), content_length: Some(10), content_encoding: None, mononoke_host: Some("*") }) (glob)
  Processing commit ChangesetId(Blake2(53b034a90fe3002a707a7da9cdf6eac3dea460ad72f7c6969dfb88fd0e69f856))
  Uploading contents: [ContentId(ContentId("*")), ContentId(ContentId("*"))] (glob)
  Upload response: [UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }, UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }] (glob)
  Upload tree response: [UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("c1afe800646ee45232ab5e70c57247b78dbf3899")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("53b19c5f23977836390e5880ec30fd252a311384")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("dbdaef03fd04c4a28dc29fb3fbe10c5ed7a090ec")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload filenodes response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("005d992c5dcf32993668f7cede29d296c494a5d9")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("f9304d84edb8a8ee2d3ce3f9de3ea944c82eba8f")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload hg changeset response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgChangesetId(HgId("e20237022b1290d98c3f14049931a8f498c18c53")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Move bookmark response SetBookmarkResponse { data: Ok(()) }
  Processing commit ChangesetId(Blake2(8a9d572a899acdef764b88671c24b94a8b0780c1591a5a9bca97184c2ef0f304))
  Uploading contents: [ContentId(ContentId("*")), ContentId(ContentId("*"))] (glob)
  Upload response: [UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }, UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("55662471e2a28db8257939b2f9a2d24e65b46a758bac12914a58f17dcde6905f"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }] (glob)
  Upload tree response: [UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("83af7e770afc39d483b9cd198c49fe919ef0461a")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("0652870aff7b4cb5e2172325519652378ae063e7")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("d7134cdd147e3dabe516366caee43b4622260069")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload filenodes response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("35e7525ce3a48913275d7061dd9a867ffef1e34d")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("778675f9ec8d35ff2fce23a34f68edd15d783853")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload hg changeset response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgChangesetId(HgId("5a95ef0f59a992dcb5385649217862599de05565")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Processing commit ChangesetId(Blake2(41deea4804cd27d1f4efbec135d839338804a5dfcaf364863bd0289067644db5))
  Uploading contents: [ContentId(ContentId("*")), ContentId(ContentId("*"))] (glob)
  Upload response: [UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }, UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }] (glob)
  Upload tree response: [UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("144ae30be86d40d8a0617b7ec37a70e618df4840")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("3b6d87c4e93a918020513a57279573f4325109ef")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("b6c10150f12d2cc4c2d3e6bef65767962032e49f")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload filenodes response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("a2e456504a5e61f763f1a0b36a6c247c7541b2b3")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("9bad1c227e9133a5bbae1652c889406d35e6dac1")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload hg changeset response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgChangesetId(HgId("fc03e5f3125836eb107f2fa5b070f841d0b62b85")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Processing commit ChangesetId(Blake2(ba1a2b3ca64cead35117cb2b707da1211cf43639ade917aee655f3875f4922c3))
  Uploading contents: [ContentId(ContentId("*")), ContentId(ContentId("*"))] (glob)
  Upload response: [UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }, UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }] (glob)
  Upload tree response: [UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("553b84eb92dd53cf5d757e536be1b42e46458017")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("fd1a9570853c1a068efbf6175c547a554015f850")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("f0c9107712ba849f0bc585147093c0776cca8247")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload filenodes response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("4eec8cfdabce9565739489483b6ad93ef7657ea9")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("aae2838d921bcc14ccbb9212f4175f300fd9f2f8")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload hg changeset response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgChangesetId(HgId("2571175c538cc794dc974c705fcb12bc848efab4")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Processing commit ChangesetId(Blake2(5b1c7130dde8e54b4285b9153d8e56d69fbf4ae685eaf9e9766cc409861995f8))
  Uploading contents: [ContentId(ContentId("*")), ContentId(ContentId("*"))] (glob)
  Upload response: [UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }, UploadToken { data: UploadTokenData { id: AnyFileContentId(ContentId(ContentId("*"))), bubble_id: None, metadata: Some(FileContentTokenMetadata(FileContentTokenMetadata { content_size: * })) }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } }] (glob)
  Upload tree response: [UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("5e3e3ee682cdb8a61b7537cfc1a821b6283c8bb5")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("33ac88b3b11b11c3fd33fe71cec4c8852ba2eeef")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTreeResponse { token: UploadToken { data: UploadTokenData { id: HgTreeId(HgId("d6c162600b768f8478ea0557302b6027ed43105d")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload filenodes response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("dba92ad67dc1f3732ab73a5f51b77129275a1724")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }, UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgFilenodeId(HgId("b31c6c30a54b89020d5ac28a67917349512d75eb")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Upload hg changeset response: [UploadTokensResponse { token: UploadToken { data: UploadTokenData { id: HgChangesetId(HgId("8c3947e5d8bd4fe70259eca001b8885651c75850")), bubble_id: None, metadata: None }, signature: UploadTokenSignature { signature: [102, 97, 107, 101, 116, 111, 107, 101, 110, 115, 105, 103, 110, 97, 116, 117, 114, 101] } } }]
  Move bookmark response SetBookmarkResponse { data: Ok(()) }

  $ mononoke_admin mutable-counters --repo-name orig get modern_sync
  Some(2)
  $ cat  $TESTTMP/modern_sync_scuba_logs | jq | rg "start_id|dry_run|repo"
      "start_id": 0,
      "dry_run": "false",
      "repo": "orig"* (glob)

  $ cd ..

  $ hg clone -q mono:dest dest --noupdate
  $ cd dest
  $ hg pull 
  pulling from mono:dest

  $ hg log > $TESTTMP/hglog2.out
  $ hg up master_bookmark 
  10 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ ls dir1/dir2
  fifth
  first
  forth
  second
  third

  $ diff  $TESTTMP/hglog.out  $TESTTMP/hglog2.out 

  $ mononoke_admin repo-info  --repo-name dest --show-commit-count
  Repo: dest
  Repo-Id: 1
  Main-Bookmark: master (not set)
  Commits: 5 (Public: 0, Draft: 5)

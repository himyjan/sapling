# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

setup configuration
  $ BLOB_TYPE="blob_sqlite" default_setup_drawdag
  A=aa53d24251ff3f54b1b2c29ae02826701b2abeb0079f1bb13b8434b54cd87675
  B=f8c75e41a0c4d29281df765f39de47bca1dcadfdc55ada4ccc2f6df567201658
  C=e32a1e342cdb1e38e88466b4c1a01ae9f410024017aa21dc0a1c5da6b3963bf2

Check that sqlblob has some data big enough to form a chunk
  $ for s in 0 1; do sqlite3 -readonly "$TESTTMP/blobstore/blobs/shard_${s}.sqlite" "SELECT COUNT(1) FROM chunk" ; done
  3
  5

Check that chunk_generations populated from put and they have length
  $ for s in 0 1; do sqlite3 -readonly "$TESTTMP/blobstore/blobs/shard_${s}.sqlite" "SELECT COUNT(1), last_seen_generation, value_len FROM chunk_generation" | sed "s/^/$s /"; done
  0 3|2|375
  1 5|2|274

Run sqlblob_gc generation size report
  $ mononoke_sqlblob_gc --storage-config-name=blobstore --shard-count=2 generation-size 2>&1 | strip_glog
  Generation | Size
  -----------------
           2 | 2.1 kiB
  Total size: 2.1 kiB

Run sqlblob_gc generation size report again, just to check mark has not broken it
Run sqlblob_gc mark
  $ mononoke_sqlblob_gc --storage-config-name=blobstore --shard-count=2 mark 2>&1 | strip_glog
  Starting initial generation set
  Completed initial generation handling on shard * (glob)
  Completed initial generation handling on shard * (glob)
  Completed initial generation set
  Starting marking generation 1
  Starting mark on data keys from shard 0
  Starting mark on data keys from shard 1
  Completed marking generation 1

Run sqlblob_gc generation size report again, just to check mark has not broken it
  $ mononoke_sqlblob_gc --storage-config-name=blobstore --shard-count=2 --scuba-log-file scuba.json generation-size 2>&1 | strip_glog
  Generation | Size
  -----------------
           2 | 2.1 kiB
  Total size: 2.1 kiB

Check the sizes are logged
  $ jq -r '.int | [ .shard, .generation, .size, .chunk_id_count, .storage_total_footprint ] | @csv' < scuba.json | sort
  ,,,,2160
  0,2,887,3,
  1,2,1273,5,

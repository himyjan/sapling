/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use staticconfig::StaticConfig;
use staticconfig::static_config;

/// Config loaded only for the Sapling identity.
/// This config contains behavior changes when running "sl" vs "hg", so normally this is
/// _not_ where you want to add default config values.
pub static CONFIG: StaticConfig = static_config!("builtin:sapling" => r###"
[remotefilelog]
# Internally this will be overridden by dynamic config to be ~/.hgcache.
cachepath=~/.sl_cache

[committemplate]
changeset={if(desc, desc, emptymsg)}\n
 SL: Enter commit message.  Lines beginning with 'SL:' are removed.
 SL: Leave message empty to abort commit.
 SL: --
 SL: user: {author}\n{ifgt(parents|count, 1,
 "SL: merging:\n{parents % 'SL:   {node|short}: {desc|firstline}\n'}")
 }{if(currentbookmark,
 "SL: bookmark '{currentbookmark}'\n")}{
 filechanges}{
 if(advice, advice, defaultadvice)}

defaultadvice = SL: --
    SL: Consider onboarding Jellyfish in this repo to speed up your workflow.
    SL: Learn how at https://fburl.com/jf-onboard\n

filechangesplain={
 file_adds % "SL: added {file}\n"}{
 file_mods % "SL: changed {file}\n"}{
 file_dels % "SL: removed {file}\n"}{
 if(files, "", "SL: no files changed\n")}
"###);

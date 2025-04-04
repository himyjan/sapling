# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2.

# reset git environment variables
export GIT_AUTHOR_NAME="test"
export GIT_AUTHOR_EMAIL="test@example.org"
export GIT_AUTHOR_DATE="2007-01-01 00:00:10 +0000"
export GIT_COMMITTER_NAME="test"
export GIT_COMMITTER_EMAIL="test@example.org"
export GIT_COMMITTER_DATE="2007-01-01 00:00:10 +0000"
unset GIT_DIR

# preserve test compatibility
setconfig git.committer='test'
setconfig git.committer-date='0 0'

# Avoid local clone error "fatal: transport 'file' not allowed" in new Git versions (see CVE-2022-39253).
export XDG_CONFIG_HOME="$TESTTMP"
git config --global protocol.file.allow always

# Silent "warning: in the working copy of 'foo', LF will be replaced by CRLF the next time Git touches it" on Windows.
git config --global core.autocrlf false

# Support long paths on Windows.
git config --global core.longpaths true

# Undo default_hgrc.py defaults we don't want.
setconfig remotenames.selectivepulldefault=main,master

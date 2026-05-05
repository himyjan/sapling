# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2.

# pyre-strict

import pytest


@pytest.fixture(scope="module")
def my_fixture() -> int:
    return 123

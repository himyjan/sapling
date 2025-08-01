/**
 * Copyright (c) 2013-present, Facebook, Inc.
 */
namespace cpp2 facebook.network.thrift
namespace cpp facebook.network.thrift
namespace py facebook.network.Address

include "thrift/annotation/cpp.thrift"
include "thrift/annotation/thrift.thrift"

# fbstring uses the small internal buffer to store the data
# if the data is small enough (< 24 bytes).
@cpp.Type{name = "::folly::fbstring"}
typedef binary fbbinary

enum AddressType {
  VUNSPEC = 0,
  V4 = 1,
  V6 = 2,
}

struct Address {
  1: required string addr;
  2: required AddressType type;
  @thrift.AllowUnsafeOptionalCustomDefaultValue
  3: optional i64 port = 0;
}

struct BinaryAddress {
  1: required fbbinary addr;
  @thrift.AllowUnsafeOptionalCustomDefaultValue
  2: optional i64 port = 0;
  3: optional string ifName;
}

struct IPPrefix {
  1: required BinaryAddress prefixAddress;
  2: required i16 prefixLength;
}

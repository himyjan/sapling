/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use clientinfo::ClientRequestInfo;
use fbinit::FacebookInit;

/// Provides data and objects needed to log SQL query telemetry, e.g.
/// client request info, scuba logger.
#[derive(Clone, Debug)]
pub struct SqlTelemetryLogger {
    /// Provides client request info so that client correlator can be attached
    /// to the query.
    client_request_info: Option<ClientRequestInfo>,

    /// fbinit to create a scuba logger
    fb: Option<FacebookInit>,
}

impl SqlTelemetryLogger {
    pub fn empty() -> Self {
        Self {
            client_request_info: None,
            fb: None,
        }
    }

    pub fn new(client_request_info: Option<ClientRequestInfo>, fb: Option<FacebookInit>) -> Self {
        Self {
            client_request_info,
            fb,
        }
    }

    pub fn client_request_info(&self) -> Option<&ClientRequestInfo> {
        self.client_request_info.as_ref()
    }

    pub fn fb(&self) -> &Option<FacebookInit> {
        &self.fb
    }
}

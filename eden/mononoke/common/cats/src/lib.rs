/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::Error;
use cats_constants::X_AUTH_CATS_HEADER;
use fbinit::FacebookInit;
use http::HeaderMap;
use metaconfig_types::Identity;
use permission_checker::MononokeIdentitySet;
use tracing::debug;
use tracing::warn;

#[cfg(not(fbcode_build))]
pub fn try_get_cats_idents(
    _fb: FacebookInit,
    _headers: &HeaderMap,
    _verifier_identity: &Identity,
) -> Option<MononokeIdentitySet> {
    None
}

/// Extract identities from CAT tokens in the request headers.
///
/// Returns `None` when no CAT header is present or when the header itself is
/// malformed (e.g. invalid base64). When the header parses, returns
/// `Some(set)` containing the identities of every token that successfully
/// verified — invalid tokens are skipped individually with a warn log so a
/// single bad token does not invalidate the whole header. All error cases are
/// logged inside this function; callers see only the resulting set.
#[cfg(fbcode_build)]
pub fn try_get_cats_idents(
    fb: FacebookInit,
    headers: &HeaderMap,
    verifier_identity: &Identity,
) -> Option<MononokeIdentitySet> {
    match parse_cat_token_list(headers) {
        Ok(None) => None,
        Ok(Some(cat_list)) => Some(verify_cat_tokens(fb, cat_list, verifier_identity)),
        Err(e) => {
            warn!(
                "Error extracting CATs identities: {}. Ignoring CAT token.",
                e
            );
            None
        }
    }
}

#[cfg(fbcode_build)]
fn parse_cat_token_list(
    headers: &HeaderMap,
) -> Result<Option<cryptocat::CryptoAuthTokenList>, Error> {
    let cats = match headers.get(X_AUTH_CATS_HEADER) {
        Some(cats) => cats,
        None => {
            debug!("CAT extraction: no {} header present", X_AUTH_CATS_HEADER);
            return Ok(None);
        }
    };
    let s_cats = cats.to_str()?;
    let cat_list = cryptocat::deserialize_crypto_auth_tokens(s_cats)?;
    debug!(
        "CAT extraction: received {} token(s) in {} header",
        cat_list.tokens.len(),
        X_AUTH_CATS_HEADER,
    );
    Ok(Some(cat_list))
}

#[cfg(fbcode_build)]
fn verify_cat_tokens(
    fb: FacebookInit,
    cat_list: cryptocat::CryptoAuthTokenList,
    verifier_identity: &Identity,
) -> MononokeIdentitySet {
    let svc_scm_ident = cryptocat::Identity {
        id_type: verifier_identity.id_type.clone(),
        id_data: verifier_identity.id_data.clone(),
        ..Default::default()
    };
    cat_list
        .tokens
        .into_iter()
        .filter_map(|token| {
            extract_identity_from_token(fb, &svc_scm_ident, verifier_identity, token)
        })
        .collect()
}

#[cfg(fbcode_build)]
fn extract_identity_from_token(
    fb: FacebookInit,
    svc_scm_ident: &cryptocat::Identity,
    verifier_identity: &Identity,
    token: cryptocat::CryptoAuthToken,
) -> Option<permission_checker::MononokeIdentity> {
    let tdata = match cryptocat::deserialize_crypto_auth_token_data(
        &token.serializedCryptoAuthTokenData[..],
    ) {
        Ok(tdata) => tdata,
        Err(e) => {
            warn!("CAT token skipped: failed to deserialize token data: {}", e);
            return None;
        }
    };

    if tdata.verifierIdentity.id_type != verifier_identity.id_type
        || tdata.verifierIdentity.id_data != verifier_identity.id_data
    {
        debug!(
            "CAT token skipped: verifier identity mismatch (token has {}:{}, expected {}:{})",
            tdata.verifierIdentity.id_type,
            tdata.verifierIdentity.id_data,
            verifier_identity.id_type,
            verifier_identity.id_data,
        );
        return None;
    }

    match cryptocat::verify_crypto_auth_token(fb, token, svc_scm_ident, None) {
        Ok(res) if res.code == cryptocat::CATVerificationCode::SUCCESS => {}
        Ok(res) => {
            warn!(
                "CAT token skipped: verification not successful. status code: {:?}",
                res.code,
            );
            return None;
        }
        Err(e) => {
            warn!("CAT token skipped: verification error: {}", e);
            return None;
        }
    }

    debug!(
        "CAT extraction: extracted identity {}:{}",
        tdata.signerIdentity.id_type, tdata.signerIdentity.id_data,
    );
    Some(permission_checker::MononokeIdentity::new(
        tdata.signerIdentity.id_type,
        tdata.signerIdentity.id_data,
    ))
}

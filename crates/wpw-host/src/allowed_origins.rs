/// Allowed extension IDs.
/// In production, these are the Chrome Web Store / Edge Add-ons published IDs.
/// For development, include the unpacked extension ID.
const ALLOWED_EXTENSION_IDS: &[&str] = &[
    // Production IDs (to be filled after extension publication)
    // Development IDs
    "nchbldhidocdgdnpjekahiennehfjlol",
];

/// Check if the calling extension is allowed.
/// Chrome passes the extension ID as the first argument: wpw-host chrome-extension://ID.../
/// Only the strict `chrome-extension://<id>/` form is accepted; any other shape is rejected
/// (no fallback to raw IDs, otherwise a non-browser caller could pass a bare ID and pass).
pub fn is_allowed(caller_arg: &str) -> bool {
    let Some(rest) = caller_arg.strip_prefix("chrome-extension://") else {
        return false;
    };
    let Some(ext_id) = rest.strip_suffix('/') else {
        return false;
    };
    if ext_id.is_empty() || !ext_id.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    ALLOWED_EXTENSION_IDS.contains(&ext_id)
}

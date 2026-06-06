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
pub fn is_allowed(caller_arg: &str) -> bool {
    // Extract extension ID from chrome-extension://ID.../ format
    let id = caller_arg
        .strip_prefix("chrome-extension://")
        .and_then(|s| s.strip_suffix('/'))
        .or_else(|| caller_arg.strip_prefix("chrome-extension://"));
    
    match id {
        Some(ext_id) => ALLOWED_EXTENSION_IDS.contains(&ext_id),
        None => {
            // If not in expected format, check raw arg
            ALLOWED_EXTENSION_IDS.contains(&caller_arg)
        }
    }
}

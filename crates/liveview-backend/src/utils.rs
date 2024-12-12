use serde::Serialize;
use url::Url;

#[derive(Serialize)]
pub(crate) enum MetadataType {
    Url,
    Data,
}

pub(crate) fn extract_metadata_url(url: Url) -> Option<(String, MetadataType)> {
    let scheme = url.scheme();

    if scheme == "http" || scheme == "https" {
        Some((url.to_string(), MetadataType::Url))
    } else if scheme == "ipfs" {
        Some((
            format!("https://ipfs.io/ipfs/{}{}", url.domain()?, url.path()),
            MetadataType::Url,
        ))
    } else if scheme == "data" {
        /* JWT data */
        Some((url.to_string(), MetadataType::Data))
    } else {
        None
    }
}

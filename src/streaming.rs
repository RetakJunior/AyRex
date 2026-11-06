use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamPlatform {
    #[default]
    Kick,
    Twitch,
    Custom,
}

impl StreamPlatform {
    pub fn label(self) -> &'static str {
        match self {
            Self::Kick => "Kick",
            Self::Twitch => "Twitch",
            Self::Custom => "Custom RTMP / SRT",
        }
    }

    pub fn default_server(self) -> &'static str {
        match self {
            Self::Kick => "rtmps://fa723fc1b171.global-contribute.live-video.net:443/app",
            Self::Twitch => "rtmps://live.twitch.tv:443/app",
            Self::Custom => "",
        }
    }

    pub fn max_bitrate_kbps(self) -> u32 {
        match self {
            Self::Kick => 8_000,
            Self::Twitch | Self::Custom => 6_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamMuxer {
    Flv,
    MpegTs,
}

impl StreamMuxer {
    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::Flv => "flv",
            Self::MpegTs => "mpegts",
        }
    }
}

pub fn muxer_for_url(url: &str) -> Result<StreamMuxer, String> {
    let url = url.trim();
    if url.starts_with("rtmp://") || url.starts_with("rtmps://") {
        Ok(StreamMuxer::Flv)
    } else if url.starts_with("srt://") {
        Ok(StreamMuxer::MpegTs)
    } else {
        Err("Yayın adresi rtmp://, rtmps:// veya srt:// ile başlamalı.".to_owned())
    }
}

pub fn redact_transport_urls(text: &str) -> String {
    let mut redacted = String::with_capacity(text.len());
    let mut offset = 0;
    while offset < text.len() {
        let remaining = &text[offset..];
        let next = ["rtmp://", "rtmps://", "srt://"]
            .iter()
            .filter_map(|prefix| remaining.find(prefix).map(|index| (index, *prefix)))
            .min_by_key(|(index, _)| *index);
        let Some((index, _)) = next else {
            redacted.push_str(remaining);
            break;
        };
        redacted.push_str(&remaining[..index]);
        redacted.push_str("<stream address>");
        let url_start = offset + index;
        let mut url_end = text[url_start..]
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | ']' | ')' | ',' | ';' | '|')
            })
            .map(|length| url_start + length)
            .unwrap_or(text.len());
        if text[url_start..url_end].ends_with(':') {
            url_end -= 1;
        }
        offset = url_end.max(url_start + 1);
    }
    redacted
}

pub fn build_stream_url(
    platform: StreamPlatform,
    server: &str,
    key: &str,
    custom_url: &str,
) -> Result<String, String> {
    if platform == StreamPlatform::Custom {
        let url = custom_url.trim();
        muxer_for_url(url)?;
        return Ok(url.to_owned());
    }

    let server = server.trim().trim_end_matches('/');
    let key = key.trim();
    if key.is_empty() {
        return Err(
            "Yayın anahtarı boş. Kick veya Twitch panelinden yayın anahtarını gir.".to_owned(),
        );
    }
    let muxer = muxer_for_url(server)?;
    if muxer != StreamMuxer::Flv {
        return Err("Kick ve Twitch için RTMP/RTMPS sunucu adresi gerekli.".to_owned());
    }

    let Some((scheme, remainder)) = server.split_once("://") else {
        return Err("RTMP sunucu adresi geçersiz.".to_owned());
    };
    let (authority, path) = remainder
        .split_once('/')
        .map_or((remainder, ""), |(authority, path)| (authority, path));
    if authority.is_empty() || authority.contains('@') || authority.contains('?') {
        return Err("Sunucu adresini anahtar olmadan, Kick/Twitch panelinden kopyala.".to_owned());
    }
    if !path.is_empty() && path != "app" {
        return Err("Sunucu adresi /app ile bitmeli; yayın anahtarını ayrı alana gir.".to_owned());
    }

    let encoded_key = percent_encode_path_segment(key);
    Ok(format!("{scheme}://{authority}/app/{encoded_key}"))
}

pub fn split_legacy_stream_url(url: &str) -> (String, Option<String>) {
    let trimmed = url.trim();
    let Some((scheme, remainder)) = trimmed.split_once("://") else {
        return (trimmed.to_owned(), None);
    };
    let Some((authority, path)) = remainder.split_once('/') else {
        return (format!("{scheme}://{remainder}/app"), None);
    };
    let Some(key) = path.strip_prefix("app/") else {
        let normalized_path = path.trim_end_matches('/');
        if normalized_path.is_empty() {
            return (format!("{scheme}://{authority}/app"), None);
        }
        return (trimmed.to_owned(), None);
    };

    if key.is_empty() {
        return (format!("{scheme}://{authority}/app"), None);
    }
    (format!("{scheme}://{authority}/app"), Some(key.to_owned()))
}

pub fn persistable_server(platform: StreamPlatform, url: &str) -> String {
    if platform == StreamPlatform::Custom {
        return StreamPlatform::Kick.default_server().to_owned();
    }

    let (server, key) = split_legacy_stream_url(url);
    if key.is_some() || has_app_path(&server) {
        server
    } else {
        platform.default_server().to_owned()
    }
}

fn has_app_path(url: &str) -> bool {
    let Some((_, remainder)) = url.split_once("://") else {
        return false;
    };
    let Some((authority, path)) = remainder.split_once('/') else {
        return false;
    };
    !authority.is_empty() && path.trim_end_matches('/') == "app"
}

fn percent_encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocols_select_expected_transport_muxer() {
        assert_eq!(
            muxer_for_url("rtmp://host/live/key").unwrap(),
            StreamMuxer::Flv
        );
        assert_eq!(
            muxer_for_url("rtmps://host/live/key").unwrap(),
            StreamMuxer::Flv
        );
        assert_eq!(
            muxer_for_url("srt://host:9000?mode=caller").unwrap(),
            StreamMuxer::MpegTs
        );
        assert!(muxer_for_url("http://host/live").is_err());
    }

    #[test]
    fn kick_and_twitch_targets_append_the_separate_key_to_the_app_path() {
        let kick = build_stream_url(
            StreamPlatform::Kick,
            StreamPlatform::Kick.default_server(),
            "key-123",
            "",
        )
        .unwrap();
        assert!(kick.ends_with("/app/key-123"));

        let twitch = build_stream_url(
            StreamPlatform::Twitch,
            StreamPlatform::Twitch.default_server(),
            "live_123",
            "",
        )
        .unwrap();
        assert_eq!(twitch, "rtmps://live.twitch.tv:443/app/live_123");
    }

    #[test]
    fn stream_keys_are_encoded_and_never_accepted_as_part_of_server_address() {
        let target = build_stream_url(
            StreamPlatform::Kick,
            "rtmps://ingest.example/app",
            "part/with?symbols",
            "",
        )
        .unwrap();
        assert_eq!(target, "rtmps://ingest.example/app/part%2Fwith%3Fsymbols");
        assert!(
            build_stream_url(
                StreamPlatform::Kick,
                "rtmps://ingest.example/app/key",
                "another-key",
                "",
            )
            .is_err()
        );
        assert!(build_stream_url(StreamPlatform::Twitch, "rtmp://host/app", "", "").is_err());
    }

    #[test]
    fn legacy_stream_key_is_separated_from_the_persistable_server() {
        let (server, key) = split_legacy_stream_url("rtmps://ingest.example/app/old-key");
        assert_eq!(server, "rtmps://ingest.example/app");
        assert_eq!(key.as_deref(), Some("old-key"));
    }

    #[test]
    fn persistence_keeps_only_the_server_for_builtin_platforms() {
        assert_eq!(
            persistable_server(
                StreamPlatform::Kick,
                "rtmps://ingest.example/app/secret-key"
            ),
            "rtmps://ingest.example/app"
        );
        assert_eq!(
            persistable_server(
                StreamPlatform::Custom,
                "srt://example:9000?passphrase=secret"
            ),
            StreamPlatform::Kick.default_server()
        );
    }

    #[test]
    fn error_text_does_not_expose_stream_keys() {
        let error = "Error opening output rtmps://ingest.example/app/secret-key: I/O error";
        assert_eq!(
            redact_transport_urls(error),
            "Error opening output <stream address>: I/O error"
        );
    }
}

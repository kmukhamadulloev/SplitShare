//! Conservative preview policy. Active document formats are never served inline.
pub fn preview_mime(name: &str) -> Option<&'static str> {
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "txt" | "md" | "log" | "csv" | "json" | "toml" | "yaml" | "yml" | "xml" | "html"
        | "htm" | "css" | "js" | "ts" | "rs" | "py" | "sh" | "sql" | "ini" | "conf" => {
            "text/plain; charset=utf-8"
        }
        _ => return None,
    })
}

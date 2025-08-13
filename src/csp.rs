use http::HeaderValue;

pub fn build_csp() -> HeaderValue {
    let directives = [
        "script-src 'self' https://accounts.google.com/gsi/client",
        "style-src 'self' https://accounts.google.com/gsi/style 'sha256-ZAdCRDnStGum6I/Iqtz5uunKn4HysAVC/9iXTVObQr8='",
        "connect-src 'self' https://accounts.google.com/gsi/status",
        "frame-src https://accounts.google.com/gsi",
        "default-src 'self' 'unsafe-inline'",
    ];
    HeaderValue::from_str(&(directives.join("; ") + ";")).unwrap()
}

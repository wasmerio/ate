pub fn origin_url(url: &Option<url::Url>, postfix: &str) -> url::Url
{
    origin_url_ext(url, postfix, false)
}

pub fn origin_url_ext(url: &Option<url::Url>, postfix: &str, force_insecure: bool) -> url::Url
{
    let origin = if let Ok(origin) = std::env::var("ORIGIN") {
        if origin.eq_ignore_ascii_case("localhost") {
            "tokera.com".to_string()
        } else {
            origin
        }
    } else {
        "tokera.sh".to_string()
    };

    let scheme = if let Ok(location) = std::env::var("LOCATION") {
        if location.starts_with("http://") || force_insecure {
            "ws".to_string()
        } else {
            "wss".to_string()
        }
        
    } else {
        "ws".to_string()
    };

    match url.clone() {
        Some(a) => a,
        None => url::Url::parse(format!("{}://{}/{}", scheme, origin, postfix).as_str()).unwrap()
    }
}
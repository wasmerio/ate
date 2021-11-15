pub fn redirect_body(target: &str) -> String {
    format!(
        r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01//EN" "http://www.w3.org/TR/html4/strict.dtd">
<html>
  <head>
    <title>Permanent Redirect</title>
    <meta http-equiv="refresh" content="0; url={}">
  </head>
  <body>
    <p>
      The document has been moved to <a href="{}">{}</a>.
    </p>
  </body>
</html>"#,
        target, target, target
    )
}

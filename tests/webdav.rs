mod fixtures;
mod utils;

use fixtures::{server, Error, TestServer, FILES};
use rstest::rstest;
use std::fs;
use std::time::SystemTime;
use xml::escape::escape_str_pcdata;

#[rstest]
fn propfind_dir(server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPFIND", format!("{}dir1", server.url())).send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:href>/dir1/</D:href>"));
    assert!(body.contains("<D:displayname>dir1</D:displayname>"));
    for f in FILES {
        assert!(body.contains(&format!("<D:href>/dir1/{}</D:href>", utils::encode_uri(f))));
        assert!(body.contains(&format!(
            "<D:displayname>{}</D:displayname>",
            escape_str_pcdata(f)
        )));
    }
    Ok(())
}

#[rstest]
fn propfind_dir_depth0(server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPFIND", format!("{}dir1", server.url()))
        .header("depth", "0")
        .send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:href>/dir1/</D:href>"));
    assert!(body.contains("<D:displayname>dir1</D:displayname>"));
    assert_eq!(
        body.lines()
            .filter(|v| *v == "<D:status>HTTP/1.1 200 OK</D:status>")
            .count(),
        1
    );
    Ok(())
}

#[rstest]
fn propfind_dir_depth2(server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPFIND", format!("{}dir1", server.url()))
        .header("depth", "2")
        .send()?;
    assert_eq!(resp.status(), 400);
    let body = resp.text()?;
    assert_eq!(body, "Invalid depth: only 0 and 1 are allowed.");
    Ok(())
}

#[rstest]
fn propfind_404(server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPFIND", format!("{}404", server.url())).send()?;
    assert_eq!(resp.status(), 404);
    Ok(())
}

#[rstest]
fn propfind_double_slash(server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPFIND", server.url()).send()?;
    assert_eq!(resp.status(), 207);
    Ok(())
}

#[rstest]
fn propfind_file(server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPFIND", format!("{}test.html", server.url())).send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:href>/test.html</D:href>"));
    assert!(body.contains("<D:displayname>test.html</D:displayname>"));
    assert_eq!(
        body.lines()
            .filter(|v| *v == "<D:status>HTTP/1.1 200 OK</D:status>")
            .count(),
        1
    );
    Ok(())
}

#[rstest]
fn proppatch_file(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPPATCH", format!("{}test.html", server.url())).send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:href>/test.html</D:href>"));
    Ok(())
}

#[rstest]
fn proppatch_file_set_mtime(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let ts = "Fri, 06 Mar 2026 01:02:03 GMT";
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
<D:set><D:prop><D:getlastmodified>{ts}</D:getlastmodified></D:prop></D:set>
</D:propertyupdate>"#
    );
    let resp = fetch!(b"PROPPATCH", format!("{}test.html", server.url()))
        .header("content-type", "application/xml; charset=utf-8")
        .body(body)
        .send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:getlastmodified/>"));
    assert!(body.contains("<D:status>HTTP/1.1 200 OK</D:status>"));
    let expected = chrono::DateTime::parse_from_rfc2822(ts)?
        .with_timezone(&chrono::Utc)
        .timestamp() as u64;
    let actual = fs::metadata(server.path().join("test.html"))?
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    assert_eq!(actual, expected);
    Ok(())
}

#[cfg(windows)]
#[rstest]
fn proppatch_file_set_creationdate(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let ts = "2026-03-06T01:02:03Z";
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
<D:set><D:prop><D:creationdate>{ts}</D:creationdate></D:prop></D:set>
</D:propertyupdate>"#
    );
    let resp = fetch!(b"PROPPATCH", format!("{}test.html", server.url()))
        .header("content-type", "application/xml; charset=utf-8")
        .body(body)
        .send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:creationdate/>"));
    assert!(body.contains("<D:status>HTTP/1.1 200 OK</D:status>"));
    let expected = chrono::DateTime::parse_from_rfc3339(ts)?
        .with_timezone(&chrono::Utc)
        .timestamp() as i64;
    let actual = fs::metadata(server.path().join("test.html"))?
        .created()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs() as i64;
    assert!((actual - expected).abs() <= 2);
    Ok(())
}

#[cfg(not(windows))]
#[rstest]
fn proppatch_file_set_creationdate_unsupported(
    #[with(&["-A"])] server: TestServer,
) -> Result<(), Error> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
<D:set><D:prop><D:creationdate>2026-03-06T01:02:03Z</D:creationdate></D:prop></D:set>
</D:propertyupdate>"#;
    let resp = fetch!(b"PROPPATCH", format!("{}test.html", server.url()))
        .header("content-type", "application/xml; charset=utf-8")
        .body(body.to_string())
        .send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:creationdate/>"));
    assert!(body.contains("<D:status>HTTP/1.1 403 Forbidden</D:status>"));
    Ok(())
}

#[cfg(not(windows))]
#[rstest]
fn proppatch_file_set_creationdate_and_mtime_ignore_creation(
    #[with(&["-A"])] server: TestServer,
) -> Result<(), Error> {
    let ts = "Fri, 06 Mar 2026 01:02:03 GMT";
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
<D:set>
<D:prop>
<D:creationdate>2026-03-06T01:02:03Z</D:creationdate>
<D:getlastmodified>{ts}</D:getlastmodified>
</D:prop>
</D:set>
</D:propertyupdate>"#
    );
    let resp = fetch!(b"PROPPATCH", format!("{}test.html", server.url()))
        .header("content-type", "application/xml; charset=utf-8")
        .body(body)
        .send()?;
    assert_eq!(resp.status(), 207);
    let body = resp.text()?;
    assert!(body.contains("<D:creationdate/>"));
    assert!(body.contains("<D:getlastmodified/>"));
    assert!(!body.contains("403 Forbidden"));
    let expected = chrono::DateTime::parse_from_rfc2822(ts)?
        .with_timezone(&chrono::Utc)
        .timestamp() as u64;
    let actual = fs::metadata(server.path().join("test.html"))?
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    assert_eq!(actual, expected);
    Ok(())
}

#[rstest]
fn proppatch_404(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"PROPPATCH", format!("{}404", server.url())).send()?;
    assert_eq!(resp.status(), 404);
    Ok(())
}

#[rstest]
fn mkcol_dir(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"MKCOL", format!("{}newdir", server.url())).send()?;
    assert_eq!(resp.status(), 201);
    Ok(())
}

#[rstest]
fn mkcol_not_allow_upload(server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"MKCOL", format!("{}newdir", server.url())).send()?;
    assert_eq!(resp.status(), 403);
    Ok(())
}

#[rstest]
fn mkcol_already_exists(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"MKCOL", format!("{}dir1", server.url())).send()?;
    assert_eq!(resp.status(), 405);
    Ok(())
}

#[rstest]
fn copy_file(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let new_url = format!("{}test2.html", server.url());
    let resp = fetch!(b"COPY", format!("{}test.html", server.url()))
        .header("Destination", &new_url)
        .send()?;
    assert_eq!(resp.status(), 204);
    let resp = reqwest::blocking::get(new_url)?;
    assert_eq!(resp.status(), 200);
    Ok(())
}

#[rstest]
fn copy_not_allow_upload(server: TestServer) -> Result<(), Error> {
    let new_url = format!("{}test2.html", server.url());
    let resp = fetch!(b"COPY", format!("{}test.html", server.url()))
        .header("Destination", &new_url)
        .send()?;
    assert_eq!(resp.status(), 403);
    Ok(())
}

#[rstest]
fn copy_file_404(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let new_url = format!("{}test2.html", server.url());
    let resp = fetch!(b"COPY", format!("{}404", server.url()))
        .header("Destination", &new_url)
        .send()?;
    assert_eq!(resp.status(), 404);
    Ok(())
}

#[rstest]
fn move_file(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let origin_url = format!("{}test.html", server.url());
    let new_url = format!("{}test2.html", server.url());
    let resp = fetch!(b"MOVE", &origin_url)
        .header("Destination", &new_url)
        .send()?;
    assert_eq!(resp.status(), 204);
    let resp = reqwest::blocking::get(new_url)?;
    assert_eq!(resp.status(), 200);
    let resp = reqwest::blocking::get(origin_url)?;
    assert_eq!(resp.status(), 404);
    Ok(())
}

#[rstest]
fn move_not_allow_upload(#[with(&["--allow-delete"])] server: TestServer) -> Result<(), Error> {
    let origin_url = format!("{}test.html", server.url());
    let new_url = format!("{}test2.html", server.url());
    let resp = fetch!(b"MOVE", &origin_url)
        .header("Destination", &new_url)
        .send()?;
    assert_eq!(resp.status(), 403);
    Ok(())
}

#[rstest]
fn move_not_allow_delete(#[with(&["--allow-upload"])] server: TestServer) -> Result<(), Error> {
    let origin_url = format!("{}test.html", server.url());
    let new_url = format!("{}test2.html", server.url());
    let resp = fetch!(b"MOVE", &origin_url)
        .header("Destination", &new_url)
        .send()?;
    assert_eq!(resp.status(), 403);
    Ok(())
}

#[rstest]
fn move_file_404(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let new_url = format!("{}test2.html", server.url());
    let resp = fetch!(b"MOVE", format!("{}404", server.url()))
        .header("Destination", &new_url)
        .send()?;
    assert_eq!(resp.status(), 404);
    Ok(())
}

#[rstest]
fn lock_file(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"LOCK", format!("{}test.html", server.url())).send()?;
    assert_eq!(resp.status(), 200);
    let body = resp.text()?;
    assert!(body.contains("<D:href>/test.html</D:href>"));
    Ok(())
}

#[rstest]
fn lock_file_404(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"LOCK", format!("{}404", server.url())).send()?;
    assert_eq!(resp.status(), 404);
    Ok(())
}

#[rstest]
fn unlock_file(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"LOCK", format!("{}test.html", server.url())).send()?;
    assert_eq!(resp.status(), 200);
    Ok(())
}

#[rstest]
fn unlock_file_404(#[with(&["-A"])] server: TestServer) -> Result<(), Error> {
    let resp = fetch!(b"LOCK", format!("{}404", server.url())).send()?;
    assert_eq!(resp.status(), 404);
    Ok(())
}

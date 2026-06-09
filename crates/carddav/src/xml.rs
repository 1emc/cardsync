use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::{io::Cursor, string::FromUtf8Error};

#[derive(Debug, Clone)]
pub struct DavResource {
    pub href: String,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum XmlBuildError {
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
    #[error(transparent)]
    Utf8(#[from] FromUtf8Error),
}

pub fn multistatus(resources: &[DavResource]) -> Result<String, XmlBuildError> {
    let mut w = Writer::new(Cursor::new(Vec::new()));
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;
    let mut ms = BytesStart::new("D:multistatus");
    ms.push_attribute(("xmlns:D", "DAV:"));
    ms.push_attribute(("xmlns:C", "urn:ietf:params:xml:ns:carddav"));
    w.write_event(Event::Start(ms))?;
    for r in resources {
        write_response(&mut w, r)?;
    }
    w.write_event(Event::End(BytesEnd::new("D:multistatus")))?;
    Ok(String::from_utf8(w.into_inner().into_inner())?)
}

fn write_text(
    w: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    text: &str,
) -> Result<(), quick_xml::Error> {
    w.write_event(Event::Start(BytesStart::new(name)))?;
    w.write_event(Event::Text(BytesText::new(text)))?;
    w.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}
fn empty(w: &mut Writer<Cursor<Vec<u8>>>, name: &str) -> Result<(), quick_xml::Error> {
    w.write_event(Event::Empty(BytesStart::new(name)))?;
    Ok(())
}
fn write_response(
    w: &mut Writer<Cursor<Vec<u8>>>,
    r: &DavResource,
) -> Result<(), quick_xml::Error> {
    w.write_event(Event::Start(BytesStart::new("D:response")))?;
    write_text(w, "D:href", &r.href)?;
    w.write_event(Event::Start(BytesStart::new("D:propstat")))?;
    w.write_event(Event::Start(BytesStart::new("D:prop")))?;
    empty(w, "D:resourcetype")?;
    if let Some(etag) = &r.etag {
        write_text(w, "D:getetag", etag)?;
    }
    if let Some(ct) = &r.content_type {
        write_text(w, "D:getcontenttype", ct)?;
    }
    if let Some(body) = &r.body {
        write_text(w, "C:address-data", body)?;
    }
    w.write_event(Event::End(BytesEnd::new("D:prop")))?;
    write_text(w, "D:status", "HTTP/1.1 200 OK")?;
    w.write_event(Event::End(BytesEnd::new("D:propstat")))?;
    w.write_event(Event::End(BytesEnd::new("D:response")))?;
    Ok(())
}

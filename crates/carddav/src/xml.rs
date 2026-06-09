use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct DavResource {
    pub href: String,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub body: Option<String>,
}

pub fn multistatus(resources: &[DavResource]) -> String {
    let mut w = Writer::new(Cursor::new(Vec::new()));
    let _ = w.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)));
    let mut ms = BytesStart::new("D:multistatus");
    ms.push_attribute(("xmlns:D", "DAV:"));
    ms.push_attribute(("xmlns:C", "urn:ietf:params:xml:ns:carddav"));
    let _ = w.write_event(Event::Start(ms));
    for r in resources {
        write_response(&mut w, r);
    }
    let _ = w.write_event(Event::End(BytesEnd::new("D:multistatus")));
    String::from_utf8(w.into_inner().into_inner()).unwrap_or_default()
}

fn write_text(w: &mut Writer<Cursor<Vec<u8>>>, name: &str, text: &str) {
    let _ = w.write_event(Event::Start(BytesStart::new(name)));
    let _ = w.write_event(Event::Text(BytesText::new(text)));
    let _ = w.write_event(Event::End(BytesEnd::new(name)));
}
fn empty(w: &mut Writer<Cursor<Vec<u8>>>, name: &str) {
    let _ = w.write_event(Event::Empty(BytesStart::new(name)));
}
fn write_response(w: &mut Writer<Cursor<Vec<u8>>>, r: &DavResource) {
    let _ = w.write_event(Event::Start(BytesStart::new("D:response")));
    write_text(w, "D:href", &r.href);
    let _ = w.write_event(Event::Start(BytesStart::new("D:propstat")));
    let _ = w.write_event(Event::Start(BytesStart::new("D:prop")));
    empty(w, "D:resourcetype");
    if let Some(etag) = &r.etag {
        write_text(w, "D:getetag", etag);
    }
    if let Some(ct) = &r.content_type {
        write_text(w, "D:getcontenttype", ct);
    }
    if let Some(body) = &r.body {
        write_text(w, "C:address-data", body);
    }
    let _ = w.write_event(Event::End(BytesEnd::new("D:prop")));
    write_text(w, "D:status", "HTTP/1.1 200 OK");
    let _ = w.write_event(Event::End(BytesEnd::new("D:propstat")));
    let _ = w.write_event(Event::End(BytesEnd::new("D:response")));
}

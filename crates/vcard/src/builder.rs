use domain::Contact;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct VcardOptions {
    pub include_mobile_phone: bool,
}
impl Default for VcardOptions {
    fn default() -> Self {
        Self {
            include_mobile_phone: false,
        }
    }
}

pub fn build_vcard(contact: &Contact, options: &VcardOptions) -> String {
    let mut lines = vec!["BEGIN:VCARD".to_string(), "VERSION:4.0".to_string()];
    push(&mut lines, "UID", &contact.carddav_uid());
    let fn_value = contact
        .display_name
        .clone()
        .or_else(|| name_join(contact.given_name.as_deref(), contact.surname.as_deref()))
        .or_else(|| contact.email.clone())
        .or_else(|| contact.user_principal_name.clone());
    if let Some(v) = fn_value.as_deref() {
        push(&mut lines, "FN", v);
    }
    if contact.given_name.is_some() || contact.surname.is_some() {
        let n = format!(
            "{};{};;;",
            esc_component(contact.surname.as_deref().unwrap_or_default()),
            esc_component(contact.given_name.as_deref().unwrap_or_default())
        );
        lines.push(format!("N:{n}"));
    }
    if let Some(v) = contact
        .email
        .as_deref()
        .or(contact.user_principal_name.as_deref())
    {
        push(&mut lines, "EMAIL;TYPE=work", v);
    }
    if let Some(v) = contact.business_phone.as_deref() {
        push(&mut lines, "TEL;TYPE=work,voice", v);
    }
    if options.include_mobile_phone {
        if let Some(v) = contact.mobile_phone.as_deref() {
            push(&mut lines, "TEL;TYPE=cell,voice", v);
        }
    }
    if let Some(v) = contact.company_name.as_deref() {
        push(&mut lines, "ORG", v);
    }
    if let Some(v) = contact.job_title.as_deref() {
        push(&mut lines, "TITLE", v);
    }
    if let Some(v) = contact.department.as_deref() {
        push(&mut lines, "ROLE", v);
    }
    if let Some(v) = contact.office_location.as_deref() {
        push(&mut lines, "ADR;TYPE=work", &format!(";;;;;;{}", v));
    }
    lines.push("END:VCARD".to_string());
    fold_lines(&lines).join("\r\n") + "\r\n"
}

pub fn contact_etag(contact: &Contact, options: &VcardOptions) -> String {
    let mut h = Sha256::new();
    h.update(build_vcard(contact, options).as_bytes());
    format!("\"{}\"", hex::encode(h.finalize()))
}

fn push(lines: &mut Vec<String>, key: &str, value: &str) {
    if !value.trim().is_empty() {
        lines.push(format!("{key}:{}", esc_value(value)));
    }
}
fn name_join(g: Option<&str>, s: Option<&str>) -> Option<String> {
    let n = [g.unwrap_or(""), s.unwrap_or("")]
        .into_iter()
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!n.is_empty()).then_some(n)
}
fn esc_value(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "")
        .replace(';', "\\;")
        .replace(',', "\\,")
}
fn esc_component(v: &str) -> String {
    esc_value(v)
}
fn fold_lines(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        if line.len() <= 75 {
            out.push(line.clone());
            continue;
        }
        let mut rest = line.as_str();
        let mut first = true;
        while !rest.is_empty() {
            let max = if first { 75 } else { 74 };
            let take = rest
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(rest.len()))
                .take_while(|i| *i <= max)
                .last()
                .unwrap_or(rest.len());
            let (part, r) = rest.split_at(take);
            out.push(if first {
                part.to_string()
            } else {
                format!(" {part}")
            });
            rest = r;
            first = false;
        }
    }
    out
}

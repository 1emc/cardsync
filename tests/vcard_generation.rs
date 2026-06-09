use chrono::Utc;
use domain::Contact;
use uuid::Uuid;

fn contact() -> Contact {
    Contact {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        source: "test".into(),
        source_object_id: "1".into(),
        display_name: Some("Max; Mustermann".into()),
        given_name: Some("Max".into()),
        surname: Some("Mustermann".into()),
        email: Some("max@example.com".into()),
        user_principal_name: None,
        business_phone: Some("+49 123".into()),
        mobile_phone: Some("+49 private".into()),
        job_title: Some("IT, Admin".into()),
        department: None,
        company_name: Some("Example GmbH".into()),
        office_location: None,
        etag: None,
        is_deleted: false,
        is_suppressed: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_seen_at: None,
    }
}

#[test]
fn vcard_contains_work_fields_and_escapes_values() {
    let v = vcard::build_vcard(&contact(), &vcard::VcardOptions::default());
    assert!(v.contains("VERSION:4.0"));
    assert!(v.contains("FN:Max\\; Mustermann"));
    assert!(v.contains("EMAIL;TYPE=work:max@example.com"));
    assert!(v.contains("TEL;TYPE=work,voice:+49 123"));
    assert!(v.contains("TITLE:IT\\, Admin"));
    assert!(!v.contains("+49 private"));
}

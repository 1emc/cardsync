#[test]
fn carddav_password_hash_roundtrip_and_basic_auth() {
    let password = carddav::auth::generate_app_password();
    let hash = carddav::auth::hash_password(&password).expect("hash");
    assert!(carddav::auth::verify_password(&password, &hash));
    assert!(!carddav::auth::verify_password("wrong", &hash));

    let header = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "ios-demo:secret"
        )
    );
    let creds = carddav::auth::parse_basic_auth(&header).expect("basic");
    assert_eq!(creds.username, "ios-demo");
    assert_eq!(creds.password, "secret");
}

#[test]
fn basic_auth_rejects_empty_username_and_accepts_case_insensitive_scheme() {
    let header = format!(
        "bAsIc {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "ios-demo:secret"
        )
    );
    let creds = carddav::auth::parse_basic_auth(&header).expect("basic");
    assert_eq!(creds.username, "ios-demo");
    assert_eq!(creds.password, "secret");

    let empty_user = format!(
        "Basic {}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, ":secret")
    );
    assert!(carddav::auth::parse_basic_auth(&empty_user).is_none());
}

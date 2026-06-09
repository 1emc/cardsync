#[test]
fn normalizes_and_hashes_without_plaintext() {
    let normalized = privacy::normalize_email("  Max.Mustermann@Example.COM ");
    assert_eq!(normalized, "max.mustermann@example.com");
    let a = privacy::suppression_hmac_hex("secret-a", &normalized).expect("valid HMAC key");
    let b = privacy::suppression_hmac_hex("secret-a", &normalized).expect("valid HMAC key");
    let c = privacy::suppression_hmac_hex("secret-b", &normalized).expect("valid HMAC key");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(!a.contains("max.mustermann"));
    assert!(privacy::constant_time_eq(&a, &b));
}

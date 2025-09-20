//! Standalone test that should compile

#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_string() {
    let s = "hello";
    assert_eq!(s.len(), 5);
}

#[test]
fn test_vector() {
    let v = vec![1, 2, 3];
    assert_eq!(v.len(), 3);
}
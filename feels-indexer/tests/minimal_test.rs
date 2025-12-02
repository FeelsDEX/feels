//! Minimal test that should compile and pass

#[test]
fn test_basic_math() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_vector() {
    let v = [1, 2, 3];
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], 1);
}

#[test]
fn test_string() {
    let s = String::from("hello");
    assert_eq!(s, "hello");
}
//! Minimal tests without external dependencies

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert_eq!(1 + 1, 2);
    }
    
    #[test]
    fn test_string() {
        let s = "hello";
        assert_eq!(s.len(), 5);
    }
}
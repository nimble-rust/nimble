use app_version::Version;
use std::str::FromStr;

#[test]
fn to_string() {
    let x = Version::new(1, 23, 46);
    assert_eq!(x.to_string(), "1.23.46");
}

#[test]
fn from_str_valid() {
    let version_str = "1.2.3";
    let version = Version::from_str(version_str).unwrap();
    assert_eq!(version, Version::new(1, 2, 3));
}

#[test]
fn test_from_tuple() {
    let version_tuple = (2, 1, 3);
    let version = Version::from(version_tuple);
    assert_eq!(version, Version::new(2, 1, 3));
}

#[test]
fn test_from_str_invalid_format() {
    let version_str = "1.2";
    let result = Version::from_str(version_str);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Invalid version format");
}

use std::borrow::Cow;

pub fn make_single_line(s: &String) -> Cow<String> {
    if s.contains('\n') {
        Cow::Owned(s.replace('\n', "↵"))
    } else {
        Cow::Borrowed(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_linefeed() {
        let s = "Hello\nWorld!".to_string();
        assert!(s.contains('\n'));
        assert!(!make_single_line(&s).contains('\n'));
    }
}

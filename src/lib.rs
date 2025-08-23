pub trait AsStr {
    fn as_str(&self) -> &str;
}

pub trait Redact: AsStr {
    fn redact(&self) -> String {
        let mut redacted = self.as_str().to_string();
        redacted.replace_range(5.., "********");
        redacted
    }
}

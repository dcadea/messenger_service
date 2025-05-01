pub trait Raw {
    fn raw(&self) -> &str;
}

pub trait Redact: Raw {
    fn redact(&self) -> String {
        let redacted = {
            let mut redacted = self.raw().to_string();
            redacted.replace_range(5.., "********");
            redacted
        };

        redacted
    }
}

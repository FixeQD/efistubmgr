use crate::DESC_PREFIX;

pub fn desc_timestamp(description: &str) -> Option<i64> {
    let rest = description.strip_prefix(DESC_PREFIX)?;
    match rest.trim().parse() {
        Ok(ts) => Some(ts),
        Err(e) => {
            eprintln!(
                "{}: warning: entry {DESC_PREFIX:?}-prefixed but unparsable timestamp {rest:?}: {e}",
                env!("CARGO_PKG_NAME")
            );
            None
        }
    }
}
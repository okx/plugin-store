/// Sanitize API-sourced strings before outputting to agent context.
/// Strips control characters (except newline) and truncates long strings
/// to prevent prompt injection from malicious market data.
const MAX_LEN: usize = 500;

pub fn sanitize_str(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect();
    if cleaned.len() > MAX_LEN {
        format!("{}…", &cleaned[..MAX_LEN])
    } else {
        cleaned
    }
}

pub fn sanitize_opt(s: Option<&str>) -> Option<String> {
    s.map(sanitize_str)
}

pub fn sanitize_opt_owned(s: &Option<String>) -> Option<String> {
    s.as_deref().map(sanitize_str)
}

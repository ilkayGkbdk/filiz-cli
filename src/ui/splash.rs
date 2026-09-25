/// Raw mode disables output post-processing, so a bare `\n` does not return the cursor.
pub fn for_raw_mode(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

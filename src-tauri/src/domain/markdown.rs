//! Transcript markdown rendering (parity with Phoenix Markdown module).

pub fn render(transcript_text: &str, filename: &str, provider: &str) -> String {
    format!(
        "# Transcript\n\n- Source: {filename}\n- Provider: {provider}\n\n{}\n",
        transcript_text.trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_source_and_provider() {
        let md = render("  hello world  ", "clip.wav", "assemblyai");
        assert!(md.contains("# Transcript"));
        assert!(md.contains("Source: clip.wav"));
        assert!(md.contains("Provider: assemblyai"));
        assert!(md.contains("hello world"));
    }
}

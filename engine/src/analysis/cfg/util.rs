pub fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start <= b_end && b_start <= a_end
}

pub fn first_match(input: &str, tokens: &[&str]) -> Option<(String, usize)> {
    let mut best: Option<(String, usize)> = None;
    for token in tokens {
        if let Some(idx) = input.find(token) {
            match &best {
                Some((_, old)) if *old <= idx => {}
                _ => best = Some(((*token).to_string(), idx)),
            }
        }
    }
    best
}

pub fn line_for_byte(source: &str, byte_offset: usize) -> usize {
    let mut row = 0usize;
    let max = byte_offset.min(source.len());
    for &b in source.as_bytes().iter().take(max) {
        if b == b'\n' {
            row += 1;
        }
    }
    row
}

pub fn snippet_around_line(source: &str, line: usize, radius: usize) -> (String, usize, usize) {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return (String::new(), 0, 0);
    }
    let max_idx = lines.len().saturating_sub(1);
    let focus = line.min(max_idx);
    let start = focus.saturating_sub(radius);
    let end = (focus + radius).min(max_idx);
    let snippet = lines[start..=end]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    (snippet, start, end)
}

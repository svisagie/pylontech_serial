use std::fs;
use std::path::Path;

// Extract all blocks that start with '@' line and end with a line containing '$$'
// Returns vector of blocks, each block a Vec<String> (without trimming original lines)
pub fn load_blocks<P: AsRef<Path>>(path: P) -> Vec<Vec<String>> {
    let content = match fs::read_to_string(&path) { Ok(c) => c, Err(_) => return vec![] };
    let mut blocks = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in content.lines() {
        if line.starts_with('@') {
            if let Some(b) = current.take() { if !b.is_empty() { blocks.push(b); } }
            current = Some(vec![line.to_string()]);
        } else if line.contains("$$") {
            if let Some(mut b) = current.take() { b.push(line.to_string()); blocks.push(b); }
        } else if let Some(ref mut b) = current { b.push(line.to_string()); }
    }
    if let Some(b) = current { if !b.is_empty() { blocks.push(b); } }
    blocks
}

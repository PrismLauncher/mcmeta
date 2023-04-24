use anyhow::Result;

fn json_matching_brace(c: char) -> char {
    match c {
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        other => other,
    }
}

/**
 * Attempts to read a complete json object at the error location from the provide body
 * to provided context to a deserialisation error. only usefull if the error was caused
 * by a data mismatch not a syntax error or EOF.
 */
#[allow(dead_code)]
pub fn get_json_context(err: &serde_json::Error, body: &str, max_len: usize) -> String {
    let line_offset = body
        .char_indices()
        .filter(|(_, c)| *c == '\n')
        .nth(err.line() - 1)
        .unwrap_or_default()
        .0;
    let mut ctx = body.split_at(line_offset).1.to_owned();
    let offset = ctx.char_indices().nth(err.column()).unwrap().0;
    ctx = ctx.split_at(offset).1.to_owned();

    let mut token_contexts: Vec<char> = vec![];
    let mut excape_next = false;
    let mut in_str = false;
    let mut found_close = false;
    let mut ctx_end = 0;
    let mut last_char: char = ctx.chars().next().unwrap_or_default();
    for (i, c) in ctx.char_indices() {
        if c == '\\' && in_str && !excape_next {
            excape_next = true;
            continue;
        }
        if c == '"' || c == '\'' {
            if in_str && !excape_next && token_contexts.last() == Some(&c) {
                in_str = false;
                found_close = true;
                token_contexts.pop();
            } else {
                in_str = true;
                token_contexts.push(c);
            }
        }
        if (c == '[' || c == '{') && !in_str {
            token_contexts.push(c);
        }
        if (c == ']' || c == '}')
            && !in_str
            && token_contexts.last() == Some(&json_matching_brace(c))
        {
            token_contexts.pop();
            found_close = true;
        }
        if (c == ',') && !in_str {
            found_close = true;
        }

        if excape_next {
            excape_next = false;
        }
        if found_close && token_contexts.is_empty() {
            ctx_end = i;
            last_char = c;
            break;
        }
    }

    if ctx_end > 0 {
        ctx = ctx[..ctx_end].to_owned() + &last_char.to_string();
    }

    if max_len > 0 && ctx.chars().count() > max_len {
        ctx = ctx
            .split_at(ctx.char_indices().nth(max_len).unwrap().0)
            .0
            .to_owned()
            + " ...";
    }
    ctx
}

/**
 * Attempts to read a complete json object just before the error location from the provide body
 * to provided context to a deserialisation error. only usefull if the error was caused
 * by a data mismatch not a syntax error or EOF.
 */
pub fn get_json_context_back(err: &serde_json::Error, body: &str, max_len: usize) -> String {
    let line_offset = body
        .char_indices()
        .filter(|(_, c)| *c == '\n')
        .nth(err.line() - 1)
        .unwrap_or_default()
        .0;
    let (pre_line, ctx_line) = body.split_at(line_offset);
    let mut ctx = ctx_line.to_owned();
    let offset = ctx.char_indices().nth(err.column()).unwrap().0;
    ctx = ctx.split_at(offset).0.to_owned();
    ctx = pre_line.to_owned() + &ctx;

    let mut token_contexts: Vec<char> = vec![];
    let mut string_open_pre = false;
    let mut in_str = false;
    let mut found_open = false;
    let mut found_close = false;
    let mut ctx_end = 0;
    let mut last_char: char = ctx.chars().rev().next().unwrap_or_default();
    for (i, c) in ctx.char_indices().rev() {
        if c == '\\' && !in_str && string_open_pre {
            token_contexts.push(last_char);
            found_open = false;
            continue;
        }
        if c == '"' || c == '\'' {
            if in_str && token_contexts.last() == Some(&c) {
                in_str = false;
                found_open = true;
                token_contexts.pop();
            } else {
                in_str = true;
                found_close = true;
                token_contexts.push(c);
            }
        }
        if (c == ']' || c == '}') && !in_str {
            token_contexts.push(c);
            found_close = true;
        }
        if (c == '[' || c == '{')
            && !in_str
            && token_contexts.last() == Some(&json_matching_brace(c))
        {
            token_contexts.pop();
            found_open = true;
        }
        if (c == ',') && !in_str && found_close {
            found_open = true;
        }

        if !in_str && string_open_pre {
            string_open_pre = false;
        }

        if in_str {
            string_open_pre = true;
        }

        last_char = c;
        if found_open && token_contexts.is_empty() {
            ctx_end = i;
            break;
        }
    }

    if ctx_end > 0 {
        ctx = ctx[ctx_end..].to_owned();
    }

    if max_len > 0 && ctx.chars().count() > max_len {
        ctx = "... ".to_owned()
            + ctx
                .split_at(ctx.char_indices().rev().nth(max_len).unwrap().0)
                .0;
    }
    ctx
}

pub enum HashAlgo {
    Sha1,
    Sha256,
}

pub fn filehash(path: &std::path::PathBuf, algo: HashAlgo) -> Result<String> {
    match algo {
        HashAlgo::Sha1 => {
            use sha1::{Digest, Sha1};

            let mut hasher = Sha1::new();
            let mut file = std::fs::File::open(path)?;
            let _bytes_written = std::io::copy(&mut file, &mut hasher)?;
            let hash_bytes = hasher.finalize();
            Ok(format!("{:X}", hash_bytes))
        }
        HashAlgo::Sha256 => {
            use sha2::{Digest, Sha256};

            let mut hasher = Sha256::new();
            let mut file = std::fs::File::open(path)?;
            let _bytes_written = std::io::copy(&mut file, &mut hasher)?;
            let hash_bytes = hasher.finalize();
            Ok(format!("{:X}", hash_bytes))
        }
    }
}

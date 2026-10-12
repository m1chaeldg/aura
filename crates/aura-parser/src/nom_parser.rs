use crate::cst::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::{char, line_ending, not_line_ending, space0, space1},
    combinator::{recognize, rest},
    multi::many0,
    sequence::{pair, tuple},
    IResult,
    Parser,
};

/// Recognizes a newline (CRLF or LF)
pub fn parse_newline(input: &str) -> IResult<&str, &str> {
    line_ending(input)
}

/// Recognizes whitespace or comment lines (trivia)
pub fn parse_trivia(input: &str) -> IResult<&str, String> {
    let (rem, raw) = alt((
        // Comment line with newline: optional spaces, '#', rest of line, newline
        recognize(tuple((space0, char('#'), not_line_ending, parse_newline))),
        // Blank line: optional spaces and newline
        recognize(tuple((space0, parse_newline))),
        // Trailing comment or trailing whitespace at end of file (no newline)
        recognize(tuple((space0, char('#'), not_line_ending))),
        recognize(space1),
    ))(input)?;

    if raw.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::NonEmpty,
        )));
    }

    Ok((rem, raw.to_string()))
}

/// Recognizes a key identifier: letters, digits, '_', '-', '.', '~'
fn parse_key(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '~')(input)
}

// Key identifier parser

/// Parses a KeyValue line into a CstKeyValue
pub fn parse_key_value(input: &str) -> IResult<&str, CstKeyValue> {
    // Look ahead to make sure it's not a table header or fence
    let (after_ws, _) = space0(input)?;
    if after_ws.starts_with('[') || after_ws.starts_with('`') || after_ws.starts_with('#') {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let (rem, raw_line) = recognize(tuple((
        space0,
        parse_key,
        space0,
        char('='),
        space0,
        not_line_ending,
        alt((parse_newline, rest)),
    )))(input)?;

    if raw_line.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::NonEmpty,
        )));
    }

    // Extract key and value from the raw line
    let mut parts = raw_line.splitn(2, '=');
    let raw_key = parts.next().unwrap_or("").trim();
    let raw_val_line = parts.next().unwrap_or("");
    let raw_val_trimmed = raw_val_line.trim_end_matches(['\r', '\n']).trim();

    let value = if (raw_val_trimmed.starts_with('"') && raw_val_trimmed.ends_with('"'))
        || (raw_val_trimmed.starts_with('\'') && raw_val_trimmed.ends_with('\''))
    {
        let inner = &raw_val_trimmed[1..raw_val_trimmed.len() - 1];
        CstValue::String(inner.to_string())
    } else if raw_val_trimmed == "true" {
        CstValue::Boolean(true)
    } else if raw_val_trimmed == "false" {
        CstValue::Boolean(false)
    } else if let Ok(i) = raw_val_trimmed.parse::<i64>() {
        CstValue::Integer(i)
    } else {
        CstValue::Raw(raw_val_trimmed.to_string())
    };

    Ok((
        rem,
        CstKeyValue {
            raw: raw_line.to_string(),
            key: raw_key.to_string(),
            value,
        },
    ))
}

/// Parses a table header like `[headers]\r\n`
pub fn parse_table_header(input: &str) -> IResult<&str, (String, String)> {
    let (rem, _) = space0(input)?;
    let (rem, _) = char('[')(rem)?;
    let (rem, name) = parse_key(rem)?;
    let (rem, _) = char(']')(rem)?;
    let (rem, _) = space0(rem)?;
    let (rem, _) = alt((parse_newline, rest))(rem)?;

    let consumed_len = input.len() - rem.len();
    let raw = input[..consumed_len].to_string();

    Ok((rem, (raw, name.to_string())))
}

/// Parses a fenced code block ```` ```lang ... ``` ````
pub fn parse_fenced_block(input: &str) -> IResult<&str, CstFencedBlock> {
    let (after_space, _) = space0(input)?;
    if !after_space.starts_with("```") {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Capture opening fence line: ```tag \n
    let (rem_open, open_line) = recognize(tuple((
        space0,
        tag("```"),
        not_line_ending,
        parse_newline,
    )))(input)?;

    let lang = open_line
        .trim_start()
        .trim_start_matches('`')
        .trim()
        .to_string();

    // Look for closing fence
    let (rem_content, content) = take_until("```")(rem_open)?;
    let (rem_close, _close_line) = recognize(tuple((
        tag("```"),
        space0,
        alt((parse_newline, rest)),
    )))(rem_content)?;

    let consumed_len = input.len() - rem_close.len();
    let raw = input[..consumed_len].to_string();

    Ok((
        rem_close,
        CstFencedBlock {
            raw,
            lang,
            content: content.to_string(),
        },
    ))
}

/// Parses table items: trivia, key_value, or fenced block
pub fn parse_table_item(input: &str) -> IResult<&str, CstTableItem> {
    // Check if input begins next table
    let (after_ws, _) = space0(input)?;
    if after_ws.starts_with('[') {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    alt((
        parse_fenced_block.map(CstTableItem::FencedBlock),
        parse_key_value.map(CstTableItem::KeyValue),
        parse_trivia.map(CstTableItem::Trivia),
    ))(input)
}

/// Parses a complete table including header and items
pub fn parse_table(input: &str) -> IResult<&str, CstTable> {
    let (rem_header, (header_raw, name)) = parse_table_header(input)?;
    let (rem, items) = many0(parse_table_item)(rem_header)?;

    Ok((
        rem,
        CstTable {
            header_raw,
            name,
            items,
        },
    ))
}

/// Parses git merge conflict blocks `<<<<<<< ... ======= ... >>>>>>> ...`
pub fn parse_conflict_block(input: &str) -> IResult<&str, CstConflictBlock> {
    let (after_ws, _) = space0(input)?;
    if !after_ws.starts_with("<<<<<<<") {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)));
    }

    let start_idx = input.find("<<<<<<<").unwrap();
    let rest_after_start = &input[start_idx..];

    let eq_pos = rest_after_start.find("\n=======").or_else(|| rest_after_start.find("\r\n======="));
    let gt_pos = rest_after_start.find("\n>>>>>>>").or_else(|| rest_after_start.find("\r\n>>>>>>>"));

    if let (Some(eq_idx), Some(gt_idx)) = (eq_pos, gt_pos) {
        if eq_idx < gt_idx {
            let line_end = rest_after_start[gt_idx + 1..].find('\n')
                .map(|p| gt_idx + 1 + p + 1)
                .unwrap_or(rest_after_start.len());

            let full_raw = &input[..start_idx + line_end];
            let rem = &input[start_idx + line_end..];

            let open_line = rest_after_start.lines().next().unwrap_or("");
            let ours_label = open_line.trim_start_matches('<').trim().to_string();

            let ours_start = open_line.len();
            let ours_content = rest_after_start[ours_start..eq_idx].trim_start_matches(['\r', '\n']).to_string();

            let close_line = rest_after_start[gt_idx + 1..line_end].lines().next().unwrap_or("");
            let theirs_label = close_line.trim_start_matches('>').trim().to_string();

            let eq_slice = &rest_after_start[eq_idx..];
            let eq_line_end = eq_slice.find('\n').unwrap_or(0);
            let theirs_start = eq_idx + eq_line_end + 1;
            let theirs_content = if theirs_start <= gt_idx {
                rest_after_start[theirs_start..gt_idx].to_string()
            } else {
                String::new()
            };

            return Ok((rem, CstConflictBlock {
                raw: full_raw.to_string(),
                ours_label,
                ours_content,
                base_content: None,
                theirs_label,
                theirs_content,
            }));
        }
    }

    Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
}

/// Parses a full document into a CstDocument
pub fn parse_document(mut input: &str) -> Result<CstDocument, String> {
    let mut items = Vec::new();

    while !input.is_empty() {
        if let Ok((rem, conflict)) = parse_conflict_block(input) {
            items.push(CstTopLevelItem::ConflictBlock(conflict));
            input = rem;
            continue;
        }

        if let Ok((rem, table)) = parse_table(input) {
            items.push(CstTopLevelItem::Table(table));
            input = rem;
            continue;
        }

        if let Ok((rem, kv)) = parse_key_value(input) {
            items.push(CstTopLevelItem::KeyValue(kv));
            input = rem;
            continue;
        }

        if let Ok((rem, trivia)) = parse_trivia(input) {
            items.push(CstTopLevelItem::Trivia(trivia));
            input = rem;
            continue;
        }

        // If nothing matches, consume one line as trivia to prevent hanging
        if let Ok((rem, unrecog)) = recognize(pair(not_line_ending, alt((parse_newline, rest))))(input) {
            if !unrecog.is_empty() {
                items.push(CstTopLevelItem::Trivia(unrecog.to_string()));
                input = rem;
                continue;
            }
        }

        break;
    }

    Ok(CstDocument { items })
}

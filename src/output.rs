//! Output rendering — `table` (default) and `json` formats.
//!
//! Tables are hand-rolled (no extra deps) to keep the binary small.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Table,
    Json,
}

impl Format {
    pub fn is_json(self) -> bool {
        self == Format::Json
    }
}

pub fn print_json(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(_) => println!("{value}"),
    }
}

/// Print a list of JSON objects as a table, selecting the given columns.
/// `columns` is a list of (header, json-key) pairs.
pub fn print_table(rows: &[Value], columns: &[(&str, &str)]) {
    if rows.is_empty() {
        println!("(none)");
        return;
    }

    let headers: Vec<&str> = columns.iter().map(|(h, _)| *h).collect();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    let cells: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            columns
                .iter()
                .map(|(_, key)| scalar(row.get(*key)))
                .collect::<Vec<_>>()
        })
        .collect();

    for row in &cells {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }

    // Header
    let header_line: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| pad(h, widths[i]))
        .collect();
    println!("{}", header_line.join("  "));
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    println!("{}", sep.join("  "));

    for row in &cells {
        let line: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, c)| pad(c, widths[i]))
            .collect();
        println!("{}", line.join("  "));
    }
}

/// Print key/value pairs (one record), e.g. a profile or a single resource.
pub fn print_kv(pairs: &[(&str, String)]) {
    let label_w = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (k, v) in pairs {
        println!("{:<width$}  {}", k, v, width = label_w);
    }
}

fn scalar(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "-".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(),
    }
}

fn display_width(s: &str) -> usize {
    s.chars().count()
}

fn pad(s: &str, width: usize) -> String {
    let len = display_width(s);
    if len >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - len))
    }
}

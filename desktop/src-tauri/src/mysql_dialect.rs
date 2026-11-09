//! Deliberately small, token-aware adaptation of the application's SQL dialect.
//! Values always travel as prepared-statement parameters, never interpolation.
use crate::error::{AppError, Result, ensure};
use rusqlite::types::Value;

#[derive(Debug)]
enum Token {
    Word(String),
    Quoted(String),
    Other(String),
    Parameter(usize),
}
fn lex(sql: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();
    let mut automatic = 0;
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, '\'' | '"' | '`') {
            let start = i;
            i += 1;
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == c {
                    i += 1;
                    if i < chars.len() && chars[i] == c {
                        i += 1;
                        continue;
                    }
                    closed = true;
                    break;
                }
                i += 1;
            }
            ensure(closed, "DATABASE_QUERY", "La consulta está incompleta.")?;
            tokens.push(Token::Quoted(chars[start..i].iter().collect()));
        } else if c == '-' && chars.get(i + 1) == Some(&'-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '?' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let index = if i == start {
                automatic += 1;
                automatic
            } else {
                chars[start..i]
                    .iter()
                    .collect::<String>()
                    .parse::<usize>()
                    .map_err(|_| AppError::rule("DATABASE_QUERY", "Parámetro no válido."))?
            };
            tokens.push(Token::Parameter(index));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token::Word(chars[start..i].iter().collect()));
        } else {
            tokens.push(Token::Other(c.to_string()));
            i += 1;
        }
    }
    Ok(tokens)
}
pub fn statements(sql: &str) -> Result<Vec<String>> {
    let tokens = lex(sql)?;
    let mut statements = Vec::new();
    let mut current = String::new();
    for token in tokens {
        match token {
            Token::Other(s) if s == ";" => {
                if !current.trim().is_empty() {
                    statements.push(std::mem::take(&mut current));
                }
            }
            Token::Word(s) | Token::Quoted(s) | Token::Other(s) => current.push_str(&s),
            Token::Parameter(n) => current.push_str(&format!("?{n}")),
        }
    }
    if !current.trim().is_empty() {
        statements.push(current);
    }
    Ok(statements)
}
pub fn translate(sql: &str, values: &[Value]) -> Result<(String, mysql::Params)> {
    let tokens = lex(sql)?;
    let mut output = String::new();
    let mut params = Vec::new();
    for token in tokens {
        match token {
            Token::Parameter(n) => {
                let value = n
                    .checked_sub(1)
                    .and_then(|i| values.get(i))
                    .ok_or_else(|| {
                        AppError::rule("DATABASE_QUERY", "Falta un parámetro de la consulta.")
                    })?;
                params.push(match value {
                    Value::Null => mysql::Value::NULL,
                    Value::Integer(n) => mysql::Value::Int(*n),
                    Value::Real(n) => mysql::Value::Double(*n),
                    Value::Text(s) => mysql::Value::Bytes(s.as_bytes().to_vec()),
                    Value::Blob(b) => mysql::Value::Bytes(b.clone()),
                });
                output.push('?');
            }
            Token::Word(s) if s.eq_ignore_ascii_case("NOCASE") => {
                output.push_str("utf8mb4_0900_as_ci")
            }
            Token::Word(s)
                if [
                    "Key",
                    "Value",
                    "User",
                    "Sequence",
                    "Role",
                    "Reference",
                    "Rank",
                    "Condition",
                ]
                .iter()
                .any(|w| s.eq_ignore_ascii_case(w)) =>
            {
                output.push_str(&format!("`{s}`"))
            }
            Token::Word(s) | Token::Quoted(s) | Token::Other(s) => output.push_str(&s),
        }
    }
    // All upserts use an explicit, known primary/unique business key. MySQL's
    // duplicate-key semantics are used only for these internal statements.
    if let Some(begin) = output.find(" ON CONFLICT(") {
        let end = output[begin..]
            .find(") DO UPDATE SET ")
            .ok_or_else(|| AppError::rule("DATABASE_QUERY", "Actualización incompatible."))?
            + begin;
        output.replace_range(
            begin..end + ") DO UPDATE SET ".len(),
            " ON DUPLICATE KEY UPDATE ",
        );
        let re = output.clone();
        for column in [
            "ConfigJson",
            "Subtitle",
            "AutoPrint",
            "TutorialStatus",
            "TutorialStep",
            "Theme",
            "ReducedMotion",
            "Value",
            "MarginMm",
            "Copies",
            "Monochrome",
            "ShowLogo",
            "Legend",
        ] {
            let name = if column == "Value" { "`Value`" } else { column };
            output = output.replace(&format!("excluded.{name}"), &format!("VALUES({name})"));
        }
        ensure(
            !output.contains("excluded."),
            "DATABASE_QUERY",
            &format!("Columna de actualización no admitida ({})", re.len()),
        )?;
    }
    Ok((output, mysql::Params::Positional(params)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binds_repeated_and_reordered_parameters_without_touching_literals() {
        let (sql, p) = translate(
            "SELECT '?9; Key', Value FROM AppMeta WHERE Key=?2 OR Value=?1 OR Value=?2",
            &[Value::Text("x' OR 1=1".into()), Value::Text("Key".into())],
        )
        .unwrap();
        assert_eq!(
            sql,
            "SELECT '?9; Key', `Value` FROM AppMeta WHERE `Key`=? OR `Value`=? OR `Value`=?"
        );
        assert_eq!(
            p,
            mysql::Params::Positional(vec!["Key".into(), "x' OR 1=1".into(), "Key".into()])
        );
    }
    #[test]
    fn splits_statements_outside_quoted_strings() {
        assert_eq!(
            statements("-- heading\nINSERT INTO T VALUES('a;b'); SELECT 1;")
                .unwrap()
                .len(),
            2
        );
    }
    #[test]
    fn translates_only_upsert_syntax() {
        let(s,_)=translate("INSERT INTO AppMeta(Key,Value) VALUES(?1,?2) ON CONFLICT(Key) DO UPDATE SET Value=excluded.Value",&[Value::Text("x".into()),Value::Text("y".into())]).unwrap();
        assert!(s.ends_with("ON DUPLICATE KEY UPDATE `Value`=VALUES(`Value`)"));
    }
}

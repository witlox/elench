//! # elench-predicate
//!
//! Parser and evaluator for `elench-predicate-v1` — a small DSL of check
//! primitives for claim predicates (ADR-0004).
//!
//! Four primitives: `grep`, `test`, `run`, `exists`. Combined with
//! comparison operators (`==`, `!=`, `>=`, `<=`, `>`, `<`), boolean
//! logic (`&&`, `||`, `!`), and string matching (`.contains`, `.matches`).
//! Not Turing-complete. Deterministic by construction (INV-23).

use std::path::PathBuf;

use regex::Regex;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A parsed predicate expression. Can be evaluated against an [`EvalContext`].
#[derive(Debug, Clone)]
pub enum Expression {
    /// Integer literal.
    Int(i64),
    /// String literal.
    Str(String),
    /// Boolean literal.
    Bool(bool),
    /// `grep(pattern, path) → int` — count of regex matches in a file.
    Grep { pattern: Regex, path: String },
    /// `test(name) → { passed: bool }` — a named test passes.
    Test { name: String },
    /// `run(cmd) → { exit: int, stdout: String }` — run a command.
    Run { cmd: String },
    /// `exists(path) → bool` — a file exists.
    Exists { path: String },
    /// Field access on a structured result: `.passed`, `.exit`, `.stdout`.
    Field {
        expr: Box<Expression>,
        field: FieldName,
    },
    /// Comparison: `left OP right`.
    Compare {
        left: Box<Expression>,
        op: CmpOp,
        right: Box<Expression>,
    },
    /// Boolean AND.
    And(Box<Expression>, Box<Expression>),
    /// Boolean OR.
    Or(Box<Expression>, Box<Expression>),
    /// Boolean NOT.
    Not(Box<Expression>),
    /// String method: `left.contains(right)`.
    StringContains {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// String method: `left.matches(right)` (regex match).
    StringMatches {
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

/// Result fields accessible via `.` notation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldName {
    /// `test(name).passed` — whether the test passed.
    Passed,
    /// `run(cmd).exit` — the process exit code.
    Exit,
    /// `run(cmd).stdout` — the process stdout.
    Stdout,
}

/// Comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
}

/// Context for evaluating a predicate. Carries the workspace root and
/// (future) sandbox configuration.
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// The workspace root. `grep` and `exists` paths are resolved
    /// relative to this (or treated as absolute if they start with `/`).
    pub workspace_root: PathBuf,
}

/// The result of evaluating an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Str(String),
    Bool(bool),
}

impl Value {
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("invalid regex: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("expected {expected}, found {found}")]
    Expected { expected: String, found: String },
    #[error("empty expression")]
    Empty,
}

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("expected bool, got {0:?}")]
    ExpectedBool(Value),
    #[error("expected int, got {0:?}")]
    ExpectedInt(Value),
    #[error("expected str, got {0:?}")]
    ExpectedStr(Value),
    #[error("field '{0}' not available on this expression")]
    InvalidField(String),
    #[error("I/O error reading {path}: {err}")]
    Io { path: String, err: std::io::Error },
    #[error("command failed to execute: {0}")]
    CommandExec(String),
    #[error("unknown field: {0}")]
    UnknownField(String),
    #[error("invalid regex: {0}")]
    InvalidRegex(#[from] regex::Error),
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse a predicate expression string into an [`Expression`].
///
/// Grammar (informal, ADR-0004):
/// ```text
/// expr        := or_expr
/// or_expr     := and_expr ('||' and_expr)*
/// and_expr    := not_expr ('&&' not_expr)*
/// not_expr    := '!' not_expr | comparison
/// comparison  := term (op term)?
/// term        := primitive ('.' field)*
/// primitive   := grep_call | test_call | run_call | exists_call
///              | int_lit | str_lit | bool_lit
/// ```
///
/// Parse a predicate expression string.
///
/// # Errors
///
/// Returns [`ParseError`] if the input is not a valid
/// `elench-predicate-v1` expression.
pub fn parse(source: &str) -> Result<Expression, ParseError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    let tokens = tokenize(trimmed)?;
    let mut parser = Parser::new(&tokens);
    let expr = parser.parse_or()?;
    if parser.pos < parser.tokens.len() {
        return Err(ParseError::UnexpectedToken(format!(
            "{:?}",
            parser.tokens[parser.pos]
        )));
    }
    Ok(expr)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int(i64),
    Str(String),
    Ident(String), // function name, field name, true/false
    Regex(String), // /pattern/
    LParen,
    RParen,
    Dot,
    Bang,
    AndAnd,
    OrOr,
    EqEq,
    NotEq,
    Ge,
    Le,
    Gt,
    Lt,
    Comma,
}

#[allow(clippy::too_many_lines)]
fn tokenize(src: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '.' => {
                tokens.push(Token::Dot);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '!' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::NotEq);
                i += 2;
            }
            '!' => {
                tokens.push(Token::Bang);
                i += 1;
            }
            '&' if i + 1 < chars.len() && chars[i + 1] == '&' => {
                tokens.push(Token::AndAnd);
                i += 2;
            }
            '|' if i + 1 < chars.len() && chars[i + 1] == '|' => {
                tokens.push(Token::OrOr);
                i += 2;
            }
            '=' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::EqEq);
                i += 2;
            }
            '>' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Ge);
                i += 2;
            }
            '<' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Le);
                i += 2;
            }
            '>' => {
                tokens.push(Token::Gt);
                i += 1;
            }
            '<' => {
                tokens.push(Token::Lt);
                i += 1;
            }
            '/' => {
                // Regex literal: /pattern/ (no escaping of / in v1)
                let mut pattern = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '/' {
                    pattern.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(ParseError::UnexpectedEof);
                }
                i += 1; // skip closing /
                tokens.push(Token::Regex(pattern));
            }
            '"' => {
                // String literal
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        match chars[i] {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            other => s.push(other),
                        }
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(ParseError::UnexpectedEof);
                }
                i += 1; // skip closing "
                tokens.push(Token::Str(s));
            }
            _ if c.is_ascii_digit() => {
                let mut n = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    n.push(chars[i]);
                    i += 1;
                }
                let n: i64 = n
                    .parse()
                    .map_err(|_| ParseError::UnexpectedToken(n.clone()))?;
                tokens.push(Token::Int(n));
            }
            _ if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    ident.push(chars[i]);
                    i += 1;
                }
                tokens.push(Token::Ident(ident));
            }
            _ => {
                return Err(ParseError::UnexpectedToken(c.to_string()));
            }
        }
    }

    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        self.pos += 1;
        tok
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;
        while self.peek() == Some(&Token::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            left = Expression::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_not()?;
        while self.peek() == Some(&Token::AndAnd) {
            self.advance();
            let right = self.parse_not()?;
            left = Expression::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expression, ParseError> {
        if self.peek() == Some(&Token::Bang) {
            self.advance();
            let inner = self.parse_not()?;
            return Ok(Expression::Not(Box::new(inner)));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_term()?;
        let op = match self.peek() {
            Some(Token::EqEq) => Some(CmpOp::Eq),
            Some(Token::NotEq) => Some(CmpOp::Ne),
            Some(Token::Ge) => Some(CmpOp::Ge),
            Some(Token::Le) => Some(CmpOp::Le),
            Some(Token::Gt) => Some(CmpOp::Gt),
            Some(Token::Lt) => Some(CmpOp::Lt),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let right = self.parse_term()?;
            Ok(Expression::Compare {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primitive()?;
        while self.peek() == Some(&Token::Dot) {
            self.advance();
            let field = match self.advance() {
                Some(Token::Ident(name)) if name == "passed" => FieldName::Passed,
                Some(Token::Ident(name)) if name == "exit" => FieldName::Exit,
                Some(Token::Ident(name)) if name == "stdout" => FieldName::Stdout,
                Some(Token::Ident(name)) if name == "contains" => {
                    self.expect_lparen()?;
                    let arg = self.parse_primitive()?;
                    self.expect_rparen()?;
                    return Ok(Expression::StringContains {
                        left: Box::new(expr),
                        right: Box::new(arg),
                    });
                }
                Some(Token::Ident(name)) if name == "matches" => {
                    self.expect_lparen()?;
                    let arg = self.parse_primitive()?;
                    self.expect_rparen()?;
                    return Ok(Expression::StringMatches {
                        left: Box::new(expr),
                        right: Box::new(arg),
                    });
                }
                Some(other) => {
                    return Err(ParseError::Expected {
                        expected: "field name".into(),
                        found: format!("{other:?}"),
                    });
                }
                None => return Err(ParseError::UnexpectedEof),
            };
            expr = Expression::Field {
                expr: Box::new(expr),
                field,
            };
        }
        Ok(expr)
    }

    #[allow(clippy::too_many_lines)]
    fn parse_primitive(&mut self) -> Result<Expression, ParseError> {
        match self.advance() {
            Some(Token::Int(n)) => Ok(Expression::Int(*n)),
            Some(Token::Str(s) | Token::Regex(s)) => Ok(Expression::Str(s.clone())),
            Some(Token::Ident(name)) if name == "true" => Ok(Expression::Bool(true)),
            Some(Token::Ident(name)) if name == "false" => Ok(Expression::Bool(false)),
            Some(Token::Ident(name)) if name == "grep" => {
                self.expect_lparen()?;
                let pattern_tok = self.advance().cloned();
                let pattern = match pattern_tok {
                    Some(Token::Regex(p) | Token::Str(p)) => Regex::new(&p)?,
                    other => {
                        return Err(ParseError::Expected {
                            expected: "regex or string pattern".into(),
                            found: format!("{other:?}"),
                        });
                    }
                };
                self.expect_comma()?;
                let path = match self.advance() {
                    Some(Token::Str(s)) => s.clone(),
                    other => {
                        return Err(ParseError::Expected {
                            expected: "string path".into(),
                            found: format!("{other:?}"),
                        });
                    }
                };
                self.expect_rparen()?;
                Ok(Expression::Grep { pattern, path })
            }
            Some(Token::Ident(name)) if name == "test" => {
                self.expect_lparen()?;
                let test_name = match self.advance() {
                    Some(Token::Str(s)) => s.clone(),
                    other => {
                        return Err(ParseError::Expected {
                            expected: "string test name".into(),
                            found: format!("{other:?}"),
                        });
                    }
                };
                self.expect_rparen()?;
                Ok(Expression::Test { name: test_name })
            }
            Some(Token::Ident(name)) if name == "run" => {
                self.expect_lparen()?;
                let cmd = match self.advance() {
                    Some(Token::Str(s)) => s.clone(),
                    other => {
                        return Err(ParseError::Expected {
                            expected: "string command".into(),
                            found: format!("{other:?}"),
                        });
                    }
                };
                self.expect_rparen()?;
                Ok(Expression::Run { cmd })
            }
            Some(Token::Ident(name)) if name == "exists" => {
                self.expect_lparen()?;
                let path = match self.advance() {
                    Some(Token::Str(s)) => s.clone(),
                    other => {
                        return Err(ParseError::Expected {
                            expected: "string path".to_string(),
                            found: format!("{other:?}"),
                        });
                    }
                };
                self.expect_rparen()?;
                Ok(Expression::Exists { path })
            }
            Some(other) => Err(ParseError::UnexpectedToken(format!("{other:?}"))),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn expect_lparen(&mut self) -> Result<(), ParseError> {
        match self.advance() {
            Some(Token::LParen) => Ok(()),
            other => Err(ParseError::Expected {
                expected: "(".into(),
                found: format!("{other:?}"),
            }),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), ParseError> {
        match self.advance() {
            Some(Token::RParen) => Ok(()),
            other => Err(ParseError::Expected {
                expected: ")".into(),
                found: format!("{other:?}"),
            }),
        }
    }

    fn expect_comma(&mut self) -> Result<(), ParseError> {
        match self.advance() {
            Some(Token::Comma) => Ok(()),
            other => Err(ParseError::Expected {
                expected: ",".into(),
                found: format!("{other:?}"),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

/// Evaluate a parsed expression against the given context.
///
/// Evaluate a parsed expression against the given context.
///
/// # Errors
///
/// Returns [`EvalError`] if evaluation fails (I/O, command
#[allow(clippy::too_many_lines)]
/// execution, type mismatch, invalid field access).
pub fn evaluate(expr: &Expression, ctx: &EvalContext) -> Result<Value, EvalError> {
    match expr {
        Expression::Int(n) => Ok(Value::Int(*n)),
        Expression::Str(s) => Ok(Value::Str(s.clone())),
        Expression::Bool(b) => Ok(Value::Bool(*b)),

        Expression::Grep { pattern, path } => {
            let resolved = resolve_path(path, ctx);
            let content = std::fs::read_to_string(&resolved).map_err(|err| EvalError::Io {
                path: path.clone(),
                err,
            })?;
            let count = content
                .lines()
                .filter(|line| pattern.is_match(line))
                .count();
            Ok(Value::Int(i64::try_from(count).unwrap_or(i64::MAX)))
        }

        Expression::Exists { path } => {
            let resolved = resolve_path(path, ctx);
            Ok(Value::Bool(resolved.exists()))
        }

        Expression::Test { name } => {
            // Run `cargo test <name> -- --exact` and check exit code.
            // For v1, this is a thin wrapper around `run`.
            let output = std::process::Command::new("cargo")
                .args(["test", name, "--", "--exact"])
                .current_dir(&ctx.workspace_root)
                .output()
                .map_err(|err| EvalError::CommandExec(err.to_string()))?;
            Ok(Value::Bool(output.status.success()))
        }

        Expression::Run { cmd } => {
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(&ctx.workspace_root)
                .output()
                .map_err(|err| EvalError::CommandExec(err.to_string()))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(Value::Str(format!(
                "{{\"exit\":{},\"stdout\":\"{}\"}}",
                output.status.code().unwrap_or(-1),
                stdout
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
            )))
        }

        Expression::Field { expr, field } => {
            // Evaluate the inner expression and access the named field.
            // For test(name).passed, the inner is Test, evaluated to Bool.
            // For run(cmd).exit, the inner is Run, evaluated to Str (JSON).
            let inner = evaluate(expr, ctx)?;
            match (inner, field) {
                (Value::Bool(b), FieldName::Passed) => Ok(Value::Bool(b)),
                (Value::Str(s), FieldName::Exit) => {
                    // Parse the JSON-like output from run
                    let exit_code = s
                        .find("\"exit\":")
                        .and_then(|i| s[i + 7..].split(',').next())
                        .and_then(|s| s.trim().parse::<i64>().ok())
                        .unwrap_or(-1);
                    Ok(Value::Int(exit_code))
                }
                (Value::Str(s), FieldName::Stdout) => {
                    let stdout = s
                        .find("\"stdout\":\"")
                        .map(|i| &s[i + 10..])
                        .and_then(|rest| rest.rfind('"').map(|j| rest[..j].to_string()))
                        .unwrap_or_default()
                        .replace("\\n", "\n")
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\");
                    Ok(Value::Str(stdout))
                }
                (val, field) => Err(EvalError::InvalidField(format!("{field:?} on {val:?}"))),
            }
        }

        Expression::Compare { left, op, right } => {
            let lv = evaluate(left, ctx)?;
            let rv = evaluate(right, ctx)?;
            let result = match (&lv, op, &rv) {
                (Value::Int(a), CmpOp::Eq, Value::Int(b)) => a == b,
                (Value::Int(a), CmpOp::Ne, Value::Int(b)) => a != b,
                (Value::Int(a), CmpOp::Ge, Value::Int(b)) => a >= b,
                (Value::Int(a), CmpOp::Le, Value::Int(b)) => a <= b,
                (Value::Int(a), CmpOp::Gt, Value::Int(b)) => a > b,
                (Value::Int(a), CmpOp::Lt, Value::Int(b)) => a < b,
                (Value::Str(a), CmpOp::Eq, Value::Str(b)) => a == b,
                (Value::Str(a), CmpOp::Ne, Value::Str(b)) => a != b,
                (Value::Bool(a), CmpOp::Eq, Value::Bool(b)) => a == b,
                (Value::Bool(a), CmpOp::Ne, Value::Bool(b)) => a != b,
                _ => {
                    return Err(EvalError::ExpectedInt(lv));
                }
            };
            Ok(Value::Bool(result))
        }

        Expression::And(a, b) => {
            let av = evaluate(a, ctx)?
                .as_bool()
                .ok_or(EvalError::ExpectedBool(Value::Bool(false)))?;
            if !av {
                return Ok(Value::Bool(false));
            }
            let bv = evaluate(b, ctx)?
                .as_bool()
                .ok_or(EvalError::ExpectedBool(Value::Bool(false)))?;
            Ok(Value::Bool(av && bv))
        }

        Expression::Or(a, b) => {
            let av = evaluate(a, ctx)?
                .as_bool()
                .ok_or(EvalError::ExpectedBool(Value::Bool(false)))?;
            if av {
                return Ok(Value::Bool(true));
            }
            let bv = evaluate(b, ctx)?
                .as_bool()
                .ok_or(EvalError::ExpectedBool(Value::Bool(false)))?;
            Ok(Value::Bool(av || bv))
        }

        Expression::Not(inner) => {
            let v = evaluate(inner, ctx)?
                .as_bool()
                .ok_or(EvalError::ExpectedBool(Value::Bool(false)))?;
            Ok(Value::Bool(!v))
        }

        Expression::StringContains { left, right } => {
            let lv = evaluate(left, ctx)?
                .as_str()
                .ok_or(EvalError::ExpectedStr(Value::Bool(false)))?
                .to_string();
            let rv = evaluate(right, ctx)?
                .as_str()
                .ok_or(EvalError::ExpectedStr(Value::Bool(false)))?
                .to_string();
            Ok(Value::Bool(lv.contains(&rv)))
        }

        Expression::StringMatches { left, right } => {
            let lv = evaluate(left, ctx)?
                .as_str()
                .ok_or(EvalError::ExpectedStr(Value::Bool(false)))?
                .to_string();
            let rv = evaluate(right, ctx)?
                .as_str()
                .ok_or(EvalError::ExpectedStr(Value::Bool(false)))?
                .to_string();
            let re = Regex::new(&rv)?;
            Ok(Value::Bool(re.is_match(&lv)))
        }
    }
}

fn resolve_path(path: &str, ctx: &EvalContext) -> PathBuf {
    if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        ctx.workspace_root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn ctx() -> EvalContext {
        EvalContext {
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    // --- Parser tests ---

    #[test]
    fn scenario_parser_grep_predicate() {
        let expr = parse(r#"grep(/KISEKI_INTENT_EPOCH_ROTATE/, "src/lib.rs") >= 1"#).unwrap();
        match expr {
            Expression::Compare {
                left,
                op: CmpOp::Ge,
                right,
            } => {
                assert!(matches!(*left, Expression::Grep { .. }));
                assert!(matches!(*right, Expression::Int(1)));
            }
            other => panic!("expected Compare, got {other:?}"),
        }
    }

    #[test]
    fn scenario_parser_test_predicate() {
        let expr = parse(r#"test("my_test").passed"#).unwrap();
        match expr {
            Expression::Field {
                expr: inner,
                field: FieldName::Passed,
            } => {
                assert!(matches!(*inner, Expression::Test { .. }));
            }
            other => panic!("expected Field, got {other:?}"),
        }
    }

    #[test]
    fn scenario_parser_run_exit_predicate() {
        let expr = parse(r#"run("cargo clippy -- -D warnings").exit == 0"#).unwrap();
        match expr {
            Expression::Compare {
                left,
                op: CmpOp::Eq,
                right,
            } => {
                assert!(matches!(*left, Expression::Field { .. }));
                assert!(matches!(*right, Expression::Int(0)));
            }
            other => panic!("expected Compare, got {other:?}"),
        }
    }

    #[test]
    fn scenario_parser_exists_predicate() {
        let expr = parse(r#"exists("scripts/perf-gate.sh")"#).unwrap();
        assert!(matches!(expr, Expression::Exists { .. }));
    }

    #[test]
    fn scenario_parser_boolean_and() {
        let expr = parse(r#"exists("a.txt") && exists("b.txt")"#).unwrap();
        assert!(matches!(expr, Expression::And(_, _)));
    }

    #[test]
    fn scenario_parser_boolean_or() {
        let expr = parse(r#"exists("a.txt") || exists("b.txt")"#).unwrap();
        assert!(matches!(expr, Expression::Or(_, _)));
    }

    #[test]
    fn scenario_parser_boolean_not() {
        let expr = parse(r#"!exists("a.txt")"#).unwrap();
        assert!(matches!(expr, Expression::Not(_)));
    }

    #[test]
    fn scenario_parser_empty_input_rejected() {
        assert!(matches!(parse(""), Err(ParseError::Empty)));
        assert!(matches!(parse("   "), Err(ParseError::Empty)));
    }

    #[test]
    fn scenario_parser_prose_in_predicate_slot_rejected() {
        let result = parse("Input validation is now handled correctly.");
        assert!(result.is_err(), "prose should be rejected by parser");
    }

    // --- Evaluator tests ---

    #[test]
    fn scenario_evaluator_grep_counts_matches() {
        let tmp = tempfile_named("test_grep.txt", "foo\nbar\nfoobar\nbaz\n");
        let expr = parse(&format!(r#"grep(/foo/, "{}") >= 2"#, tmp.to_str().unwrap())).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn scenario_evaluator_grep_zero_matches() {
        let tmp = tempfile_named("test_grep_zero.txt", "bar\nbaz\n");
        let expr = parse(&format!(r#"grep(/foo/, "{}") >= 1"#, tmp.to_str().unwrap())).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn scenario_evaluator_exists_true() {
        let tmp = tempfile_named("test_exists.txt", "content\n");
        let expr = parse(&format!(r#"exists("{}")"#, tmp.to_str().unwrap())).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn scenario_evaluator_exists_false() {
        let expr = parse(r#"exists("/tmp/opencode/elench_nonexistent_file.txt")"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn scenario_evaluator_boolean_and() {
        let tmp = tempfile_named("test_and.txt", "content\n");
        let expr = parse(&format!(
            r#"exists("{}") && exists("/tmp/opencode/elench_nonexistent.txt")"#,
            tmp.to_str().unwrap()
        ))
        .unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(false)); // first true, second false
    }

    #[test]
    fn scenario_evaluator_boolean_or() {
        let tmp = tempfile_named("test_or.txt", "content\n");
        let expr = parse(&format!(
            r#"exists("{}") || exists("/tmp/opencode/elench_nonexistent.txt")"#,
            tmp.to_str().unwrap()
        ))
        .unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true)); // first true
    }

    #[test]
    fn scenario_evaluator_boolean_not() {
        let expr = parse(r#"!exists("/tmp/opencode/elench_nonexistent.txt")"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true)); // not false = true
    }

    #[test]
    fn scenario_evaluator_integer_comparison() {
        let tmp = tempfile_named("test_cmp.txt", "a\nb\nc\n");
        let expr = parse(&format!(r#"grep(/.*/, "{}") == 3"#, tmp.to_str().unwrap())).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn scenario_evaluator_run_exit_code() {
        let expr = parse(r#"run("true").exit == 0"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn scenario_evaluator_run_nonzero_exit() {
        let expr = parse(r#"run("false").exit != 0"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    // --- Round-trip tests ---

    #[test]
    fn scenario_round_trip_grep_predicate() {
        let tmp = tempfile_named("round_trip.txt", "foo\nbar\n");
        let source = format!(r#"grep(/foo/, "{}") >= 1"#, tmp.to_str().unwrap());
        let expr = parse(&source).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn scenario_round_trip_complex_predicate() {
        let source = r#"exists("Cargo.toml") && grep(/elench/, "Cargo.toml") >= 1"#;
        let expr = parse(source).unwrap();
        let result = evaluate(
            &expr,
            &EvalContext {
                workspace_root: std::env::current_dir().unwrap(),
            },
        )
        .unwrap();
        assert_eq!(result, Value::Bool(true)); // this repo has Cargo.toml with "elench"
    }

    // --- .contains() and .matches() tests (GAP-3) ---

    #[test]
    fn scenario_string_contains_true() {
        let expr = parse(r#""hello world".contains("world")"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn scenario_string_contains_false() {
        let expr = parse(r#""hello world".contains("xyz")"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn scenario_string_matches_true() {
        let expr = parse(r#""hello world".matches(/hello/)"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn scenario_string_matches_false() {
        let expr = parse(r#""hello world".matches(/xyz/)"#).unwrap();
        let result = evaluate(&expr, &ctx()).unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    // --- INV-08: predicate without expression rejected ---
    // (scenario_parser_prose_in_predicate_slot_rejected above already
    // covers this; the duplicate test was removed per auditor GAP-15.)

    /// Create a temporary file with the given content, return its path.
    fn tempfile_named(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("elench_test_{name}"));
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }
}

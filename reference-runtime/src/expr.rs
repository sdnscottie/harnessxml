//! The HarnessXML expression language — specification chapter 10.
//!
//! Copyright 2026 VisML. SPDX-License-Identifier: Apache-2.0
//!
//! Deliberately small: it reads values, compares them, and combines the
//! results. No user-defined functions, no recursion, no assignment, no side
//! effects, and no access to the clock, the environment or the filesystem.
//!
//! Every one of those omissions is load-bearing. The whole value proposition is
//! validating a workflow BEFORE it runs, and each of them would make an
//! expression's value depend on something outside the document — which makes
//! the workflow irreproducible and its validation meaningless.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------- values

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
        }
    }

    pub fn truthy(&self) -> Result<bool, EvalError> {
        match self {
            Value::Bool(b) => Ok(*b),
            // §10.7 — a guard/case/condition evaluating to a non-boolean is
            // HX-4107, never coerced. Treating null as false would silently
            // take the `otherwise` branch whenever an upstream value was
            // missing, which looks exactly like a deliberate routing decision
            // in every log and is not one.
            other => Err(EvalError::new(
                "HX-4107",
                format!(
                    "condition evaluated to {} ({}), not a boolean",
                    other,
                    other.type_name()
                ),
            )),
        }
    }

    /// JSON-ish rendering, used in traces.
    pub fn to_json(&self) -> String {
        match self {
            Value::Null => "null".into(),
            Value::Bool(b) => b.to_string(),
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Value::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Value::Array(a) => format!(
                "[{}]",
                a.iter().map(|v| v.to_json()).collect::<Vec<_>>().join(",")
            ),
            Value::Map(m) => format!(
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("\"{k}\":{}", v.to_json()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Str(s) => f.write_str(s),
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            other => f.write_str(&other.to_json()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EvalError {
    pub code: &'static str,
    pub message: String,
}

impl EvalError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code, self.message)
    }
}

type R<T> = Result<T, EvalError>;

// ---------------------------------------------------------------- context

/// Everything an expression is allowed to see. Note what is absent: no clock,
/// no environment, no filesystem, no network.
#[derive(Default)]
pub struct Ctx {
    /// node id -> port name -> value, for nodes that have SUCCEEDED.
    pub outputs: HashMap<String, HashMap<String, Value>>,
    /// Loop iteration variables currently in scope.
    pub vars: HashMap<String, Value>,
    /// `<config>` of the node being evaluated.
    pub config: HashMap<String, Value>,
    /// Declaration attributes, exposed through artifact('id') / resource('id').
    pub artifacts: HashMap<String, BTreeMap<String, Value>>,
    pub resources: HashMap<String, BTreeMap<String, Value>>,
    /// Nodes that reached SKIPPED — their outputs are unavailable (§5.6), which
    /// is different from "produced null".
    pub skipped: Vec<String>,
}

// ---------------------------------------------------------------- lexer

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Eof,
}

fn lex(src: &str) -> R<Vec<Tok>> {
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == '.') {
                i += 1;
            }
            let s: String = b[start..i].iter().collect();
            out.push(Tok::Num(s.parse().map_err(|_| {
                EvalError::new("HX-3101", format!("malformed number '{s}'"))
            })?));
            continue;
        }
        if c == '\'' || c == '"' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < b.len() && b[i] != quote {
                if b[i] == '\\' && i + 1 < b.len() {
                    i += 1;
                }
                s.push(b[i]);
                i += 1;
            }
            if i >= b.len() {
                return Err(EvalError::new("HX-3101", "unterminated string literal"));
            }
            i += 1;
            out.push(Tok::Str(s));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_' || b[i] == '-') {
                i += 1;
            }
            out.push(Tok::Ident(b[start..i].iter().collect()));
            continue;
        }
        // multi-character operators first
        let two: String = b[i..(i + 2).min(b.len())].iter().collect();
        if ["==", "!=", "<=", ">=", "&&", "||", "??"].contains(&two.as_str()) {
            out.push(Tok::Op(two));
            i += 2;
            continue;
        }
        i += 1;
        match c {
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            '[' => out.push(Tok::LBracket),
            ']' => out.push(Tok::RBracket),
            ',' => out.push(Tok::Comma),
            '.' => out.push(Tok::Dot),
            '+' | '-' | '*' | '/' | '%' | '<' | '>' | '!' => out.push(Tok::Op(c.to_string())),
            _ => {
                return Err(EvalError::new(
                    "HX-3101",
                    format!("unexpected character '{c}'"),
                ));
            }
        }
    }
    out.push(Tok::Eof);
    Ok(out)
}

// ---------------------------------------------------------------- ast

#[derive(Debug, Clone)]
enum Ast {
    Lit(Value),
    /// A dotted path: `classify.confidence`, `config.threshold`, or a bare `item`.
    Path(Vec<String>),
    Call(String, Vec<Ast>),
    /// Field access on the result of anything that is not a bare path —
    /// `artifact('x').digest`, `(m).k`. A bare path consumes its own dots.
    Member(Box<Ast>, String),
    Unary(String, Box<Ast>),
    Bin(String, Box<Ast>, Box<Ast>),
    Array(Vec<Ast>),
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.t[self.i]
    }
    fn next(&mut self) -> Tok {
        let t = self.t[self.i].clone();
        self.i += 1;
        t
    }
    fn eat_op(&mut self, ops: &[&str]) -> Option<String> {
        if let Tok::Op(o) = self.peek()
            && ops.contains(&o.as_str())
        {
            let o = o.clone();
            self.i += 1;
            return Some(o);
        }
        None
    }
    fn eat_word(&mut self, words: &[&str]) -> Option<String> {
        if let Tok::Ident(w) = self.peek()
            && words.contains(&w.as_str())
        {
            let w = w.clone();
            self.i += 1;
            return Some(w);
        }
        None
    }

    // expr := coalesce
    fn expr(&mut self) -> R<Ast> {
        self.coalesce()
    }

    fn coalesce(&mut self) -> R<Ast> {
        let mut left = self.or()?;
        while self.eat_op(&["??"]).is_some() {
            let right = self.or()?;
            left = Ast::Bin("??".into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn or(&mut self) -> R<Ast> {
        let mut left = self.and()?;
        loop {
            let hit = self.eat_op(&["||"]).is_some() || self.eat_word(&["or"]).is_some();
            if !hit {
                break;
            }
            let right = self.and()?;
            left = Ast::Bin("or".into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and(&mut self) -> R<Ast> {
        let mut left = self.not()?;
        loop {
            let hit = self.eat_op(&["&&"]).is_some() || self.eat_word(&["and"]).is_some();
            if !hit {
                break;
            }
            let right = self.not()?;
            left = Ast::Bin("and".into(), Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn not(&mut self) -> R<Ast> {
        if self.eat_op(&["!"]).is_some() || self.eat_word(&["not"]).is_some() {
            return Ok(Ast::Unary("not".into(), Box::new(self.not()?)));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> R<Ast> {
        let left = self.add()?;
        if let Some(op) = self.eat_op(&["==", "!=", "<", "<=", ">", ">="]) {
            let right = self.add()?;
            return Ok(Ast::Bin(op, Box::new(left), Box::new(right)));
        }
        if self.eat_word(&["in"]).is_some() {
            let right = self.add()?;
            return Ok(Ast::Bin("in".into(), Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn add(&mut self) -> R<Ast> {
        let mut left = self.mul()?;
        while let Some(op) = self.eat_op(&["+", "-"]) {
            let right = self.mul()?;
            left = Ast::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn mul(&mut self) -> R<Ast> {
        let mut left = self.unary()?;
        while let Some(op) = self.eat_op(&["*", "/", "%"]) {
            let right = self.unary()?;
            left = Ast::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> R<Ast> {
        if self.eat_op(&["-"]).is_some() {
            return Ok(Ast::Unary("-".into(), Box::new(self.unary()?)));
        }
        self.postfix()
    }

    /// `primary ( '.' ident )*` — so a call or a parenthesised expression can
    /// be indexed into, e.g. `artifact('taxonomy').digest`.
    fn postfix(&mut self) -> R<Ast> {
        let mut node = self.primary()?;
        while matches!(self.peek(), Tok::Dot) {
            self.i += 1;
            match self.next() {
                Tok::Ident(field) => node = Ast::Member(Box::new(node), field),
                t => {
                    return Err(EvalError::new(
                        "HX-3101",
                        format!("expected identifier after '.', got {t:?}"),
                    ));
                }
            }
        }
        Ok(node)
    }

    fn primary(&mut self) -> R<Ast> {
        match self.next() {
            Tok::Num(n) => Ok(Ast::Lit(Value::Num(n))),
            Tok::Str(s) => Ok(Ast::Lit(Value::Str(s))),
            Tok::LParen => {
                let e = self.expr()?;
                match self.next() {
                    Tok::RParen => Ok(e),
                    t => Err(EvalError::new(
                        "HX-3101",
                        format!("expected ')' , got {t:?}"),
                    )),
                }
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                if !matches!(self.peek(), Tok::RBracket) {
                    loop {
                        items.push(self.expr()?);
                        if matches!(self.peek(), Tok::Comma) {
                            self.i += 1;
                        } else {
                            break;
                        }
                    }
                }
                match self.next() {
                    Tok::RBracket => Ok(Ast::Array(items)),
                    t => Err(EvalError::new(
                        "HX-3101",
                        format!("expected ']', got {t:?}"),
                    )),
                }
            }
            Tok::Ident(w) => {
                match w.as_str() {
                    "true" => return Ok(Ast::Lit(Value::Bool(true))),
                    "false" => return Ok(Ast::Lit(Value::Bool(false))),
                    "null" => return Ok(Ast::Lit(Value::Null)),
                    _ => {}
                }
                // function call?
                if matches!(self.peek(), Tok::LParen) {
                    self.i += 1;
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            args.push(self.expr()?);
                            if matches!(self.peek(), Tok::Comma) {
                                self.i += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    match self.next() {
                        Tok::RParen => {}
                        t => {
                            return Err(EvalError::new(
                                "HX-3101",
                                format!("expected ')' after arguments, got {t:?}"),
                            ));
                        }
                    }
                    return Ok(Ast::Call(w, args));
                }
                // dotted path
                let mut parts = vec![w];
                while matches!(self.peek(), Tok::Dot) {
                    self.i += 1;
                    match self.next() {
                        Tok::Ident(p) => parts.push(p),
                        t => {
                            return Err(EvalError::new(
                                "HX-3101",
                                format!("expected identifier after '.', got {t:?}"),
                            ));
                        }
                    }
                }
                Ok(Ast::Path(parts))
            }
            t => Err(EvalError::new(
                "HX-3101",
                format!("unexpected token {t:?} in expression"),
            )),
        }
    }
}

// ---------------------------------------------------------------- eval

fn num(v: &Value, op: &str) -> R<f64> {
    match v {
        Value::Num(n) => Ok(*n),
        other => Err(EvalError::new(
            "HX-4106",
            format!(
                "operator '{op}' needs a number, got {} ({})",
                other,
                other.type_name()
            ),
        )),
    }
}

fn eval(a: &Ast, ctx: &Ctx) -> R<Value> {
    match a {
        Ast::Lit(v) => Ok(v.clone()),

        Ast::Array(items) => Ok(Value::Array(
            items.iter().map(|i| eval(i, ctx)).collect::<R<Vec<_>>>()?,
        )),

        Ast::Path(parts) => resolve_path(parts, ctx),

        Ast::Call(name, args) => {
            let a: Vec<Value> = args.iter().map(|x| eval(x, ctx)).collect::<R<Vec<_>>>()?;
            call(name, &a, ctx)
        }

        Ast::Member(base, field) => {
            let v = eval(base, ctx)?;
            descend(v, std::slice::from_ref(field))
        }

        Ast::Unary(op, inner) => {
            let v = eval(inner, ctx)?;
            match op.as_str() {
                "not" => Ok(Value::Bool(!v.truthy()?)),
                "-" => Ok(Value::Num(-num(&v, "-")?)),
                _ => Err(EvalError::new("HX-3101", format!("unknown unary '{op}'"))),
            }
        }

        Ast::Bin(op, l, r) => {
            // Short-circuit, so `a != null and a.b > 1` is safe to write.
            match op.as_str() {
                "and" => {
                    return Ok(Value::Bool(
                        eval(l, ctx)?.truthy()? && eval(r, ctx)?.truthy()?,
                    ));
                }
                "or" => {
                    return Ok(Value::Bool(
                        eval(l, ctx)?.truthy()? || eval(r, ctx)?.truthy()?,
                    ));
                }
                "??" => {
                    let lv = eval(l, ctx)?;
                    return Ok(if lv == Value::Null { eval(r, ctx)? } else { lv });
                }
                _ => {}
            }

            let lv = eval(l, ctx)?;
            let rv = eval(r, ctx)?;

            match op.as_str() {
                // §10.5 — NO type coercion. '1' == 1 is false. Coercion between
                // a string and a number is the classic reason a threshold
                // silently never matches, and here that threshold might be an
                // approval limit.
                "==" => Ok(Value::Bool(lv == rv)),
                "!=" => Ok(Value::Bool(lv != rv)),

                "<" | "<=" | ">" | ">=" => {
                    // Ordered comparison is defined for numbers and for strings.
                    let ord = match (&lv, &rv) {
                        (Value::Num(a), Value::Num(b)) => a.partial_cmp(b),
                        (Value::Str(a), Value::Str(b)) => Some(a.cmp(b)),
                        _ => {
                            return Err(EvalError::new(
                                "HX-4106",
                                format!(
                                    "cannot order {} ({}) against {} ({})",
                                    lv,
                                    lv.type_name(),
                                    rv,
                                    rv.type_name()
                                ),
                            ));
                        }
                    };
                    let Some(ord) = ord else {
                        return Err(EvalError::new("HX-4106", "comparison against NaN"));
                    };
                    Ok(Value::Bool(match op.as_str() {
                        "<" => ord.is_lt(),
                        "<=" => ord.is_le(),
                        ">" => ord.is_gt(),
                        _ => ord.is_ge(),
                    }))
                }

                "+" => match (&lv, &rv) {
                    (Value::Str(a), b) => Ok(Value::Str(format!("{a}{b}"))),
                    (a, Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
                    _ => Ok(Value::Num(num(&lv, "+")? + num(&rv, "+")?)),
                },
                "-" => Ok(Value::Num(num(&lv, "-")? - num(&rv, "-")?)),
                "*" => Ok(Value::Num(num(&lv, "*")? * num(&rv, "*")?)),
                "/" => {
                    let d = num(&rv, "/")?;
                    if d == 0.0 {
                        return Err(EvalError::new("HX-4106", "division by zero"));
                    }
                    Ok(Value::Num(num(&lv, "/")? / d))
                }
                "%" => {
                    let d = num(&rv, "%")?;
                    if d == 0.0 {
                        return Err(EvalError::new("HX-4106", "modulo by zero"));
                    }
                    Ok(Value::Num(num(&lv, "%")? % d))
                }

                "in" => match &rv {
                    Value::Array(items) => Ok(Value::Bool(items.contains(&lv))),
                    Value::Str(s) => Ok(Value::Bool(s.contains(&lv.to_string()))),
                    Value::Map(m) => Ok(Value::Bool(m.contains_key(&lv.to_string()))),
                    other => Err(EvalError::new(
                        "HX-4106",
                        format!(
                            "'in' needs an array, string or map on the right, got {}",
                            other.type_name()
                        ),
                    )),
                },

                _ => Err(EvalError::new(
                    "HX-3101",
                    format!("unknown operator '{op}'"),
                )),
            }
        }
    }
}

fn resolve_path(parts: &[String], ctx: &Ctx) -> R<Value> {
    // A loop iteration variable, possibly indexed into: `grasp`, `row.id`
    if let Some(v) = ctx.vars.get(&parts[0]) {
        return descend(v.clone(), &parts[1..]);
    }
    if parts[0] == "config" && parts.len() >= 2 {
        return Ok(ctx.config.get(&parts[1]).cloned().unwrap_or(Value::Null));
    }
    if parts.len() >= 2 {
        if let Some(ports) = ctx.outputs.get(&parts[0]) {
            let v = ports.get(&parts[1]).cloned().unwrap_or(Value::Null);
            return descend(v, &parts[2..]);
        }
        // §5.6 — a SKIPPED producer's outputs are UNAVAILABLE. Reporting that
        // distinctly matters: "the node was skipped" and "the node returned
        // null" are different facts, and only one of them is a design error.
        if ctx.skipped.contains(&parts[0]) {
            return Err(EvalError::new(
                "HX-4101",
                format!(
                    "'{}.{}' is unavailable because node '{}' was SKIPPED",
                    parts[0], parts[1], parts[0]
                ),
            ));
        }
    }
    // §10.7 — an unresolved reference is Null rather than an error, so that
    // `${x != null}` is expressible.
    Ok(Value::Null)
}

fn descend(mut v: Value, rest: &[String]) -> R<Value> {
    for key in rest {
        v = match v {
            Value::Map(m) => m.get(key).cloned().unwrap_or(Value::Null),
            Value::Null => Value::Null,
            other => {
                return Err(EvalError::new(
                    "HX-4106",
                    format!("cannot read '{key}' from {} ({})", other, other.type_name()),
                ));
            }
        };
    }
    Ok(v)
}

/// §10.6 — the builtin set is CLOSED in 1.0. An unknown function is HX-3105.
/// A runtime must not offer extras in the core namespace: a document that runs
/// on one runtime and fails validation on another is exactly the
/// interoperability failure the specification exists to prevent.
fn call(name: &str, a: &[Value], ctx: &Ctx) -> R<Value> {
    let arity = |n: usize| -> R<()> {
        if a.len() == n {
            Ok(())
        } else {
            Err(EvalError::new(
                "HX-4106",
                format!("{name}() takes {n} argument(s), got {}", a.len()),
            ))
        }
    };
    match name {
        "len" => {
            arity(1)?;
            Ok(Value::Num(match &a[0] {
                Value::Str(s) => s.chars().count() as f64,
                Value::Array(v) => v.len() as f64,
                Value::Map(m) => m.len() as f64,
                Value::Null => 0.0,
                other => {
                    return Err(EvalError::new(
                        "HX-4106",
                        format!(
                            "len() needs a string, array or map, got {}",
                            other.type_name()
                        ),
                    ));
                }
            }))
        }
        "empty" => {
            arity(1)?;
            Ok(Value::Bool(match &a[0] {
                Value::Str(s) => s.is_empty(),
                Value::Array(v) => v.is_empty(),
                Value::Map(m) => m.is_empty(),
                Value::Null => true,
                _ => false,
            }))
        }
        "has" => {
            arity(2)?;
            let k = a[1].to_string();
            Ok(Value::Bool(match &a[0] {
                Value::Map(m) => m.contains_key(&k),
                _ => false,
            }))
        }
        "lower" => {
            arity(1)?;
            Ok(Value::Str(a[0].to_string().to_lowercase()))
        }
        "upper" => {
            arity(1)?;
            Ok(Value::Str(a[0].to_string().to_uppercase()))
        }
        "contains" => {
            arity(2)?;
            Ok(Value::Bool(match &a[0] {
                Value::Str(s) => s.contains(&a[1].to_string()),
                Value::Array(v) => v.contains(&a[1]),
                _ => false,
            }))
        }
        "abs" => {
            arity(1)?;
            Ok(Value::Num(num(&a[0], "abs")?.abs()))
        }
        "round" => {
            arity(1)?;
            Ok(Value::Num(num(&a[0], "round")?.round()))
        }
        "min" => {
            arity(2)?;
            Ok(Value::Num(num(&a[0], "min")?.min(num(&a[1], "min")?)))
        }
        "max" => {
            arity(2)?;
            Ok(Value::Num(num(&a[0], "max")?.max(num(&a[1], "max")?)))
        }
        "artifact" => {
            arity(1)?;
            Ok(ctx
                .artifacts
                .get(&a[0].to_string())
                .map(|m| Value::Map(m.clone()))
                .unwrap_or(Value::Null))
        }
        "resource" => {
            arity(1)?;
            Ok(ctx
                .resources
                .get(&a[0].to_string())
                .map(|m| Value::Map(m.clone()))
                .unwrap_or(Value::Null))
        }
        _ => Err(EvalError::new(
            "HX-3105",
            format!("unknown function '{name}()' — the builtin set is closed in 1.0"),
        )),
    }
}

// ---------------------------------------------------------------- public API

/// Evaluate a bare expression (the inside of a `${ }`).
pub fn eval_expr(src: &str, ctx: &Ctx) -> R<Value> {
    let toks = lex(src)?;
    let mut p = Parser { t: toks, i: 0 };
    let ast = p.expr()?;
    if !matches!(p.peek(), Tok::Eof) {
        return Err(EvalError::new(
            "HX-3101",
            format!("trailing input after expression: {:?}", p.peek()),
        ));
    }
    eval(&ast, ctx)
}

/// Evaluate an attribute value, which may be a literal, a single `${expr}`, or
/// text with expressions interpolated into it (§10.2).
///
/// A value that is EXACTLY one `${...}` keeps its type — so `${count}` yields a
/// number rather than the string "3". Mixed text always yields a string.
pub fn eval_interpolated(src: &str, ctx: &Ctx) -> R<Value> {
    let trimmed = src.trim();
    if trimmed.starts_with("${") && trimmed.ends_with('}') {
        let inner = &trimmed[2..trimmed.len() - 1];
        // Guard against "${a} ${b}" being mistaken for one expression.
        if !inner.contains("${") {
            return eval_expr(inner, ctx);
        }
    }
    if !src.contains("${") {
        return Ok(Value::Str(src.to_string()));
    }

    let mut out = String::new();
    let mut rest = src;
    while let Some(start) = rest.find("${") {
        // `$${` escapes a literal `${`
        if start > 0 && rest.as_bytes()[start - 1] == b'$' {
            out.push_str(&rest[..start - 1]);
            out.push_str("${");
            rest = &rest[start + 2..];
            continue;
        }
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            return Err(EvalError::new("HX-3101", "unterminated ${ in value"));
        };
        out.push_str(&eval_expr(&after[..end], ctx)?.to_string());
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(Value::Str(out))
}

/// Evaluate something that MUST be a boolean — a guard, case or edge condition.
pub fn eval_condition(src: &str, ctx: &Ctx) -> R<bool> {
    eval_interpolated(src, ctx)?.truthy()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        let mut c = Ctx::default();
        let mut classify = HashMap::new();
        classify.insert("confidence".to_string(), Value::Num(0.93));
        classify.insert("category".to_string(), Value::Str("invoice".into()));
        c.outputs.insert("classify".to_string(), classify);
        c.vars.insert("item".to_string(), Value::Num(7.0));
        c.config.insert("threshold".to_string(), Value::Num(0.9));
        let mut art = BTreeMap::new();
        art.insert("digest".to_string(), Value::Str("sha256:abc".into()));
        c.artifacts.insert("taxonomy".to_string(), art);
        c
    }

    fn b(src: &str) -> bool {
        eval_condition(src, &ctx()).unwrap()
    }

    #[test]
    fn comparisons_and_logic() {
        assert!(b("${classify.confidence >= 0.90}"));
        assert!(!b("${classify.confidence < 0.5}"));
        assert!(b(
            "${classify.confidence > 0.9 and classify.category == 'invoice'}"
        ));
        assert!(b("${not (classify.confidence < 0.5)}"));
        assert!(b("${classify.category in ['invoice','receipt']}"));
    }

    #[test]
    fn no_type_coercion() {
        // §10.5 — the rule that stops a threshold silently never matching.
        assert!(!b("${'1' == 1}"));
        assert!(b("${'1' != 1}"));
    }

    #[test]
    fn null_is_not_false() {
        // A missing value must not quietly take the otherwise-branch.
        let e = eval_condition("${missing.port}", &ctx()).unwrap_err();
        assert_eq!(e.code, "HX-4107", "{}", e.message);
    }

    #[test]
    fn null_comparison_is_allowed_but_arithmetic_is_not() {
        assert!(b("${missing.port == null}"));
        assert!(b("${classify.category != null}"));
        let e = eval_expr("missing.port + 1", &ctx()).unwrap_err();
        assert_eq!(e.code, "HX-4106");
    }

    #[test]
    fn coalesce_and_config() {
        assert_eq!(
            eval_expr("config.threshold", &ctx()).unwrap(),
            Value::Num(0.9)
        );
        assert_eq!(
            eval_expr("missing.x ?? 42", &ctx()).unwrap(),
            Value::Num(42.0)
        );
    }

    #[test]
    fn loop_variable_and_artifact() {
        assert_eq!(eval_expr("item", &ctx()).unwrap(), Value::Num(7.0));
        assert_eq!(
            eval_expr("artifact('taxonomy').digest", &ctx()).unwrap(),
            Value::Str("sha256:abc".into())
        );
    }

    #[test]
    fn unknown_function_is_hx_3105() {
        let e = eval_expr("now()", &ctx()).unwrap_err();
        assert_eq!(e.code, "HX-3105");
    }

    #[test]
    fn builtins() {
        assert_eq!(eval_expr("len('abcd')", &ctx()).unwrap(), Value::Num(4.0));
        assert_eq!(eval_expr("max(2, 9)", &ctx()).unwrap(), Value::Num(9.0));
        assert_eq!(
            eval_expr("upper('ok')", &ctx()).unwrap(),
            Value::Str("OK".into())
        );
    }

    #[test]
    fn interpolation_keeps_type_when_whole_value_is_one_expression() {
        let c = ctx();
        assert_eq!(eval_interpolated("${item}", &c).unwrap(), Value::Num(7.0));
        assert_eq!(
            eval_interpolated("n=${item}!", &c).unwrap(),
            Value::Str("n=7!".into())
        );
        assert_eq!(
            eval_interpolated("plain text", &c).unwrap(),
            Value::Str("plain text".into())
        );
    }

    #[test]
    fn skipped_producer_is_reported_distinctly() {
        let mut c = ctx();
        c.skipped.push("match_po".into());
        let e = eval_expr("match_po.matched", &c).unwrap_err();
        assert_eq!(e.code, "HX-4101");
        assert!(e.message.contains("SKIPPED"));
    }

    #[test]
    fn malformed_expression_is_hx_3101() {
        assert_eq!(eval_expr("1 +", &ctx()).unwrap_err().code, "HX-3101");
        assert_eq!(eval_expr("(1", &ctx()).unwrap_err().code, "HX-3101");
    }
}

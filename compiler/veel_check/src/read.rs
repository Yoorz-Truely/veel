//! Reading one node against its kind: its handle, its arguments, its attributes on its line and
//! as child lines, and the kinds its block holds, with the diagnostics of each mistake.

use std::collections::HashMap;

use veel_schema::{Arg, Kind, Value as Form, is_kebab, slice};
use veel_syntax::ast::{Entry, Node, Value};
use veel_syntax::{Diagnostic, FileId, Span, SyntaxToken, TextEdit, closest};

use crate::codes;

/// Where diagnostics of one file go, with the file's text for fixes.
pub(crate) struct Sink<'t> {
    pub file: FileId,
    pub text: &'t str,
    pub out: Vec<Diagnostic>,
}

impl Sink<'_> {
    /// An error at `span` of this file.
    pub fn error(&self, code: &'static str, span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(code, self.file, span, message)
    }

    /// Reports `d`.
    pub fn push(&mut self, d: Diagnostic) {
        self.out.push(d);
    }

    /// `span` widened to delete it cleanly: a whole line when only whitespace stands before it on
    /// its line and it ends the line, otherwise with the whitespace before it.
    pub fn deletion(&self, span: Span) -> Span {
        let before = &self.text[..span.start];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        if before[line_start..].trim().is_empty() && self.text[..span.end].ends_with('\n') {
            return Span::new(line_start, span.end);
        }
        let ws = before.len() - before.trim_end_matches([' ', '\t']).len();
        Span::new(span.start - ws, span.end)
    }

    /// A fix that replaces `span` of this file with `with`.
    pub fn edit(&self, span: Span, with: impl Into<String>) -> Vec<TextEdit> {
        vec![TextEdit {
            file: self.file,
            span,
            replacement: with.into(),
        }]
    }
}

/// An attribute or parameter as written: on its owner's line, or as a child line.
#[derive(Clone, Copy)]
pub(crate) struct Written<'a> {
    /// Its name's span.
    pub key: Span,
    /// Its value; [`Value::Missing`] for a child line with none.
    pub value: Value<'a>,
    /// The child line, when it is one.
    pub line: Option<Node<'a>>,
}

impl Written<'_> {
    /// Whether it is a child line with no value (a switch written alone, or a keyframe block).
    pub fn alone(&self) -> bool {
        matches!(self.value, Value::Missing(_))
    }
}

/// What a node holds, read against its kind.
pub(crate) struct Fields<'a> {
    /// Its handle token, `@name`.
    pub handle: Option<SyntaxToken<'a>>,
    /// Its arguments, by the name of the kind's argument they are.
    pub args: HashMap<&'static str, Value<'a>>,
    /// Its attributes and parameters, by name.
    pub attrs: HashMap<String, Written<'a>>,
    /// The child kinds of its block, in order.
    pub children: Vec<Node<'a>>,
}

impl<'a> Fields<'a> {
    /// The handle's name, without `@`, when it is a kebab-case handle.
    pub fn handle_name(&self) -> Option<&'a str> {
        self.handle.map(|t| &t.text[1..]).filter(|h| is_kebab(h))
    }
}

/// How a node is named in messages: its kind and its handle.
pub(crate) fn describe(kind: &str, node: Node<'_>) -> String {
    match node.handle() {
        Some(h) => format!("{kind} {}", h.text),
        None => format!("the {kind}"),
    }
}

/// `word` in kebab-case, if lowering it and turning other characters into hyphens makes one.
pub(crate) fn kebab(word: &str) -> Option<String> {
    let mut out = String::new();
    for c in word.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    is_kebab(&out).then_some(out)
}

/// A sample of a value of `form`, for messages.
pub(crate) fn sample(form: Form) -> &'static str {
    match form {
        Form::Text => "\"Name\"",
        Form::Path => "\"media/shot.mov\"",
        Form::Hash => "sha256:…",
        Form::Rate => "25",
        Form::Size => "[1920 1080]",
        Form::SourceLength => "600s",
        Form::Position => "0",
        Form::SourceRange => "0..+5s",
        Form::Length => "12",
        Form::ClipTime => "0",
        Form::Timecode => "00:00:00:00",
        Form::Handle(_) => "@source",
        Form::Word(words) => words.first().copied().unwrap_or("…"),
        Form::OnOff => "on",
        Form::Rect => "[0 0 1920 1080]",
        Form::Point => "[0.5 0.5]",
        Form::Angle => "0deg",
        Form::Choice => "…",
        Form::Parameter => "…",
    }
}

/// Every kind name of the slice, for telling a misplaced kind from an unknown name.
fn known_kind(name: &str) -> bool {
    slice::kind(name).is_some()
}

/// Reads `node` as `kind`. `params` are the names an effect's line and block may set besides
/// its kind's attributes; a child line of one is returned for its owner to read, block and all.
/// `None` takes any name as a parameter, unreported: an effect whose asset is unknown.
pub(crate) fn read<'a>(
    sink: &mut Sink<'_>,
    node: Node<'a>,
    kind: &'static Kind,
    params: Option<&[&'static str]>,
) -> Fields<'a> {
    let any = params.is_none();
    let params = params.unwrap_or(&[]);
    let owner = describe(kind.name, node);
    let name_span = node.name_token().map_or(node.span(), |t| t.span);
    let mut fields = Fields {
        handle: None,
        args: HashMap::new(),
        attrs: HashMap::new(),
        children: Vec::new(),
    };
    let entries: Vec<Entry<'a>> = node.entries().collect();
    let mut rest = entries.as_slice();
    if kind.handle {
        match entries.first() {
            Some(Entry::Arg(Value::Word(t))) if t.text.starts_with('@') => {
                fields.handle = Some(*t);
                rest = &entries[1..];
                let name = &t.text[1..];
                if !is_kebab(name) {
                    let mut d = sink.error(
                        codes::HANDLE,
                        t.span,
                        format!(
                            "`{}` is not a handle: a handle is `@` and a kebab-case name",
                            t.text
                        ),
                    );
                    if let Some(k) = kebab(name) {
                        d = d.with_fix(format!("write `@{k}`"), sink.edit(t.span, format!("@{k}")));
                    }
                    sink.push(d);
                }
            }
            _ => {
                let d = sink.error(
                    codes::MISSING,
                    name_span,
                    format!(
                        "the {0} has no handle: write `{0} @name`; a handle is required here",
                        kind.name
                    ),
                );
                sink.push(d);
            }
        }
    }
    let attr_names: Vec<&'static str> = kind.attributes.iter().map(|a| a.name).collect();
    let mut next_arg = 0;
    for entry in rest {
        match *entry {
            Entry::Arg(v) => {
                let mut placed = false;
                while let Some(arg) = kind.args.get(next_arg) {
                    next_arg += 1;
                    let is_string = matches!(v, Value::Str(_) | Value::Raw(_));
                    if !arg.required && arg.value == Form::Text && !is_string {
                        continue;
                    }
                    fields.args.insert(arg.name, v);
                    placed = true;
                    break;
                }
                if !placed {
                    stray(sink, &owner, kind, v.span(), v.text());
                }
            }
            Entry::Prop { key, value } | Entry::Keyword { key, value } => {
                let name = veel_syntax::ast::scalar_text(key).into_owned();
                if !is_kebab(&name) {
                    continue; // `veel-name`, from the syntax
                }
                if any || attr_names.contains(&name.as_str()) || params.contains(&name.as_str()) {
                    let w = Written {
                        key: key.span,
                        value,
                        line: None,
                    };
                    record(
                        sink,
                        &mut fields,
                        &owner,
                        name,
                        w,
                        entry_span(key.span, value),
                    );
                } else {
                    unknown_property(sink, &owner, &name, key.span, &attr_names, params);
                }
            }
        }
    }
    for arg in &kind.args[next_arg.min(kind.args.len())..] {
        if arg.required && !fields.args.contains_key(arg.name) {
            missing_arg(sink, &owner, kind, arg, name_span);
        }
    }
    for child in node.children() {
        let Some(token) = child.name_token() else {
            continue;
        };
        let name = child.name().into_owned();
        if !is_kebab(&name) && !name.starts_with('@') {
            continue; // `veel-name`, from the syntax
        }
        if any || attr_names.contains(&name.as_str()) || params.contains(&name.as_str()) {
            let is_param = any || params.contains(&name.as_str());
            let w = child_line(sink, child, token.span, is_param);
            let span = child.content_span();
            record(sink, &mut fields, &owner, name, w, span);
        } else if kind.children.contains(&name.as_str()) {
            fields.children.push(child);
        } else {
            misplaced(sink, &owner, kind, &name, token.span, &attr_names, params);
        }
    }
    for a in kind.attributes {
        if a.required && !fields.attrs.contains_key(a.name) {
            let d = sink.error(
                codes::MISSING,
                name_span,
                format!(
                    "{owner} has no `{}`: add `{} {}` ({})",
                    a.name,
                    a.name,
                    sample(a.value),
                    a.doc.to_lowercase()
                ),
            );
            sink.push(d);
        }
    }
    fields
}

/// The span of a `key=value` or keyword entry.
fn entry_span(key: Span, value: Value<'_>) -> Span {
    key.cover(value.span())
}

/// Reads an attribute's child line: `name value`, or `name` alone. A parameter's line may also
/// carry `time=` and a block, which its effect reads.
fn child_line<'a>(sink: &mut Sink<'_>, line: Node<'a>, key: Span, is_param: bool) -> Written<'a> {
    let mut value = Value::Missing(Span::empty(key.end));
    for entry in line.entries() {
        match entry {
            Entry::Arg(v) if matches!(value, Value::Missing(_)) => value = v,
            Entry::Arg(v) => {
                let d = sink
                    .error(
                        codes::TYPE,
                        v.span(),
                        format!(
                            "`{}` takes one value; `{}` is one too many",
                            line.name(),
                            v.text()
                        ),
                    )
                    .with_fix("remove it", sink.edit(sink.deletion(v.span()), ""));
                sink.push(d);
            }
            Entry::Prop { key: k, .. } | Entry::Keyword { key: k, .. } if !is_param => {
                let d = sink.error(
                    codes::UNKNOWN_PROPERTY,
                    k.span,
                    format!(
                        "`{}` is a value and has no property `{}`",
                        line.name(),
                        veel_syntax::ast::scalar_text(k)
                    ),
                );
                sink.push(d);
            }
            Entry::Prop { .. } | Entry::Keyword { .. } => {}
        }
    }
    if !is_param && let Some(block) = line.block() {
        let d = sink.error(
            codes::TYPE,
            block.span(),
            format!("`{}` is a value and takes no block", line.name()),
        );
        sink.push(d);
    }
    Written {
        key,
        value,
        line: Some(line),
    }
}

/// Records `name` as written at `span`, or reports it given twice.
fn record<'a>(
    sink: &mut Sink<'_>,
    fields: &mut Fields<'a>,
    owner: &str,
    name: String,
    w: Written<'a>,
    span: Span,
) {
    if let Some(first) = fields.attrs.get(&name) {
        let d = sink
            .error(
                codes::REPEATED,
                w.key,
                format!("{owner} sets `{name}` twice; write it once"),
            )
            .with_label(sink.file, first.key, "first set here")
            .with_fix("remove this one", sink.edit(sink.deletion(span), ""));
        sink.push(d);
        return;
    }
    fields.attrs.insert(name, w);
}

/// Reports a value with no argument left for it.
fn stray(sink: &mut Sink<'_>, owner: &str, kind: &Kind, span: Span, text: &str) {
    let takes = if kind.args.is_empty() {
        "no value after its handle".to_string()
    } else {
        kind.args
            .iter()
            .map(|a| format!("its {}", a.name))
            .collect::<Vec<_>>()
            .join(", then ")
    };
    let mut d = sink.error(
        codes::TYPE,
        span,
        format!(
            "`{text}` is one value too many: a {} takes {takes}",
            kind.name
        ),
    );
    if !owner.starts_with("the ") {
        d = d.with_note(format!("in {owner}"));
    }
    let d = d.with_fix("remove it", sink.edit(sink.deletion(span), ""));
    sink.push(d);
}

/// Reports a required argument left out.
fn missing_arg(sink: &mut Sink<'_>, owner: &str, kind: &Kind, arg: &Arg, at: Span) {
    let d = sink.error(
        codes::MISSING,
        at,
        format!(
            "{owner} has no {}: write it after the {}, as `{}`",
            arg.name,
            if kind.handle { "handle" } else { "kind" },
            sample(arg.value)
        ),
    );
    sink.push(d);
}

/// Reports a property its owner does not have, with the nearest name it does have.
fn unknown_property(
    sink: &mut Sink<'_>,
    owner: &str,
    name: &str,
    span: Span,
    attrs: &[&'static str],
    params: &[&'static str],
) {
    let names: Vec<&str> = attrs.iter().chain(params).copied().collect();
    let mut d = sink.error(
        codes::UNKNOWN_PROPERTY,
        span,
        format!("{owner} has no property `{name}`{}", list(&names)),
    );
    if let Some(c) = closest(name, names.iter().copied()) {
        d = d.with_fix(format!("write `{c}`"), sink.edit(span, c));
    }
    sink.push(d);
}

/// Reports a child line that is neither an attribute nor a kind its owner holds.
fn misplaced(
    sink: &mut Sink<'_>,
    owner: &str,
    kind: &Kind,
    name: &str,
    span: Span,
    attrs: &[&'static str],
    params: &[&'static str],
) {
    let holds: Vec<&str> = attrs
        .iter()
        .chain(params)
        .chain(kind.children)
        .copied()
        .collect();
    if known_kind(name) {
        let d = sink.error(
            codes::KIND,
            span,
            format!(
                "a `{name}` cannot stand in a `{}`{}",
                kind.name,
                list(&holds)
            ),
        );
        sink.push(d);
        return;
    }
    let near = closest(name, holds.iter().copied());
    let code = match near {
        Some(c) if kind.children.contains(&c) => codes::KIND,
        Some(_) => codes::UNKNOWN_PROPERTY,
        None if kind.children.is_empty() => codes::UNKNOWN_PROPERTY,
        None => codes::KIND,
    };
    let message = if code == codes::KIND {
        format!("`{name}` is not a kind {owner} holds{}", list(&holds))
    } else {
        format!("{owner} has no property `{name}`{}", list(&holds))
    };
    let mut d = sink.error(code, span, message);
    if let Some(c) = near {
        d = d.with_fix(format!("write `{c}`"), sink.edit(span, c));
    }
    sink.push(d);
}

/// `; it takes `a`, `b` and `c``, or nothing for no names.
pub(crate) fn list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => format!("; it takes `{one}`"),
        _ => {
            let (last, rest) = names.split_last().expect("two or more");
            let rest: Vec<String> = rest.iter().map(|n| format!("`{n}`")).collect();
            format!("; it takes {} and `{last}`", rest.join(", "))
        }
    }
}

/// Reports a property on a parameter's line other than a keyframe block's `time=`.
pub(crate) fn unknown_on_block(sink: &mut Sink<'_>, param: &str, key: SyntaxToken<'_>) {
    let d = sink.error(
        codes::UNKNOWN_PROPERTY,
        key.span,
        format!(
            "`{param}`'s line has no property `{}`; a block of keys may carry `time=`",
            veel_syntax::ast::scalar_text(key)
        ),
    );
    sink.push(d);
}

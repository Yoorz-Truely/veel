//! The canonical order: keyword attributes, each line's properties and each block's lines in the
//! schema's order, then the layout.

use veel_schema::{Kind, slice};
use veel_syntax::{NodeKind, SyntaxElement, SyntaxNode, TokenKind, ast, parse};

use crate::format;

/// What a node is, for the order of what it holds.
#[derive(Clone, Copy)]
enum Role {
    /// A node of a kind of the slice.
    Kind(&'static Kind),
    /// An effect, whose line and block hold its parameters.
    Effect(Option<&'static veel_schema::Effect>),
    /// A parameter's line, whose block holds `key` lines.
    Param,
    /// Anything else: kept as written.
    Other,
}

/// The rank of a line or property named `name` in what `role` holds; `None` when the schema does
/// not know it there, so it goes last.
fn rank(role: Role, name: &str) -> Option<usize> {
    match role {
        Role::Kind(k) => k
            .attributes
            .iter()
            .position(|a| a.name == name)
            .or_else(|| {
                k.children
                    .iter()
                    .position(|c| *c == name)
                    .map(|i| k.attributes.len() + i)
            }),
        Role::Effect(e) => e.and_then(|e| e.loaded_params().position(|p| p.name == name)),
        Role::Param => match name {
            "time" => Some(0),
            "key" => Some(1),
            _ => None,
        },
        Role::Other => None,
    }
}

/// The role of a child line named `name` in what `owner` holds.
fn child_role(owner: Role, line: ast::Node<'_>) -> Role {
    let name = line.name();
    match owner {
        Role::Kind(k) if k.children.contains(&name.as_ref()) => match name.as_ref() {
            "effect" => Role::Effect(
                line.args()
                    .nth(1)
                    .and_then(|v| v.as_word())
                    .and_then(slice::effect),
            ),
            other => slice::kind(other).map_or(Role::Other, Role::Kind),
        },
        Role::Effect(Some(e)) if e.param(&name).is_some() => Role::Param,
        Role::Param if name == "key" => slice::kind("key").map_or(Role::Other, Role::Kind),
        _ => Role::Other,
    }
}

/// `text` in canonical form (the crate's documentation lists the rules). A file that does not
/// parse gets the layout alone.
pub fn canonical(text: &str) -> String {
    let file = parse(text);
    if !file.errors().is_empty() {
        return format(text);
    }
    let mut out = String::with_capacity(text.len());
    let root = file.root();
    for element in root.children() {
        match element {
            SyntaxElement::Token(t) => out.push_str(t.text),
            SyntaxElement::Node(n) if n.kind() == NodeKind::Node => {
                let line = ast::Node(n);
                let role = if line.is_commented_out() {
                    Role::Other
                } else {
                    match line.name().as_ref() {
                        "source" | "sequence" => {
                            slice::kind(&line.name()).map_or(Role::Other, Role::Kind)
                        }
                        _ => Role::Other,
                    }
                };
                out.push_str(&node_text(n, role));
            }
            SyntaxElement::Node(n) => out.push_str(n.text()),
        }
    }
    format(&out)
}

/// The text of a node in canonical order: its properties sorted among themselves, `at`, `to`
/// and `len` as keywords, its block's lines sorted.
fn node_text(node: SyntaxNode<'_>, role: Role) -> String {
    if matches!(role, Role::Other) {
        return node.text().to_string();
    }
    let props: Vec<SyntaxNode<'_>> = node
        .child_nodes()
        .filter(|n| matches!(n.kind(), NodeKind::Prop | NodeKind::Keyword))
        .collect();
    let mut sorted: Vec<(usize, String)> = props
        .iter()
        .enumerate()
        .map(|(i, p)| (i, prop_text(*p)))
        .collect();
    let key = |i: usize| {
        let name = props[i]
            .child_tokens()
            .next()
            .map(|t| ast::scalar_text(t).into_owned())
            .unwrap_or_default();
        rank(role, &name).unwrap_or(usize::MAX)
    };
    sorted.sort_by_key(|(i, _)| key(*i));
    let mut next_prop = sorted.into_iter().map(|(_, t)| t);
    let mut out = String::new();
    for element in node.children() {
        match element {
            SyntaxElement::Token(t) => out.push_str(t.text),
            SyntaxElement::Node(n) => match n.kind() {
                NodeKind::Prop | NodeKind::Keyword => {
                    out.push_str(&next_prop.next().expect("one text per property"));
                }
                NodeKind::Block => out.push_str(&block_text(n, role)),
                _ => out.push_str(n.text()),
            },
        }
    }
    out
}

/// A property's text, `at=12s` written as the keyword `at 12s` when its value is a word.
fn prop_text(prop: SyntaxNode<'_>) -> String {
    if prop.kind() == NodeKind::Prop {
        let tokens: Vec<_> = prop.child_tokens().collect();
        if let [key, eq, value] = tokens[..]
            && veel_syntax::KEYWORDS.contains(&key.text)
            && eq.kind == TokenKind::Equals
            && value.kind == TokenKind::Word
            && prop.child_nodes().next().is_none()
        {
            return format!("{} {}", key.text, value.text);
        }
    }
    prop.text().to_string()
}

/// One line of a block and the comments and space before it, which move with it.
struct Item {
    leading: String,
    body: String,
    rank: usize,
}

/// A block's text with its lines in canonical order.
fn block_text(block: SyntaxNode<'_>, owner: Role) -> String {
    let mut head = String::new();
    let mut items: Vec<Item> = Vec::new();
    let mut pending = String::new();
    let mut in_head = true;
    let mut has_comment = false;
    for element in block.children() {
        match element {
            SyntaxElement::Token(t) => {
                has_comment |= t.kind.is_comment();
                if in_head {
                    head.push_str(t.text);
                    if t.kind == TokenKind::Newline {
                        in_head = false;
                    }
                } else {
                    pending.push_str(t.text);
                }
            }
            SyntaxElement::Node(n) if n.kind() == NodeKind::Node => {
                in_head = false;
                let line = ast::Node(n);
                let (role, r) = if line.is_commented_out() {
                    (Role::Other, None)
                } else {
                    (child_role(owner, line), rank(owner, &line.name()))
                };
                items.push(Item {
                    leading: std::mem::take(&mut pending),
                    body: node_text(n, role),
                    rank: r.unwrap_or(usize::MAX),
                });
            }
            SyntaxElement::Node(n) => pending.push_str(n.text()),
        }
    }
    let tail = pending;
    let in_order = items.windows(2).all(|w| w[0].rank <= w[1].rank);
    if in_order {
        return join(head, items, tail);
    }
    if !block.text().contains('\n') {
        if has_comment {
            return join(head, items, tail);
        }
        items.sort_by_key(|i| i.rank);
        let bodies: Vec<String> = items
            .iter()
            .map(|i| {
                i.body
                    .trim_end()
                    .trim_end_matches(';')
                    .trim_end()
                    .to_string()
            })
            .collect();
        return format!("{{ {} }}", bodies.join("; "));
    }
    items.sort_by_key(|i| i.rank);
    for item in &mut items {
        if !item.body.ends_with('\n') && !item.body.trim_end().ends_with(';') {
            item.body.push('\n');
        }
    }
    join(head, items, tail)
}

fn join(head: String, items: Vec<Item>, tail: String) -> String {
    let mut out = head;
    for item in items {
        out.push_str(&item.leading);
        out.push_str(&item.body);
    }
    out.push_str(&tail);
    out
}

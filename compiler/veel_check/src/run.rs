//! The check of a whole project: the syntax of each file, then its sources, its sequences and
//! everything in them, then each track's overlaps and crossfades.

use std::collections::HashMap;

use veel_schema::{Value as Form, is_kebab, slice};
use veel_syntax::ast::{Node, Value};
use veel_syntax::{Diagnostic, FileId, Parse, Span, SyntaxToken, closest};
use veel_time::{Clock, Time, Timecode, Unit};

use crate::codes;
use crate::model::{Clip, Effect, Key, Param, ParamValue, Project, Sequence, Source, Track};
use crate::read::{Sink, describe, read};
use crate::values::{self, SourceClock};

/// The kinds a file may hold at its top.
const TOP: &[&str] = &["source", "sequence"];

fn kind(name: &str) -> &'static veel_schema::Kind {
    slice::kind(name).expect("a kind of the slice")
}

/// Every handle the project defines, by kind, for references and duplicates.
#[derive(Default)]
struct Handles {
    /// Every handle written on a node of each kind, defined or not yet, for "names a `track`".
    seen: HashMap<String, &'static str>,
    /// The handles defined so far, by kind, with where.
    defined: HashMap<(&'static str, String), (FileId, Span)>,
}

impl Handles {
    /// Defines `token`'s handle for `kind`, or reports it defined twice.
    fn define(&mut self, sink: &mut Sink<'_>, kind: &'static str, token: SyntaxToken<'_>) {
        let name = &token.text[1..];
        if !is_kebab(name) {
            return;
        }
        let key = (kind, name.to_string());
        if let Some(&(file, span)) = self.defined.get(&key) {
            let free = (2..)
                .map(|n| format!("{name}-{n}"))
                .find(|h| !self.defined.contains_key(&(kind, h.clone())))
                .expect("a free handle");
            let d = sink
                .error(
                    codes::HANDLE,
                    token.span,
                    format!("the {kind} handle `@{name}` is defined twice"),
                )
                .with_label(file, span, "first defined here")
                .with_fix(
                    format!("rename this one `@{free}`"),
                    sink.edit(token.span, format!("@{free}")),
                );
            sink.push(d);
            return;
        }
        self.defined.insert(key, (sink.file, token.span));
    }

    /// The defined handles of `kind`.
    fn of(&self, kind: &str) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .defined
            .keys()
            .filter(|(k, _)| *k == kind)
            .map(|(_, h)| h.as_str())
            .collect();
        out.sort();
        out
    }
}

/// A clip as the track's checks need it: its model, and where its words are.
struct Placed<'a> {
    clip: Clip,
    node: Node<'a>,
    at: Option<Value<'a>>,
    crossfade: Option<Value<'a>>,
    /// Its source's length in the sequence's frames, rounded down.
    source_frames: Option<i64>,
}

pub(crate) fn check(parses: &[Parse]) -> (Vec<Diagnostic>, Project) {
    let mut out = Vec::new();
    let mut project = Project::default();
    let mut broken = false;
    for (i, parse) in parses.iter().enumerate() {
        let found = veel_syntax::diagnostics(parse, FileId(i as u32));
        broken |= !parse.errors().is_empty();
        out.extend(found);
    }
    // One mistake gives one error: a file that does not parse is not read further, nor is any
    // file that might name what it defines.
    if broken {
        return (out, project);
    }

    let mut handles = Handles::default();
    let mut tops: Vec<(usize, Node<'_>)> = Vec::new();
    for (i, parse) in parses.iter().enumerate() {
        let mut sink = Sink {
            file: FileId(i as u32),
            text: parse.text(),
            out: Vec::new(),
        };
        for node in parse.document().nodes() {
            let name = node.name();
            if TOP.contains(&name.as_ref()) {
                tops.push((i, node));
                see(&mut handles, node);
            } else if is_kebab(&name) {
                top_misplaced(&mut sink, node, &name);
            }
        }
        out.extend(sink.out);
    }

    let mut sources: HashMap<String, Source> = HashMap::new();
    for &(i, node) in tops.iter().filter(|(_, n)| n.name() == "source") {
        let mut sink = sink_for(parses, i);
        if let Some(s) = source(&mut sink, &mut handles, node) {
            sources.entry(s.handle.clone()).or_insert_with(|| s.clone());
            project.sources.push(s);
        }
        out.extend(sink.out);
    }
    for &(i, node) in tops.iter().filter(|(_, n)| n.name() == "sequence") {
        let mut sink = sink_for(parses, i);
        if let Some(s) = sequence(&mut sink, &mut handles, &sources, node) {
            project.sequences.push(s);
        }
        out.extend(sink.out);
    }
    (out, project)
}

fn sink_for(parses: &[Parse], i: usize) -> Sink<'_> {
    Sink {
        file: FileId(i as u32),
        text: parses[i].text(),
        out: Vec::new(),
    }
}

/// Notes every handle a top-level node and its sequence's tracks and clips write.
fn see(handles: &mut Handles, node: Node<'_>) {
    let mut note = |kind: &'static str, n: Node<'_>| {
        if let Some(h) = n.handle() {
            handles.seen.entry(h.text[1..].to_string()).or_insert(kind);
        }
    };
    match node.name().as_ref() {
        "source" => note("source", node),
        "sequence" => {
            note("sequence", node);
            for t in node.children().filter(|c| c.name() == "track") {
                note("track", t);
                for c in t.children().filter(|c| c.name() == "clip") {
                    note("clip", c);
                }
            }
        }
        _ => {}
    }
}

/// Reports a node at the top of a file that is not a source or a sequence.
fn top_misplaced(sink: &mut Sink<'_>, node: Node<'_>, name: &str) {
    let span = node.name_token().map_or(node.span(), |t| t.span);
    if let Some(k) = slice::kind(name) {
        let inside = slice::KINDS
            .iter()
            .find(|o| o.children.contains(&k.name))
            .map_or("an effect's block".to_string(), |o| {
                format!("a `{}`", o.name)
            });
        let d = sink.error(
            codes::KIND,
            span,
            format!("a `{name}` stands in {inside}, not at the top of a file, which holds `source` and `sequence`"),
        );
        sink.push(d);
        return;
    }
    let mut d = sink.error(
        codes::KIND,
        span,
        format!(
            "`{name}` is not a kind of the slice; the top of a file holds `source` and `sequence`"
        ),
    );
    if let Some(c) = closest(name, TOP.iter().copied()) {
        d = d.with_fix(format!("write `{c}`"), sink.edit(span, c));
    }
    sink.push(d);
}

fn display_name(sink: &mut Sink<'_>, f: &crate::read::Fields<'_>) -> Option<String> {
    f.args
        .get("display name")
        .and_then(|v| values::text(sink, *v, "a display name"))
}

fn source(sink: &mut Sink<'_>, handles: &mut Handles, node: Node<'_>) -> Option<Source> {
    let f = read(sink, node, kind("source"), Some(&[]));
    if let Some(t) = f.handle {
        handles.define(sink, "source", t);
    }
    let name = display_name(sink, &f);
    let attr = |n: &str| f.attrs.get(n).map(|w| w.value);
    let media = attr("media").and_then(|v| values::text(sink, v, "a source's media path"));
    let hash = attr("hash").and_then(|v| values::hash(sink, v));
    let rate = attr("rate").and_then(|v| values::rate(sink, v));
    let len = match (attr("len"), rate) {
        (Some(v), Some(r)) => values::source_len(sink, v, r),
        _ => None,
    };
    let start = match (attr("start-timecode"), rate) {
        (Some(v), Some(r)) => values::timecode(sink, v, r),
        _ => None,
    };
    Some(Source {
        handle: f.handle_name()?.to_string(),
        name,
        file: sink.file,
        span: node.span(),
        media,
        hash,
        rate,
        len,
        start: start.unwrap_or(ZERO),
    })
}

const ZERO: Timecode = Timecode {
    hours: 0,
    minutes: 0,
    seconds: 0,
    frames: 0,
};

fn sequence(
    sink: &mut Sink<'_>,
    handles: &mut Handles,
    sources: &HashMap<String, Source>,
    node: Node<'_>,
) -> Option<Sequence> {
    let f = read(sink, node, kind("sequence"), Some(&[]));
    if let Some(t) = f.handle {
        handles.define(sink, "sequence", t);
    }
    let name = display_name(sink, &f);
    let rate = f
        .attrs
        .get("rate")
        .and_then(|w| values::rate(sink, w.value));
    let size = f
        .attrs
        .get("size")
        .and_then(|w| values::size(sink, w.value));
    let mut start = ZERO;
    let mut first_timecode: Option<Node<'_>> = None;
    for tc in f.children.iter().filter(|c| c.name() == "timecode") {
        if let Some(first) = first_timecode {
            let span = tc.name_token().map_or(tc.span(), |t| t.span);
            let d = sink
                .error(
                    codes::REPEATED,
                    span,
                    format!(
                        "{} has two `timecode` lines; it holds at most one",
                        describe("sequence", node)
                    ),
                )
                .with_label(
                    sink.file,
                    first.name_token().map_or(first.span(), |t| t.span),
                    "the first",
                )
                .with_fix("remove this one", sink.edit(sink.deletion(tc.span()), ""));
            sink.push(d);
            continue;
        }
        first_timecode = Some(*tc);
        let t = read(sink, *tc, kind("timecode"), Some(&[]));
        if let Some(v) = t.args.get("counting") {
            values::one_of(
                sink,
                *v,
                slice::TIMECODE_COUNTING,
                "a timecode counting of the slice",
            );
        }
        if let (Some(w), Some(r)) = (t.attrs.get("start"), rate) {
            start = values::timecode(sink, w.value, r).unwrap_or(ZERO);
        }
    }
    let clock = rate.and_then(|r| Clock::with_timecode(r, r, start).ok());
    let mut tracks = Vec::new();
    for t in f.children.iter().filter(|c| c.name() == "track") {
        if let Some(track) = track(sink, handles, sources, *t, clock.as_ref()) {
            tracks.push(track);
        }
    }
    Some(Sequence {
        handle: f.handle_name()?.to_string(),
        name,
        file: sink.file,
        span: node.span(),
        rate,
        size,
        start,
        tracks,
    })
}

fn track(
    sink: &mut Sink<'_>,
    handles: &mut Handles,
    sources: &HashMap<String, Source>,
    node: Node<'_>,
    clock: Option<&Clock>,
) -> Option<Track> {
    let f = read(sink, node, kind("track"), Some(&[]));
    if let Some(t) = f.handle {
        handles.define(sink, "track", t);
    }
    if let Some(v) = f.args.get("track kind") {
        values::one_of(sink, *v, slice::TRACK_KINDS, "a track kind of the slice");
    }
    let name = display_name(sink, &f);
    let mut placed = Vec::new();
    for c in f.children.iter().filter(|c| c.name() == "clip") {
        if let Some(p) = clip(sink, handles, sources, *c, clock) {
            placed.push(p);
        }
    }
    let handle = f.handle_name()?.to_string();
    overlaps(sink, &handle, &placed, clock);
    crossfades(sink, &handle, &placed, clock);
    Some(Track {
        handle,
        name,
        span: node.span(),
        clips: placed.into_iter().map(|p| p.clip).collect(),
    })
}

fn clip<'a>(
    sink: &mut Sink<'_>,
    handles: &mut Handles,
    sources: &HashMap<String, Source>,
    node: Node<'a>,
    clock: Option<&Clock>,
) -> Option<Placed<'a>> {
    let f = read(sink, node, kind("clip"), Some(&[]));
    if let Some(t) = f.handle {
        handles.define(sink, "clip", t);
    }
    let from = f.attrs.get("from").map(|w| w.value);
    let source = from.and_then(|v| resolve(sink, handles, v, "source", sources));
    let mut range = None;
    let mut source_frames = None;
    if let (Some(src), Some(clock), Some(v)) = (source, clock, f.args.get("source range"))
        && let Some(src_rate) = src.rate
        && let Ok(src_clock) = Clock::with_timecode(clock.rate(), src_rate, src.start)
    {
        let sc = SourceClock {
            clock: src_clock,
            handle: src.handle.clone(),
            len: src.len.map(|l| (l, src_rate)),
        };
        range = values::source_range(sink, *v, &sc);
        source_frames = src
            .len
            .and_then(|l| values::frames_in(l, src_rate, clock.rate()));
    }
    let at_value = f.attrs.get("at").map(|w| w.value);
    let at = match (at_value, clock) {
        (Some(v), Some(c)) => values::position(sink, v, c),
        _ => None,
    };
    let mut effects = Vec::new();
    let mut effect_handles: HashMap<String, Span> = HashMap::new();
    let mut crossfade: Option<(Node<'_>, Option<Value<'_>>, Option<i64>)> = None;
    let owner = describe("clip", node);
    for child in &f.children {
        match child.name().as_ref() {
            "effect" => {
                if let Some(e) = effect(sink, *child, clock, &mut effect_handles, &owner) {
                    effects.push(e);
                }
            }
            "crossfade" => {
                if let Some((first, _, _)) = crossfade {
                    let span = child.name_token().map_or(child.span(), |t| t.span);
                    let d = sink
                        .error(
                            codes::REPEATED,
                            span,
                            format!("{owner} has two crossfades; it holds at most one"),
                        )
                        .with_label(
                            sink.file,
                            first.name_token().map_or(first.span(), |t| t.span),
                            "the first",
                        )
                        .with_fix(
                            "remove this one",
                            sink.edit(sink.deletion(child.span()), ""),
                        );
                    sink.push(d);
                    continue;
                }
                let c = read(sink, *child, kind("crossfade"), Some(&[]));
                let v = c.args.get("length").copied();
                let len = match (v, clock) {
                    (Some(v), Some(clock)) => {
                        values::frames(sink, v, clock, "a crossfade's length", 1)
                    }
                    _ => None,
                };
                crossfade = Some((*child, v, len));
            }
            _ => {}
        }
    }
    let model = Clip {
        handle: f.handle_name()?.to_string(),
        span: node.span(),
        source: source.map(|s| s.handle.clone()),
        range,
        at,
        effects,
        crossfade: crossfade.and_then(|c| c.2),
    };
    Some(Placed {
        clip: model,
        node,
        at: at_value,
        crossfade: crossfade.and_then(|c| c.2.and(c.1)),
        source_frames,
    })
}

/// The source `v` names, or a `veel-handle` error: it names nothing, or an element of another
/// kind, with the nearest handle of the right kind as its fix.
fn resolve<'s>(
    sink: &mut Sink<'_>,
    handles: &Handles,
    v: Value<'_>,
    kind: &'static str,
    sources: &'s HashMap<String, Source>,
) -> Option<&'s Source> {
    let name = values::handle_word(sink, v, kind)?;
    if let Some(s) = sources.get(name) {
        return Some(s);
    }
    if handles.defined.contains_key(&(kind, name.to_string())) {
        return None; // defined, but its own lines could not be read
    }
    let mut names = handles.of(kind);
    names.extend(sources.keys().map(String::as_str));
    names.sort();
    names.dedup();
    let message = match handles.seen.get(name) {
        Some(other) => format!("`@{name}` is a {other}, not a {kind}"),
        None => format!("no {kind} is `@{name}`"),
    };
    let listed = if names.is_empty() {
        format!("; the project defines no {kind}")
    } else {
        format!(
            "; the project's {kind}s are {}",
            names
                .iter()
                .map(|n| format!("`@{n}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut d = sink.error(codes::HANDLE, v.span(), format!("{message}{listed}"));
    if let Some(c) = closest(name, names.iter().copied()) {
        d = d.with_fix(
            format!("write `@{c}`"),
            sink.edit(v.span(), format!("@{c}")),
        );
    }
    sink.push(d);
    None
}

fn effect(
    sink: &mut Sink<'_>,
    node: Node<'_>,
    clock: Option<&Clock>,
    seen: &mut HashMap<String, Span>,
    owner: &str,
) -> Option<Effect> {
    // The asset is read first: it says which parameters the line and block may set.
    let asset_word = node.args().nth(1).and_then(|v| v.as_word());
    let schema = asset_word.and_then(slice::effect);
    let names: Vec<&'static str> = schema
        .map(|e| e.loaded_params().map(|p| p.name).collect())
        .unwrap_or_default();
    let f = read(sink, node, kind("effect"), schema.map(|_| names.as_slice()));
    if let Some(v) = f.args.get("asset") {
        values::one_of(sink, *v, &["qtblend"], "an effect of the slice");
    }
    if let (Some(t), Some(h)) = (f.handle, f.handle_name()) {
        if let Some(first) = seen.get(h) {
            let free = (2..)
                .map(|n| format!("{h}-{n}"))
                .find(|x| !seen.contains_key(x))
                .expect("a free handle");
            let d = sink
                .error(
                    codes::HANDLE,
                    t.span,
                    format!(
                        "{owner} has two effects `@{h}`; an effect's handle is unique in its clip"
                    ),
                )
                .with_label(sink.file, *first, "first defined here")
                .with_fix(
                    format!("rename this one `@{free}`"),
                    sink.edit(t.span, format!("@{free}")),
                );
            sink.push(d);
        } else {
            seen.insert(h.to_string(), t.span);
        }
    }
    let schema = schema?;
    let mut params = Vec::new();
    let mut written: Vec<_> = f.attrs.iter().collect();
    written.sort_by_key(|(_, w)| w.key.start);
    for (name, w) in written {
        let Some(param) = schema.param(name) else {
            continue;
        };
        let block = w.line.and_then(|l| l.block());
        if let Some(block) = block {
            let line = w.line.expect("a block is on a line");
            if !param.keyframes {
                let d = sink.error(
                    codes::TYPE,
                    block.span(),
                    format!(
                        "`{name}` takes no keyframes: write one value, as `{name} {}`",
                        param
                            .choices
                            .first()
                            .map_or(crate::read::sample(param.value), |c| c.word)
                    ),
                );
                sink.push(d);
                continue;
            }
            if !w.alone() {
                let d = sink
                    .error(
                        codes::REPEATED,
                        w.value.span(),
                        format!("`{name}` has both a value and a block of keys; it takes one or the other"),
                    )
                    .with_fix("keep the keys", sink.edit(sink.deletion(w.value.span()), ""));
                sink.push(d);
                continue;
            }
            for entry in line.entries() {
                if let veel_syntax::ast::Entry::Prop { key, value }
                | veel_syntax::ast::Entry::Keyword { key, value } = entry
                {
                    if key.text == "time" {
                        values::one_of(sink, value, slice::KEY_TIMES, "a key time of the slice");
                    } else {
                        crate::read::unknown_on_block(sink, name, key);
                    }
                }
            }
            if let Some(keys) = keys(sink, line, param, clock) {
                params.push(Param {
                    name: name.clone(),
                    value: ParamValue::Keys(keys),
                });
            }
            continue;
        }
        if let Some(line) = w.line {
            for entry in line.entries() {
                if let veel_syntax::ast::Entry::Prop { key, .. }
                | veel_syntax::ast::Entry::Keyword { key, .. } = entry
                {
                    crate::read::unknown_on_block(sink, name, key);
                }
            }
        }
        if w.alone() && param.value != Form::OnOff {
            let at = w
                .line
                .and_then(|l| l.name_token())
                .map_or(w.key, |t| t.span);
            let d = sink.error(
                codes::MISSING,
                at,
                format!(
                    "`{name}` has no value: write `{name} {}`{}",
                    crate::read::sample(param.value),
                    if param.keyframes {
                        " or a block of keys"
                    } else {
                        ""
                    }
                ),
            );
            sink.push(d);
            continue;
        }
        if let Some(text) = param_value(sink, w.value, param) {
            params.push(Param {
                name: name.clone(),
                value: ParamValue::Value(text),
            });
        }
    }
    Some(Effect {
        handle: f.handle_name()?.to_string(),
        asset: schema.id.to_string(),
        params,
    })
}

/// A parameter's value, read as its form; its text as written (`on` for a switch alone).
fn param_value(sink: &mut Sink<'_>, v: Value<'_>, param: &veel_schema::Param) -> Option<String> {
    match param.value {
        Form::Rect => values::rect(sink, v).map(|_| v.text().to_string()),
        Form::Point => values::point(sink, v).map(|_| v.text().to_string()),
        Form::Angle => values::angle(sink, v).map(|_| v.text().to_string()),
        Form::Choice => values::choice(sink, v, param).map(str::to_string),
        Form::OnOff => values::switch(sink, v).map(str::to_string),
        other => unreachable!("the slice's effect has no parameter of form {other:?}"),
    }
}

/// The keys of a parameter's block, in increasing time from 0.
fn keys(
    sink: &mut Sink<'_>,
    line: Node<'_>,
    param: &veel_schema::Param,
    clock: Option<&Clock>,
) -> Option<Vec<Key>> {
    let mut out = Vec::new();
    let mut last: Option<(i64, Span)> = None;
    let mut ok = true;
    let mut any = false;
    for k in line.children() {
        let name = k.name();
        let span = k.name_token().map_or(k.span(), |t| t.span);
        if name != "key" {
            if !is_kebab(&name) {
                continue;
            }
            let mut d = sink.error(
                codes::KIND,
                span,
                format!(
                    "`{}`'s block holds `key` lines; `{name}` cannot stand in it",
                    param.name
                ),
            );
            if closest(&name, ["key"]).is_some() {
                d = d.with_fix("write `key`", sink.edit(span, "key"));
            }
            sink.push(d);
            ok = false;
            continue;
        }
        any = true;
        let f = read(sink, k, kind("key"), Some(&[]));
        let time = match (f.args.get("time"), clock) {
            (Some(v), Some(c)) => values::frames(sink, *v, c, "a key's time", 0),
            _ => None,
        };
        let value = f
            .args
            .get("value")
            .and_then(|v| param_value(sink, *v, param));
        let ease = match f.attrs.get("ease") {
            Some(w) => values::one_of(sink, w.value, slice::EASE_WORDS, "a keyframe type"),
            None => Some("linear"),
        };
        if let (Some(t), Some((prev, prev_span))) = (time, last)
            && t <= prev
        {
            let d = sink
                .error(
                    codes::KEY_ORDER,
                    f.args["time"].span(),
                    format!(
                        "this key, at frame {t} of the clip, is not after the key before it, at frame {prev}: keys are written in increasing time"
                    ),
                )
                .with_label(sink.file, prev_span, "the key before");
            sink.push(d);
            ok = false;
        }
        if let Some(t) = time {
            last = Some((t, f.args["time"].span()));
        }
        match (time, value, ease) {
            (Some(frame), Some(value), Some(ease)) => out.push(Key {
                frame,
                value,
                ease: ease.to_string(),
            }),
            _ => ok = false,
        }
    }
    if !any && ok {
        let span = line.name_token().map_or(line.span(), |t| t.span);
        let d = sink.error(
            codes::MISSING,
            span,
            format!(
                "`{}`'s block holds no key: write `key 0 {}` in it, or one value",
                param.name,
                crate::read::sample(param.value)
            ),
        );
        sink.push(d);
        return None;
    }
    ok.then_some(out)
}

/// The unit a time word is written in; frames for anything else.
fn unit(v: Option<Value<'_>>) -> Unit {
    v.and_then(|v| v.as_word())
        .and_then(|w| Time::parse(w).ok())
        .map_or(Unit::Frames, |t| t.unit())
}

/// Two clips of one track that share a frame: the later written is reported, with the frame
/// where it could start, after the earlier, as its fix.
fn overlaps(sink: &mut Sink<'_>, track: &str, placed: &[Placed<'_>], clock: Option<&Clock>) {
    for (j, b) in placed.iter().enumerate() {
        if b.clip.at.is_none() || b.clip.range.is_none() {
            continue;
        }
        for a in &placed[..j] {
            if a.clip.at.is_none() || a.clip.range.is_none() {
                continue;
            }
            let (pa, pb) = (a.clip.place(), b.clip.place());
            let n = pa.overlap(pb);
            if n == 0 {
                continue;
            }
            let at = b.at.expect("a placed clip has its `at`");
            let mut d = sink
                .error(
                    codes::OVERLAP,
                    b.node.name_token().map_or(b.node.span(), |t| t.span),
                    format!(
                        "clip @{} overlaps @{} by {n} frames on track @{track}",
                        b.clip.handle, a.clip.handle
                    ),
                )
                .with_label(
                    sink.file,
                    a.node.name_token().map_or(a.node.span(), |t| t.span),
                    format!(
                        "@{} covers frames {} up to {}",
                        a.clip.handle,
                        pa.start(),
                        pa.end()
                    ),
                );
            if let Some(clock) = clock {
                let word = clock
                    .write(pa.end(), unit(Some(at)), false)
                    .unwrap_or_else(|| pa.end().to_string());
                d = d.with_fix(
                    format!(
                        "start @{} at {word}, where @{} ends",
                        b.clip.handle, a.clip.handle
                    ),
                    sink.edit(at.span(), word),
                );
            }
            sink.push(d);
            break;
        }
    }
}

/// Each crossfade's next clip, the source frames its mix reads, and the mixes at each clip's
/// two ends (spec, section 17, "Crossfade"; board row B-44).
fn crossfades(sink: &mut Sink<'_>, track: &str, placed: &[Placed<'_>], clock: Option<&Clock>) {
    // For each clip, the clip its crossfade dissolves into.
    let mut next: Vec<Option<usize>> = vec![None; placed.len()];
    for (i, a) in placed.iter().enumerate() {
        let (Some(n), Some(v)) = (a.clip.crossfade, a.crossfade) else {
            continue;
        };
        if a.clip.at.is_none() || a.clip.range.is_none() {
            continue;
        }
        let cut = a.clip.place().end();
        let found = placed
            .iter()
            .position(|b| b.clip.range.is_some() && b.clip.at == Some(cut));
        let Some(j) = found else {
            let mut d = sink.error(
                codes::CROSSFADE_NEXT,
                v.span(),
                format!(
                    "@{}'s crossfade has no clip to dissolve into: no clip on track @{track} starts at frame {cut}, where @{} ends",
                    a.clip.handle, a.clip.handle
                ),
            );
            let later = placed
                .iter()
                .filter(|b| b.clip.at.is_some_and(|at| at > cut) && b.clip.range.is_some())
                .min_by_key(|b| b.clip.at);
            d = match later {
                Some(b) => d.with_note(format!(
                    "the next clip on the track, @{}, starts at frame {}; a crossfade joins clips that meet",
                    b.clip.handle,
                    b.clip.at.expect("placed")
                )),
                None => d.with_note("no clip follows it on the track; remove the crossfade, or place the clip it dissolves into at its end"),
            };
            sink.push(d);
            continue;
        };
        next[i] = Some(j);
        let b = &placed[j];
        let (after, before) = (n / 2, n - n / 2);
        let ra = a.clip.range.expect("placed");
        let rb = b.clip.range.expect("placed");
        if let Some(len) = a.source_frames
            && ra.end() + after > len
        {
            let d = sink.error(
                codes::RANGE,
                v.span(),
                format!(
                    "the crossfade runs @{} {after} frames past the cut, to frame {} of its source, which is {len} frames long here",
                    a.clip.handle,
                    ra.end() + after
                ),
            );
            sink.push(d);
        }
        if rb.start() < before {
            let d = sink.error(
                codes::RANGE,
                v.span(),
                format!(
                    "the crossfade starts @{} {before} frames before the cut, but its range starts {} frames into its source",
                    b.clip.handle,
                    rb.start()
                ),
            );
            sink.push(d);
        }
    }
    // The mixes at each clip's two ends fit in its length.
    for (x, clip) in placed.iter().enumerate() {
        let incoming = (0..placed.len()).find(|&i| next[i] == Some(x));
        let into = incoming.map_or(0, |i| placed[i].clip.crossfade.expect("a crossfade") / 2);
        let out = next[x].map_or(0, |_| {
            let n = clip.clip.crossfade.expect("a crossfade");
            n - n / 2
        });
        let len = clip.clip.range.map_or(0, |r| r.len());
        if into + out <= len {
            continue;
        }
        let h = &clip.clip.handle;
        let (at, message, fixed) = match (incoming, next[x]) {
            (Some(i), Some(_)) => (
                clip.crossfade,
                format!(
                    "the mixes at @{h}'s two ends overlap: @{}'s crossfade runs {into} frames into @{h}, and @{h}'s own starts {out} frames before its end, {} frames in all; @{h} is {len} frames long",
                    placed[i].clip.handle,
                    into + out
                ),
                (len > into).then(|| 2 * (len - into)),
            ),
            (None, Some(_)) => (
                clip.crossfade,
                format!(
                    "@{h}'s crossfade starts {out} frames before its end, but @{h} is {len} frames long"
                ),
                Some(2 * len),
            ),
            (Some(i), None) => (
                placed[i].crossfade,
                format!(
                    "@{}'s crossfade runs {into} frames into @{h}, which is {len} frames long",
                    placed[i].clip.handle
                ),
                Some(2 * len + 1),
            ),
            (None, None) => continue,
        };
        let Some(at) = at else { continue };
        let mut d = sink.error(codes::CROSSFADE_OVERLAP, at.span(), message);
        if let (Some(n), Some(clock)) = (fixed.filter(|n| *n >= 1), clock) {
            let word = clock
                .write(n, unit(Some(at)), true)
                .unwrap_or_else(|| n.to_string());
            d = d.with_fix(
                format!("shorten the crossfade to `{word}`"),
                sink.edit(at.span(), word),
            );
        }
        sink.push(d);
    }
}

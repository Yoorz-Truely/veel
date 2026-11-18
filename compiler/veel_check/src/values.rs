//! Reading one value of each form the slice takes (spec, section 17, "Attributes and their
//! values" and "Values of the effect's parameters"), with the diagnostics of each mistake.

use veel_schema::{Param, is_kebab};
use veel_syntax::ast::Value;
use veel_syntax::{Span, closest};
use veel_time::{Clock, Range, Rate, Time, TimeError, Timecode, Unit};

use crate::codes;
use crate::read::Sink;

/// Reports `err`, from the word at `span`, with the nearest exact choices as fixes; `rewrite`
/// turns a choice into the text that replaces the span (the whole range word around it).
pub(crate) fn time_error(
    sink: &mut Sink<'_>,
    err: &TimeError,
    span: Span,
    rewrite: &dyn Fn(&str) -> String,
) {
    let mut d = sink.error(codes::of_time(err.code()), span, err.to_string());
    match err {
        TimeError::Inexact(e) => {
            for choice in &e.choices {
                d = d.with_fix(
                    format!("write `{choice}`"),
                    sink.edit(span, rewrite(choice)),
                );
            }
        }
        TimeError::NotExactRate { exact, .. } => {
            d = d.with_fix(
                format!("write `{exact}`"),
                sink.edit(span, exact.to_string()),
            );
        }
        _ => {}
    }
    sink.push(d);
}

/// Reports a value of the wrong type.
fn wrong_type(sink: &mut Sink<'_>, v: Value<'_>, what: &str) {
    wrong_type_or(sink, v, what, false);
}

/// Reports a value of the wrong type; where a word was expected and a string holds one, its fix
/// takes the quotes away.
fn wrong_type_or(sink: &mut Sink<'_>, v: Value<'_>, what: &str, word_expected: bool) {
    let found = match v {
        Value::Str(_) | Value::Raw(_) => "a string",
        Value::List(_) => "a list",
        Value::Word(_) => "a word",
        Value::Missing(_) => "nothing",
    };
    let mut d = sink.error(
        codes::TYPE,
        v.span(),
        format!("{what}; found {found}{}", shown(v)),
    );
    if word_expected
        && let Some(inner) = v.as_string()
        && !inner.is_empty()
        && veel_syntax::lex(&inner).0.len() == 1
        && veel_syntax::lex(&inner).0[0].kind == veel_syntax::TokenKind::Word
    {
        d = d.with_fix(
            format!("write `{inner}`"),
            sink.edit(v.span(), inner.into_owned()),
        );
    }
    sink.push(d);
}

/// `n value` or `n values`.
fn values_count(n: usize) -> String {
    if n == 1 {
        "1 value".to_string()
    } else {
        format!("{n} values")
    }
}

fn shown(v: Value<'_>) -> String {
    match v {
        Value::Missing(_) | Value::List(_) => String::new(),
        _ => format!(", `{}`", v.text()),
    }
}

/// The word of `v`, or a `veel-type` saying what was expected.
pub(crate) fn word<'a>(sink: &mut Sink<'_>, v: Value<'a>, what: &str) -> Option<&'a str> {
    match v {
        Value::Word(t) => Some(t.text),
        _ => {
            wrong_type_or(sink, v, what, true);
            None
        }
    }
}

/// A quoted string: a display name or a path.
pub(crate) fn text(sink: &mut Sink<'_>, v: Value<'_>, what: &str) -> Option<String> {
    if let Some(s) = v.as_string() {
        return Some(s.into_owned());
    }
    let mut d = sink.error(
        codes::TYPE,
        v.span(),
        format!("{what} is a quoted string{}", shown(v)),
    );
    if let Value::Word(t) = v {
        d = d.with_fix(
            format!("write `\"{}\"`", t.text),
            sink.edit(t.span, format!("\"{}\"", t.text)),
        );
    }
    sink.push(d);
    None
}

/// A content hash: `sha256:` and 64 lowercase hexadecimal digits (V10).
pub(crate) fn hash(sink: &mut Sink<'_>, v: Value<'_>) -> Option<String> {
    let w = word(
        sink,
        v,
        "a hash is `sha256:` and 64 lowercase hexadecimal digits",
    )?;
    let digits = w.strip_prefix("sha256:");
    if digits.is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        return Some(w.to_string());
    }
    let mut d = sink.error(
        codes::TYPE,
        v.span(),
        format!("`{w}` is not a hash: write `sha256:` and 64 lowercase hexadecimal digits"),
    );
    if let Some(h) = digits.map(str::to_ascii_lowercase)
        && h.len() == 64
        && h.bytes().all(|b| b.is_ascii_hexdigit())
    {
        d = d.with_fix(
            format!("write `sha256:{h}`"),
            sink.edit(v.span(), format!("sha256:{h}")),
        );
    }
    sink.push(d);
    None
}

/// An exact rate.
pub(crate) fn rate(sink: &mut Sink<'_>, v: Value<'_>) -> Option<Rate> {
    let w = word(
        sink,
        v,
        "a rate is a whole number (`25`) or a fraction (`30000/1001`)",
    )?;
    Rate::parse(w)
        .map_err(|e| time_error(sink, &e, v.span(), &|c| c.to_string()))
        .ok()
}

/// A frame size, `[width height]`, whole pixels, each at least 1.
pub(crate) fn size(sink: &mut Sink<'_>, v: Value<'_>) -> Option<(u32, u32)> {
    let what = "a size is `[width height]` in whole pixels, as `[1920 1080]`";
    if !matches!(v, Value::List(_)) {
        wrong_type(sink, v, what);
        return None;
    }
    let items = v.items();
    if items.len() != 2 {
        let d = sink.error(
            codes::TYPE,
            v.span(),
            format!("{what}; found {}", values_count(items.len())),
        );
        sink.push(d);
        return None;
    }
    let mut out = [0u32; 2];
    for (i, item) in items.iter().enumerate() {
        let w = word(sink, *item, what)?;
        match w.parse::<u32>() {
            Ok(0) => {
                let d = sink.error(
                    codes::RANGE,
                    item.span(),
                    "a frame is at least 1 pixel wide and high",
                );
                sink.push(d);
                return None;
            }
            Ok(n) if w.bytes().all(|b| b.is_ascii_digit()) => out[i] = n,
            _ => {
                let d = sink.error(
                    codes::TYPE,
                    item.span(),
                    format!("`{w}` is not a whole number of pixels; {what}"),
                );
                sink.push(d);
                return None;
            }
        }
    }
    Some((out[0], out[1]))
}

/// A timecode, `hh:mm:ss:ff`, whose fields are in range at `rate`.
pub(crate) fn timecode(sink: &mut Sink<'_>, v: Value<'_>, rate: Rate) -> Option<Timecode> {
    let w = word(sink, v, "a timecode is `hh:mm:ss:ff`, as `01:00:00:00`")?;
    let tc = Timecode::parse(w)
        .and_then(|tc| tc.count(rate).map(|_| tc))
        .map_err(|e| time_error(sink, &e, v.span(), &|c| c.to_string()))
        .ok()?;
    Some(tc)
}

/// One of `words`, with the nearest as its fix.
pub(crate) fn one_of(
    sink: &mut Sink<'_>,
    v: Value<'_>,
    words: &[&'static str],
    what: &str,
) -> Option<&'static str> {
    let all = words
        .iter()
        .map(|w| format!("`{w}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let w = word(sink, v, &format!("{what} is one of {all}"))?;
    if let Some(found) = words.iter().find(|x| **x == w) {
        return Some(found);
    }
    let mut d = sink.error(
        codes::ENUM,
        v.span(),
        format!("`{w}` is not {what}: write {}", one_of_list(words)),
    );
    if let Some(c) = closest(w, words.iter().copied()).or(match words {
        [only] => Some(*only),
        _ => None,
    }) {
        d = d.with_fix(format!("write `{c}`"), sink.edit(v.span(), c));
    }
    sink.push(d);
    None
}

fn one_of_list(words: &[&str]) -> String {
    match words {
        [one] => format!("`{one}`"),
        _ if words.len() > 12 => format!(
            "one of {} and {} more",
            words[..12]
                .iter()
                .map(|w| format!("`{w}`"))
                .collect::<Vec<_>>()
                .join(", "),
            words.len() - 12
        ),
        _ => format!(
            "one of {}",
            words
                .iter()
                .map(|w| format!("`{w}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// A handle written as a value: `@name`, kebab-case. Its name, without `@`.
pub(crate) fn handle_word<'a>(sink: &mut Sink<'_>, v: Value<'a>, kind: &str) -> Option<&'a str> {
    let w = word(
        sink,
        v,
        &format!("this is the handle of a {kind}, as `@name`"),
    )?;
    match w.strip_prefix('@') {
        Some(h) if is_kebab(h) => Some(h),
        _ => {
            let mut d = sink.error(
                codes::HANDLE,
                v.span(),
                format!("`{w}` is not a handle: write the handle of a {kind}, `@` and its name"),
            );
            if let Some(k) = crate::read::kebab(w.trim_start_matches('@')) {
                d = d.with_fix(
                    format!("write `@{k}`"),
                    sink.edit(v.span(), format!("@{k}")),
                );
            }
            sink.push(d);
            None
        }
    }
}

/// A sequence position: frames, `s` or `ms` from frame 0, or the sequence's timecode.
pub(crate) fn position(sink: &mut Sink<'_>, v: Value<'_>, clock: &Clock) -> Option<i64> {
    let w = word(sink, v, "a position is frames, `s`, `ms` or a timecode")?;
    match clock.frames(w) {
        Ok(f) if f < 0 => {
            let d = sink.error(
                codes::RANGE,
                v.span(),
                format!("`{w}` is before frame 0 of the sequence"),
            );
            sink.push(d);
            None
        }
        Ok(f) => Some(f),
        Err(e) => {
            time_error(sink, &e, v.span(), &|c| c.to_string());
            None
        }
    }
}

/// A length or a clip-local time: frames, `s` or `ms`, never a timecode, never negative. `what`
/// names it in messages; `least` is the smallest it may be.
pub(crate) fn frames(
    sink: &mut Sink<'_>,
    v: Value<'_>,
    clock: &Clock,
    what: &str,
    least: i64,
) -> Option<i64> {
    let w = word(sink, v, &format!("{what} is frames, `s` or `ms`"))?;
    if matches!(Time::parse(w), Ok(Time::Timecode(_))) {
        let mut d = sink.error(
            codes::UNIT,
            v.span(),
            format!("{what} is frames, `s` or `ms`; a timecode, `{w}`, names an absolute frame"),
        );
        if let Ok(n) = clock.duration(w) {
            d = d.with_fix(
                format!("write `{n}`, its frames"),
                sink.edit(v.span(), n.to_string()),
            );
        }
        sink.push(d);
        return None;
    }
    match clock.duration(w) {
        Ok(f) if f < least => {
            let at_least = if least == 0 {
                "not negative".to_string()
            } else {
                format!("at least {least} frame")
            };
            let d = sink.error(
                codes::RANGE,
                v.span(),
                format!("{what} is {at_least}; `{w}` is {f} frames"),
            );
            sink.push(d);
            None
        }
        Ok(f) => Some(f),
        Err(e) => {
            time_error(sink, &e, v.span(), &|c| c.to_string());
            None
        }
    }
}

/// A source's length at its own rate: `s`, `ms` or a timecode length, never bare frames.
pub(crate) fn source_len(sink: &mut Sink<'_>, v: Value<'_>, rate: Rate) -> Option<i64> {
    let w = word(
        sink,
        v,
        "a source's length is `s`, `ms` or a timecode length",
    )?;
    let clock = Clock::new(rate);
    if let Ok(Time::Frames(n)) = Time::parse(w) {
        let mut d = sink.error(
            codes::UNIT,
            v.span(),
            format!(
                "a source's length needs a unit: `{w}` would be frames, and no sequence encloses a \
                 source to give them a rate"
            ),
        );
        if let Some(tc) = clock.write(n, Unit::Timecode, true) {
            d = d.with_fix(
                format!("write `{tc}`, {n} frames at {rate}"),
                sink.edit(v.span(), tc),
            );
        }
        sink.push(d);
        return None;
    }
    match clock.duration(w) {
        Ok(f) if f < 0 => {
            let d = sink.error(
                codes::RANGE,
                v.span(),
                format!("a length is not negative; `{w}` is {f} frames"),
            );
            sink.push(d);
            None
        }
        Ok(f) => Some(f),
        Err(e) => {
            time_error(sink, &e, v.span(), &|c| c.to_string());
            None
        }
    }
}

/// What a clip's source range is read against: the sequence's rate and the source's.
pub(crate) struct SourceClock {
    /// Frames at the sequence's rate, timecode at the source's from its start.
    pub clock: Clock,
    /// The source's handle, for messages.
    pub handle: String,
    /// The source's length, in its own frames, and its rate.
    pub len: Option<(i64, Rate)>,
}

/// A clip's source range, `in..out` or `in..+length`, half-open, in the sequence's frames: in
/// the source, at least one frame long. Its end past the source's is a `veel-range` error
/// whose fix ends it at the source's last frame.
pub(crate) fn source_range(sink: &mut Sink<'_>, v: Value<'_>, src: &SourceClock) -> Option<Range> {
    let w = word(sink, v, "a source range is `in..out` or `in..+length`")?;
    let span = v.span();
    let Some((a, b)) = w.split_once("..") else {
        let d = sink.error(
            codes::TYPE,
            span,
            format!("`{w}` is not a range: write `in..out` or `in..+length`"),
        );
        sink.push(d);
        return None;
    };
    let clock = &src.clock;
    let start = match clock.frames(a) {
        Ok(f) => f,
        Err(e) => {
            time_error(sink, &e, span, &|c| format!("{c}..{b}"));
            return None;
        }
    };
    let (len_word, end) = match b.strip_prefix('+') {
        Some(len) => match clock.duration(len) {
            Ok(n) => (Some(len), start.checked_add(n)?),
            Err(e) => {
                time_error(sink, &e, span, &|c| format!("{a}..+{c}"));
                return None;
            }
        },
        None => match clock.frames(b) {
            Ok(f) => (None, f),
            Err(e) => {
                time_error(sink, &e, span, &|c| format!("{a}..{c}"));
                return None;
            }
        },
    };
    if start < 0 {
        let d = sink.error(
            codes::RANGE,
            span,
            format!(
                "`{w}` starts {} frames before the first frame of @{}",
                -start, src.handle
            ),
        );
        sink.push(d);
        return None;
    }
    if end <= start {
        let d = sink.error(
            codes::RANGE,
            span,
            if end == start {
                format!("`{w}` holds no frame: a clip is at least one frame long")
            } else {
                format!("`{w}` ends before it starts")
            },
        );
        sink.push(d);
        return None;
    }
    let range = Range::new(start, end).expect("ordered");
    if let Some((len, rate)) = src.len
        && let Some(last) = frames_in(len, rate, clock.rate())
        && end > last
    {
        let over = end - last;
        let mut d = sink.error(
            codes::RANGE,
            span,
            format!(
                "`{w}` runs {over} frames past the end of @{}, which is {last} frames long here",
                src.handle
            ),
        );
        if last > start {
            let unit = |word: &str| Time::parse(word).map(|t| t.unit()).unwrap_or(Unit::Frames);
            let fixed = match len_word {
                Some(l) => clock
                    .write(last - start, unit(l), true)
                    .map(|l| format!("{a}..+{l}")),
                None => clock
                    .write(last, unit(b), false)
                    .map(|o| format!("{a}..{o}")),
            }
            .unwrap_or_else(|| format!("{a}..{last}"));
            d = d.with_fix(
                format!("end it at the source's last frame: `{fixed}`"),
                sink.edit(span, fixed),
            );
        }
        sink.push(d);
        return None;
    }
    Some(range)
}

/// The whole sequence frames at `seq` in `len` frames at `rate`, rounded down: an end past it
/// is past the source's end, whether or not the source's end lands on a sequence frame.
pub(crate) fn frames_in(len: i64, rate: Rate, seq: Rate) -> Option<i64> {
    let secs = rate.seconds(len);
    let f = seq.frames_exact(secs)?;
    i64::try_from(f.floor()).ok()
}

/// A number word and its unit: `-96`, `0.5`, `85%`, `2.5deg`.
pub(crate) fn number(word: &str) -> Option<(f64, &str)> {
    let b = word.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        i += 1;
    }
    let digits_from = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let int = i - digits_from;
    let mut frac = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let f = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        frac = i - f;
    }
    if int == 0 && frac == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'-' || b[j] == b'+') {
            j += 1;
        }
        let e = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > e {
            i = j;
        }
    }
    let value: f64 = word[..i].parse().ok()?;
    value.is_finite().then_some((value, &word[i..]))
}

/// A number item of a list or a value, with the units `units` allows (`""` for bare). Its value,
/// a percent divided by 100.
fn quantity(sink: &mut Sink<'_>, v: Value<'_>, what: &str, units: &[&str]) -> Option<f64> {
    let w = word(sink, v, &format!("{what} is a number"))?;
    let Some((n, unit)) = number(w) else {
        let d = sink.error(
            codes::TYPE,
            v.span(),
            format!("`{w}` is not a number; {what} is a number"),
        );
        sink.push(d);
        return None;
    };
    if !units.contains(&unit) {
        let takes = units
            .iter()
            .map(|u| {
                if u.is_empty() {
                    "bare".to_string()
                } else {
                    format!("`{u}`")
                }
            })
            .collect::<Vec<_>>()
            .join(" or ");
        let mut d = sink.error(
            codes::UNIT,
            v.span(),
            format!("`{unit}` is not a unit of {what}, which is written {takes}"),
        );
        if units.contains(&"") {
            let bare = &w[..w.len() - unit.len()];
            d = d.with_fix(format!("write `{bare}`"), sink.edit(v.span(), bare));
        }
        sink.push(d);
        return None;
    }
    Some(if unit == "%" { n / 100.0 } else { n })
}

/// A rectangle, `[x y w h]` or `[x y w h opacity]`: pixels of the sequence's frame, bare
/// numbers; `w` and `h` at least 0 (board row B-56); the opacity from 0 to 1, or a percent.
pub(crate) fn rect(sink: &mut Sink<'_>, v: Value<'_>) -> Option<()> {
    let what = "a rectangle is `[x y w h]` or `[x y w h opacity]`";
    if !matches!(v, Value::List(_)) {
        wrong_type(sink, v, what);
        return None;
    }
    let items = v.items();
    if !(4..=5).contains(&items.len()) {
        let d = sink.error(
            codes::TYPE,
            v.span(),
            format!("{what}; found {}", values_count(items.len())),
        );
        sink.push(d);
        return None;
    }
    let names = ["its x", "its y", "its width", "its height"];
    let mut ok = true;
    for (i, item) in items.iter().take(4).enumerate() {
        match quantity(
            sink,
            *item,
            &format!("a rectangle's {}", &names[i][4..]),
            &[""],
        ) {
            Some(n) if i >= 2 && n < 0.0 => {
                let d = sink.error(
                    codes::RANGE,
                    item.span(),
                    format!(
                        "{} is at least 0 pixels; `{}` is negative",
                        &names[i][4..],
                        item.text()
                    ),
                );
                sink.push(d);
                ok = false;
            }
            Some(_) => {}
            None => ok = false,
        }
    }
    if let Some(o) = items.get(4) {
        match quantity(sink, *o, "an opacity", &["", "%"]) {
            Some(n) if !(0.0..=1.0).contains(&n) => {
                let d = sink.error(
                    codes::RANGE,
                    o.span(),
                    format!(
                        "an opacity is from 0 to 1, or 0% to 100%; `{}` is outside it",
                        o.text()
                    ),
                );
                sink.push(d);
                ok = false;
            }
            Some(_) => {}
            None => ok = false,
        }
    }
    ok.then_some(())
}

/// A point, `[x y]`, each a number as stored or a percent of it.
pub(crate) fn point(sink: &mut Sink<'_>, v: Value<'_>) -> Option<()> {
    let what = "a point is `[x y]`";
    if !matches!(v, Value::List(_)) {
        wrong_type(sink, v, what);
        return None;
    }
    let items = v.items();
    if items.len() != 2 {
        let d = sink.error(
            codes::TYPE,
            v.span(),
            format!("{what}; found {}", values_count(items.len())),
        );
        sink.push(d);
        return None;
    }
    let mut ok = true;
    for item in items {
        ok &= quantity(sink, item, "a point's coordinate", &["", "%"]).is_some();
    }
    ok.then_some(())
}

/// An angle: degrees, bare or with `deg`.
pub(crate) fn angle(sink: &mut Sink<'_>, v: Value<'_>) -> Option<()> {
    quantity(sink, v, "an angle", &["", "deg"]).map(|_| ())
}

/// A switch: `on` or `off`; written alone, `on`.
pub(crate) fn switch(sink: &mut Sink<'_>, v: Value<'_>) -> Option<&'static str> {
    if matches!(v, Value::Missing(_)) {
        return Some("on");
    }
    one_of(sink, v, &["on", "off"], "a switch's value")
}

/// One of `param`'s choice words. A stored number is not a word; its fix is the word for it.
pub(crate) fn choice(sink: &mut Sink<'_>, v: Value<'_>, param: &Param) -> Option<&'static str> {
    let words: Vec<&'static str> = param.choices.iter().map(|c| c.word).collect();
    if let Value::Word(t) = v
        && let Some(c) = param.choices.iter().find(|c| c.stored == t.text)
    {
        let d = sink
            .error(
                codes::ENUM,
                v.span(),
                format!(
                    "`{}` is the number Kdenlive stores; Veel writes `{}`'s words: `{}` here",
                    t.text, param.name, c.word
                ),
            )
            .with_fix(format!("write `{}`", c.word), sink.edit(v.span(), c.word));
        sink.push(d);
        return None;
    }
    one_of(sink, v, &words, &format!("a `{}` word", param.name))
}

//! Veel's exact time: integer frames at rational rates, with seconds, milliseconds and timecode
//! converted to frames by one exact rule, and half-open ranges (spec section 6; V20, V21).
//!
//! This is the minimal time of phase 1a (task 1a-02). A frame is the atomic unit: a bare integer
//! is frames at the sequence's rate, and every other form is converted to whole frames exactly.
//! **A value that does not land on a whole frame is an error naming the nearest exact choices,
//! never rounded** (V20, corrected November 15, 2026, as Null does). Scowr's time crate rounds
//! halves up; nothing here rounds at all.
//!
//! What phase 1a reads: frames (`120`, `-12`), seconds (`2s`, `1.5s`), milliseconds (`250ms`),
//! non-drop timecode (`01:00:10:12`), and the ranges `in..out` and `in..+duration`, half-open.
//! Drop-frame counting (V42), samples (`smp`), `min`, `h`, `m:ss.fff` and variable-frame-rate maps
//! come with task 1c-01.
//!
//! # The model
//!
//! - [`Rate`] is an exact positive rational: `25`, `30000/1001`.
//! - [`Time`] is what a word says, before a rate is known: frames, seconds, milliseconds or a
//!   [`Timecode`].
//! - [`Clock`] says what a frame is (the sequence's rate) and how timecode counts (its rate and
//!   the timecode of frame 0), and turns a word into whole frames, or a frame back into a word.
//! - [`Range`] is a half-open `[start, end)` range of frames.
//!
//! ```
//! use veel_time::{Clock, Rate, Timecode};
//!
//! let rate = Rate::parse("25").unwrap();
//! let main = Clock::with_timecode(rate, rate, Timecode::parse("01:00:00:00").unwrap()).unwrap();
//! assert_eq!(main.frames("01:00:04:00").unwrap(), 100);
//! assert_eq!(main.frames("2s").unwrap(), 50);
//! assert_eq!(main.range("0..+5s").unwrap().len(), 125);
//!
//! // At 24000/1001, 12 s is not a whole frame: an error, never rounded.
//! let film = Clock::new(Rate::parse("24000/1001").unwrap());
//! let err = film.frames("12s").unwrap_err();
//! assert_eq!(err.code(), "veel-time-inexact");
//! ```

mod ratio;

pub use ratio::Ratio;
use std::fmt;

/// The most digits a number in a time word may have, so every conversion fits.
const MAX_DIGITS: usize = 18;

/// An exact frame rate: a positive rational in lowest terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rate {
    num: u32,
    den: u32,
}

impl Rate {
    /// `num / den` frames per second, if both are at least 1.
    pub fn new(num: u32, den: u32) -> Option<Rate> {
        if num == 0 || den == 0 {
            return None;
        }
        let r = Ratio::new(num.into(), den.into())?;
        Some(Rate {
            num: r.num() as u32,
            den: r.den() as u32,
        })
    }

    /// Parses a rate word: a whole number (`25`) or a fraction (`30000/1001`). A decimal is an
    /// error with the exact rate as its fix (`29.97 is not exact; write 30000/1001`).
    pub fn parse(word: &str) -> Result<Rate, TimeError> {
        let bad = || TimeError::NotARate {
            word: word.to_string(),
        };
        let digits =
            |s: &str| !s.is_empty() && s.len() <= 9 && s.bytes().all(|b| b.is_ascii_digit());
        if let Some((a, b)) = word.split_once('/') {
            if !digits(a) || !digits(b) {
                return Err(bad());
            }
            return Rate::new(a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?)
                .ok_or_else(bad);
        }
        if let Some((a, b)) = word.split_once('.') {
            if !digits(a) || !digits(b) {
                return Err(bad());
            }
            let value = Ratio::new(
                format!("{a}{b}").parse().map_err(|_| bad())?,
                10i128.pow(b.len() as u32),
            )
            .ok_or_else(bad)?;
            return Err(TimeError::NotExactRate {
                word: word.to_string(),
                exact: exact_rate_for(value).ok_or_else(bad)?,
            });
        }
        if !digits(word) {
            return Err(bad());
        }
        Rate::new(word.parse().map_err(|_| bad())?, 1).ok_or_else(bad)
    }

    /// The numerator, in lowest terms.
    pub fn num(self) -> u32 {
        self.num
    }

    /// The denominator, in lowest terms.
    pub fn den(self) -> u32 {
        self.den
    }

    /// The rate as an exact rational, in frames per second.
    pub fn ratio(self) -> Ratio {
        Ratio::new(self.num.into(), self.den.into()).expect("a rate is positive")
    }

    /// Frames per timecode second: the rate rounded up to a whole number, so `30000/1001`
    /// counts 30 frames to a timecode second, as non-drop timecode does.
    pub fn timecode_base(self) -> u32 {
        self.num.div_ceil(self.den)
    }

    /// The exact time of `frames` frames, in seconds.
    pub fn seconds(self, frames: i64) -> Ratio {
        Ratio::int(frames.into())
            .checked_div(self.ratio())
            .expect("an i64 frame count divides exactly")
    }

    /// The exact number of frames in `seconds`, which may not be whole.
    pub fn frames_exact(self, seconds: Ratio) -> Option<Ratio> {
        seconds.checked_mul(self.ratio())
    }
}

/// The exact rate a decimal rate word stands for: the NTSC rate it abbreviates (`29.97` is
/// `30000/1001`), or else the decimal's own value as a fraction.
fn exact_rate_for(value: Ratio) -> Option<Rate> {
    for base in [24u32, 30, 48, 60, 120, 240] {
        let ntsc = Ratio::new(i128::from(base) * 1000, 1001)?;
        let diff = value.checked_sub(ntsc)?;
        // 23.976 and 23.98, 29.97, 59.94, 47.952, 119.88: within a hundredth.
        if diff.num().abs() * 100 < diff.den() {
            return Rate::new(base * 1000, 1001);
        }
    }
    Rate::new(
        u32::try_from(value.num()).ok()?,
        u32::try_from(value.den()).ok()?,
    )
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

/// A non-drop timecode label, `hh:mm:ss:ff`, before a rate checks its fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Timecode {
    /// Hours.
    pub hours: u32,
    /// Minutes, below 60 once checked.
    pub minutes: u32,
    /// Seconds, below 60 once checked.
    pub seconds: u32,
    /// Frames, below the rate's [`Rate::timecode_base`] once checked.
    pub frames: u32,
}

impl Timecode {
    /// Parses `hh:mm:ss:ff`: four fields of digits, minutes and seconds of two digits each.
    pub fn parse(word: &str) -> Result<Timecode, TimeError> {
        let bad = || TimeError::NotATime {
            word: word.to_string(),
        };
        let fields: Vec<&str> = word.split(':').collect();
        let [h, m, s, f] = fields[..] else {
            return Err(bad());
        };
        let field = |t: &str, two: bool| -> Option<u32> {
            let ok = if two {
                t.len() == 2
            } else {
                (1..=6).contains(&t.len())
            };
            (ok && t.bytes().all(|b| b.is_ascii_digit()))
                .then(|| t.parse().ok())
                .flatten()
        };
        Ok(Timecode {
            hours: field(h, false).ok_or_else(bad)?,
            minutes: field(m, true).ok_or_else(bad)?,
            seconds: field(s, true).ok_or_else(bad)?,
            frames: field(f, false).ok_or_else(bad)?,
        })
    }

    /// The label's frame count from `00:00:00:00`, counting non-drop at `rate`, if its fields
    /// are in range for it.
    pub fn count(self, rate: Rate) -> Result<i64, TimeError> {
        let base = rate.timecode_base();
        let field = |what: &'static str, value: u32, limit: u32| {
            if value < limit {
                Ok(())
            } else {
                Err(TimeError::TimecodeField {
                    word: self.to_string(),
                    what,
                    value,
                    limit,
                    rate,
                })
            }
        };
        field("minutes", self.minutes, 60)?;
        field("seconds", self.seconds, 60)?;
        field("frames", self.frames, base)?;
        let secs = (i128::from(self.hours) * 60 + i128::from(self.minutes)) * 60
            + i128::from(self.seconds);
        i64::try_from(secs * i128::from(base) + i128::from(self.frames)).map_err(|_| {
            TimeError::TooLarge {
                word: self.to_string(),
            }
        })
    }

    /// The label of frame count `count` from `00:00:00:00`, counting non-drop at `rate`, if
    /// `count` is not negative and its hours fit.
    pub fn from_count(count: i64, rate: Rate) -> Option<Timecode> {
        if count < 0 {
            return None;
        }
        let base = i64::from(rate.timecode_base());
        let secs = count / base;
        Some(Timecode {
            hours: u32::try_from(secs / 3600).ok()?,
            minutes: (secs / 60 % 60) as u32,
            seconds: (secs % 60) as u32,
            frames: (count % base) as u32,
        })
    }
}

impl fmt::Display for Timecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}:{:02}",
            self.hours, self.minutes, self.seconds, self.frames
        )
    }
}

/// The unit a time word is written in, which write-back keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    /// A bare integer: frames at the sequence's rate.
    Frames,
    /// `s`.
    Seconds,
    /// `ms`.
    Millis,
    /// `hh:mm:ss:ff`.
    Timecode,
}

/// What a time word says, before a [`Clock`] turns it into frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Time {
    /// A whole number of frames.
    Frames(i64),
    /// Seconds, an exact decimal.
    Seconds(Ratio),
    /// Milliseconds, an exact decimal.
    Millis(Ratio),
    /// A timecode label.
    Timecode(Timecode),
}

impl Time {
    /// Parses a time word: `120`, `-12`, `2s`, `1.5s`, `250ms`, `01:00:10:12`.
    pub fn parse(word: &str) -> Result<Time, TimeError> {
        let bad = || TimeError::NotATime {
            word: word.to_string(),
        };
        if word.contains(':') {
            return Timecode::parse(word).map(Time::Timecode);
        }
        let body = word.strip_prefix('-').unwrap_or(word);
        let number_len = body
            .bytes()
            .position(|b| !(b.is_ascii_digit() || b == b'.'))
            .unwrap_or(body.len());
        let (number, unit) = body.split_at(number_len);
        let (int, frac) = number.split_once('.').unwrap_or((number, ""));
        let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
        if int.is_empty()
            || !digits(int)
            || !digits(frac)
            || (number.contains('.') && frac.is_empty())
        {
            return Err(bad());
        }
        if int.len() + frac.len() > MAX_DIGITS {
            return Err(TimeError::TooLarge {
                word: word.to_string(),
            });
        }
        let sign = if word.starts_with('-') { -1 } else { 1 };
        let scaled: i128 = format!("{int}{frac}").parse().map_err(|_| bad())?;
        let value = Ratio::new(sign * scaled, 10i128.pow(frac.len() as u32)).ok_or_else(bad)?;
        match unit {
            "" if value.is_integer() => Ok(Time::Frames(value.num() as i64)),
            "" => Err(TimeError::Inexact(Box::new(Inexact {
                word: word.to_string(),
                frames: value,
                rate: None,
                below: value.floor() as i64,
                above: value.floor() as i64 + 1,
                choices: [value.floor().to_string(), (value.floor() + 1).to_string()],
            }))),
            "s" => Ok(Time::Seconds(value)),
            "ms" => Ok(Time::Millis(value)),
            _ => Err(TimeError::UnknownUnit {
                word: word.to_string(),
                unit: unit.to_string(),
            }),
        }
    }

    /// The unit it is written in.
    pub fn unit(&self) -> Unit {
        match self {
            Time::Frames(_) => Unit::Frames,
            Time::Seconds(_) => Unit::Seconds,
            Time::Millis(_) => Unit::Millis,
            Time::Timecode(_) => Unit::Timecode,
        }
    }
}

/// A half-open range of frames, `[start, end)`: `end` is the first frame after it (V21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Range {
    start: i64,
    end: i64,
}

impl Range {
    /// `[start, end)`, if it does not end before it starts. An empty range (`start == end`) is a
    /// range; whether an element may be empty is the checker's.
    pub fn new(start: i64, end: i64) -> Option<Range> {
        (start <= end).then_some(Range { start, end })
    }

    /// The first frame in it.
    pub fn start(self) -> i64 {
        self.start
    }

    /// The first frame after it.
    pub fn end(self) -> i64 {
        self.end
    }

    /// Its length in frames, `end - start`, with no ±1.
    pub fn len(self) -> i64 {
        self.end - self.start
    }

    /// Whether it holds no frame.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Whether `frame` is in it: `start <= frame < end`.
    pub fn contains(self, frame: i64) -> bool {
        self.start <= frame && frame < self.end
    }

    /// The frames both ranges hold, as a length: 0 when they only touch.
    pub fn overlap(self, o: Range) -> i64 {
        (self.end.min(o.end) - self.start.max(o.start)).max(0)
    }
}

/// What a frame is, and how timecode counts: turns time words into whole frames and back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    rate: Rate,
    timecode_rate: Rate,
    start: i64,
}

impl Clock {
    /// Frames at `rate`, with timecode at the same rate starting at `00:00:00:00`.
    pub fn new(rate: Rate) -> Clock {
        Clock {
            rate,
            timecode_rate: rate,
            start: 0,
        }
    }

    /// Frames at `rate` (the sequence's), with timecode counted at `timecode_rate` from `start`
    /// at frame 0: a sequence's own timecode (the same rate, its `start=`), or a source's (its
    /// rate and `start-timecode`, for a clip's source range).
    pub fn with_timecode(
        rate: Rate,
        timecode_rate: Rate,
        start: Timecode,
    ) -> Result<Clock, TimeError> {
        Ok(Clock {
            rate,
            timecode_rate,
            start: start.count(timecode_rate)?,
        })
    }

    /// The rate frames count at.
    pub fn rate(&self) -> Rate {
        self.rate
    }

    /// The whole frame a position word names. A timecode is absolute: its frame 0 is the clock's
    /// start, and a label before it is an error.
    pub fn frames(&self, word: &str) -> Result<i64, TimeError> {
        self.convert(word, Time::parse(word)?, false)
    }

    /// The whole number of frames a duration word names. A timecode counts from `00:00:00:00`.
    pub fn duration(&self, word: &str) -> Result<i64, TimeError> {
        self.convert(word, Time::parse(word)?, true)
    }

    /// The half-open range an `in..out` or `in..+duration` word names.
    pub fn range(&self, word: &str) -> Result<Range, TimeError> {
        let (a, b) = word.split_once("..").ok_or_else(|| TimeError::NotARange {
            word: word.to_string(),
        })?;
        let start = self.frames(a)?;
        let end = match b.strip_prefix('+') {
            Some(len) => {
                let len = self.duration(len)?;
                start.checked_add(len).ok_or_else(|| TimeError::TooLarge {
                    word: word.to_string(),
                })?
            }
            None => self.frames(b)?,
        };
        Range::new(start, end).ok_or_else(|| TimeError::Backwards {
            word: word.to_string(),
        })
    }

    /// `time`, from `word`, in whole frames: the one exact conversion.
    fn convert(&self, word: &str, time: Time, duration: bool) -> Result<i64, TimeError> {
        let large = || TimeError::TooLarge {
            word: word.to_string(),
        };
        let frames = match time {
            Time::Frames(n) => return Ok(n),
            Time::Seconds(s) => self.rate.frames_exact(s).ok_or_else(large)?,
            Time::Millis(ms) => self
                .rate
                .frames_exact(ms.checked_div(Ratio::int(1000)).ok_or_else(large)?)
                .ok_or_else(large)?,
            Time::Timecode(tc) => {
                let count = tc.count(self.timecode_rate)?;
                let index = if duration { count } else { count - self.start };
                if index < 0 {
                    return Err(TimeError::BeforeStart {
                        word: word.to_string(),
                        start: Timecode::from_count(self.start, self.timecode_rate)
                            .expect("the start is a label"),
                    });
                }
                self.rate
                    .frames_exact(self.timecode_rate.seconds(index))
                    .ok_or_else(large)?
            }
        };
        if frames.is_integer() {
            return i64::try_from(frames.num()).map_err(|_| large());
        }
        let below = i64::try_from(frames.floor()).map_err(|_| large())?;
        let above = below.checked_add(1).ok_or_else(large)?;
        let unit = time.unit();
        let choice = |f: i64| {
            self.write(f, unit, duration)
                .unwrap_or_else(|| f.to_string())
        };
        Err(TimeError::Inexact(Box::new(Inexact {
            word: word.to_string(),
            frames,
            rate: Some(self.rate),
            below,
            above,
            choices: [choice(below), choice(above)],
        })))
    }

    /// The word in `unit` that names exactly `frames`, if there is one: always in frames, in `s`
    /// and `ms` when the time is a finite decimal of at most 18 digits (the most [`Time::parse`]
    /// reads back), and in timecode when the frame falls on a timecode frame at or after the start
    /// (from `00:00:00:00` for a duration).
    pub fn write(&self, frames: i64, unit: Unit, duration: bool) -> Option<String> {
        let seconds = self.rate.seconds(frames);
        let readable = |decimal: String| {
            let digits = decimal.bytes().filter(u8::is_ascii_digit).count();
            (digits <= MAX_DIGITS).then_some(decimal)
        };
        match unit {
            Unit::Frames => Some(frames.to_string()),
            Unit::Seconds => Some(format!("{}s", readable(seconds.to_decimal()?)?)),
            Unit::Millis => Some(format!(
                "{}ms",
                readable(seconds.checked_mul(Ratio::int(1000))?.to_decimal()?)?
            )),
            Unit::Timecode => {
                let index = self.timecode_rate.frames_exact(seconds)?;
                if !index.is_integer() {
                    return None;
                }
                let index = i64::try_from(index.num()).ok()?;
                let count = if duration {
                    index
                } else {
                    index.checked_add(self.start)?
                };
                Timecode::from_count(count, self.timecode_rate).map(|t| t.to_string())
            }
        }
    }
}

/// A time that does not land on a whole frame (`veel-time-inexact`): never rounded, reported with
/// the whole frames on either side of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inexact {
    /// The word as written.
    pub word: String,
    /// The exact number of frames it names.
    pub frames: Ratio,
    /// The rate frames count at; `None` for a bare decimal, which is frames already.
    pub rate: Option<Rate>,
    /// The whole frame just before it.
    pub below: i64,
    /// The whole frame just after it.
    pub above: i64,
    /// `below` and `above` written in the word's unit where that is exact, otherwise in frames.
    pub choices: [String; 2],
}

/// Why a time word does not give whole frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeError {
    /// Not a time word at all.
    NotATime {
        /// The word.
        word: String,
    },
    /// Not a range word: no `..`.
    NotARange {
        /// The word.
        word: String,
    },
    /// A unit Veel's time does not read.
    UnknownUnit {
        /// The word.
        word: String,
        /// Its unit.
        unit: String,
    },
    /// Not a rate word: not a positive whole number or fraction.
    NotARate {
        /// The word.
        word: String,
    },
    /// A rate written as a decimal, with the exact rate it stands for.
    NotExactRate {
        /// The word.
        word: String,
        /// The exact rate.
        exact: Rate,
    },
    /// A timecode field out of range for its rate.
    TimecodeField {
        /// The timecode.
        word: String,
        /// `minutes`, `seconds` or `frames`.
        what: &'static str,
        /// The field's value.
        value: u32,
        /// The first value out of range.
        limit: u32,
        /// The rate the timecode counts at.
        rate: Rate,
    },
    /// A timecode position before the clock's start.
    BeforeStart {
        /// The word.
        word: String,
        /// The timecode of frame 0.
        start: Timecode,
    },
    /// A range that ends before it starts.
    Backwards {
        /// The word.
        word: String,
    },
    /// A value too large for a frame count.
    TooLarge {
        /// The word.
        word: String,
    },
    /// A time that does not land on a whole frame.
    Inexact(Box<Inexact>),
}

impl TimeError {
    /// The diagnostic code the checker reports it with (spec section 14).
    pub fn code(&self) -> &'static str {
        match self {
            TimeError::Inexact(_) | TimeError::NotExactRate { .. } => "veel-time-inexact",
            TimeError::UnknownUnit { .. } => "veel-unit",
            TimeError::BeforeStart { .. }
            | TimeError::Backwards { .. }
            | TimeError::TooLarge { .. } => "veel-range",
            TimeError::NotATime { .. }
            | TimeError::NotARange { .. }
            | TimeError::NotARate { .. }
            | TimeError::TimecodeField { .. } => "veel-type",
        }
    }
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeError::NotATime { word } => write!(
                f,
                "`{word}` is not a time: write frames (`120`), seconds (`2s`), milliseconds \
                 (`250ms`) or timecode (`01:00:10:12`)"
            ),
            TimeError::NotARange { word } => write!(
                f,
                "`{word}` is not a range: write `in..out` or `in..+duration`"
            ),
            TimeError::UnknownUnit { word, unit } => write!(
                f,
                "`{word}`: `{unit}` is not a unit of time here; write frames, `s`, `ms` or timecode"
            ),
            TimeError::NotARate { word } => write!(
                f,
                "`{word}` is not a rate: write a whole number (`25`) or a fraction (`30000/1001`)"
            ),
            TimeError::NotExactRate { word, exact } => {
                write!(f, "{word} is not exact; write {exact}")
            }
            TimeError::TimecodeField {
                word,
                what,
                value,
                limit,
                rate,
            } => write!(
                f,
                "`{word}` has {value} {what}; timecode at {rate} counts {what} from 0 to {}",
                limit - 1
            ),
            TimeError::BeforeStart { word, start } => {
                write!(f, "`{word}` is before the timecode of frame 0, {start}")
            }
            TimeError::Backwards { word } => {
                write!(f, "`{word}` ends before it starts")
            }
            TimeError::TooLarge { word } => write!(f, "`{word}` is too large for a frame count"),
            TimeError::Inexact(e) => {
                let at = match e.rate {
                    Some(rate) => format!(" at {rate} frames per second"),
                    None => String::new(),
                };
                write!(
                    f,
                    "`{}` is {} frames{at}, not a whole frame; write `{}` or `{}`",
                    e.word,
                    e.frames.approx(3),
                    e.choices[0],
                    e.choices[1]
                )
            }
        }
    }
}

impl std::error::Error for TimeError {}

#[cfg(test)]
mod tests;

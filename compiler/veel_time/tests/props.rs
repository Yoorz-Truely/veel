//! Property tests: every conversion is exact or an error, never rounded (V20, corrected), at
//! common and arbitrary rational rates, with `30000/1001`'s boundaries tested on their own.

use proptest::prelude::*;
use veel_time::{Clock, Range, Rate, Ratio, Time, TimeError, Timecode, Unit};

fn ntsc() -> Rate {
    Rate::new(30000, 1001).unwrap()
}

/// Common rates, and arbitrary rationals up to 240,000/1,001.
fn rates() -> impl Strategy<Value = Rate> {
    prop_oneof![
        prop::sample::select(vec![
            (24, 1),
            (25, 1),
            (30, 1),
            (50, 1),
            (60, 1),
            (24000, 1001),
            (30000, 1001),
            (60000, 1001),
        ])
        .prop_map(|(n, d)| Rate::new(n, d).unwrap()),
        (1u32..=240_000, 1u32..=1001).prop_filter_map("a rate", |(n, d)| Rate::new(n, d)),
    ]
}

/// The exact frames `seconds` names at `rate`.
fn exact(rate: Rate, seconds: Ratio) -> Ratio {
    rate.frames_exact(seconds).unwrap()
}

/// Asserts `result` is the conversion of a time naming exactly `frames`: whole frames when it is
/// whole, otherwise an inexact error with the whole frames on either side, whose choices (when
/// written in the word's unit) convert to exactly those frames.
fn assert_exact_or_inexact(clock: &Clock, result: Result<i64, TimeError>, frames: Ratio) {
    match result {
        Ok(f) => {
            assert!(frames.is_integer(), "{frames} frames came back whole: {f}");
            assert_eq!(Ratio::int(f.into()), frames);
        }
        Err(TimeError::Inexact(e)) => {
            assert!(!frames.is_integer(), "{frames} frames is whole");
            assert_eq!(e.frames, frames);
            assert_eq!(i128::from(e.below), frames.floor());
            assert_eq!(e.above, e.below + 1);
            for (choice, want) in e.choices.iter().zip([e.below, e.above]) {
                assert_eq!(clock.frames(choice), Ok(want), "choice {choice}");
            }
        }
        Err(other) => panic!("{other:?}"),
    }
}

proptest! {
    /// Frames to seconds and back is the identity, and a frame written in seconds (when the
    /// decimal is finite) reads back as that frame.
    #[test]
    fn frames_round_trip_through_seconds(rate in rates(), f in -1_000_000i64..1_000_000) {
        prop_assert_eq!(exact(rate, rate.seconds(f)), Ratio::int(f.into()));
        let clock = Clock::new(rate);
        for unit in [Unit::Frames, Unit::Seconds, Unit::Millis] {
            if let Some(word) = clock.write(f, unit, false) {
                prop_assert_eq!(clock.frames(&word), Ok(f), "{}", word);
            }
        }
    }

    /// A decimal of seconds or milliseconds gives whole frames exactly when it lands on one, and
    /// otherwise an inexact error: never a rounded frame.
    #[test]
    fn seconds_and_millis_are_exact_or_errors(
        rate in rates(),
        int in 0i64..1_000_000,
        frac in 0u32..1000,
        negative in any::<bool>(),
        millis in any::<bool>(),
    ) {
        let sign = if negative { "-" } else { "" };
        let unit = if millis { "ms" } else { "s" };
        let word = format!("{sign}{int}.{frac:03}{unit}");
        let value = Ratio::new(
            (i128::from(int) * 1000 + i128::from(frac)) * if negative { -1 } else { 1 },
            if millis { 1_000_000 } else { 1000 },
        ).unwrap();
        let clock = Clock::new(rate);
        assert_exact_or_inexact(&clock, clock.frames(&word), exact(rate, value));
    }

    /// At 30000/1001, `n ms` is a whole frame exactly when 1001 divides n, and is then 30n/1001
    /// frames: 1001 ms is 30 frames.
    #[test]
    fn ntsc_millis_land_on_frames_only_at_multiples_of_1001(n in 0i64..10_000_000) {
        let clock = Clock::new(ntsc());
        let got = clock.frames(&format!("{n}ms"));
        if n % 1001 == 0 {
            prop_assert_eq!(got, Ok(30 * n / 1001));
        } else {
            prop_assert_eq!(got.unwrap_err().code(), "veel-time-inexact");
        }
    }

    /// At 30000/1001, `n s` is a whole frame exactly when 1001 divides n.
    #[test]
    fn ntsc_seconds_land_on_frames_only_at_multiples_of_1001(n in 0i64..1_000_000_000) {
        let clock = Clock::new(ntsc());
        let got = clock.frames(&format!("{n}s"));
        if n % 1001 == 0 {
            prop_assert_eq!(got, Ok(30000 * n / 1001));
        } else {
            prop_assert_eq!(got.unwrap_err().code(), "veel-time-inexact");
        }
    }

    /// The boundaries at 30000/1001: `k × 1001 ms` is frame 30k, and a millisecond either side is
    /// not a frame, with 30k and its neighbour as the choices.
    #[test]
    fn ntsc_boundaries_either_side_of_a_whole_frame(k in 1i64..10_000_000) {
        let clock = Clock::new(ntsc());
        let at = k * 1001;
        prop_assert_eq!(clock.frames(&format!("{at}ms")), Ok(30 * k));
        let TimeError::Inexact(after) = clock.frames(&format!("{}ms", at + 1)).unwrap_err() else {
            panic!()
        };
        prop_assert_eq!((after.below, after.above), (30 * k, 30 * k + 1));
        let TimeError::Inexact(before) = clock.frames(&format!("{}ms", at - 1)).unwrap_err() else {
            panic!()
        };
        prop_assert_eq!((before.below, before.above), (30 * k - 1, 30 * k));
        // The same boundary in seconds, three decimals: 1.001 s is 30 frames.
        let secs = format!("{}.{:03}s", at / 1000, at % 1000);
        prop_assert_eq!(clock.frames(&secs), Ok(30 * k));
    }

    /// Non-drop timecode at any rate: a count's label counts back to it, prints and parses back,
    /// and the next count moves the frames field on by one or carries into the seconds.
    #[test]
    fn timecode_counts_round_trip(rate in rates(), count in 0i64..1_000_000_000) {
        let label = Timecode::from_count(count, rate).unwrap();
        prop_assert_eq!(label.count(rate), Ok(count));
        prop_assert_eq!(Timecode::parse(&label.to_string()), Ok(label));
        let next = Timecode::from_count(count + 1, rate).unwrap();
        if label.frames + 1 == rate.timecode_base() {
            prop_assert_eq!(next.frames, 0);
            prop_assert_eq!(next.seconds, (label.seconds + 1) % 60);
        } else {
            prop_assert_eq!(next.frames, label.frames + 1);
        }
    }

    /// At 30000/1001 a timecode label names its count's frame, and the label after
    /// `hh:mm:ss:29` is the next second's `00`: there is no `:30`.
    #[test]
    fn ntsc_timecode_names_its_frame(count in 0i64..10_000_000) {
        let clock = Clock::new(ntsc());
        let label = Timecode::from_count(count, ntsc()).unwrap();
        prop_assert!(label.frames < 30);
        prop_assert_eq!(clock.frames(&label.to_string()), Ok(count));
        let mut over = label;
        over.frames = 30;
        prop_assert_eq!(clock.frames(&over.to_string()).unwrap_err().code(), "veel-type");
    }

    /// A source at 24000/1001 in a sequence at 30000/1001: source frame d is 5d/4 sequence frames,
    /// whole exactly when 4 divides d.
    #[test]
    fn a_source_timecode_at_another_rate_is_exact_or_an_error(d in 0i64..10_000_000) {
        let film = Rate::new(24000, 1001).unwrap();
        let clock = Clock::with_timecode(ntsc(), film, Timecode::parse("00:00:00:00").unwrap()).unwrap();
        let word = Timecode::from_count(d, film).unwrap().to_string();
        let frames = Ratio::new(i128::from(d) * 5, 4).unwrap();
        assert_exact_or_inexact(&clock, clock.frames(&word), frames);
        if d % 4 == 0 {
            prop_assert_eq!(clock.frames(&word), Ok(d * 5 / 4));
        }
    }

    /// `a..+d` is `a..a+d`; a range's length is `end - start`; ranges that touch do not overlap,
    /// and a range holds its start but not its end.
    #[test]
    fn ranges_are_half_open(a in -1_000_000i64..1_000_000, d in 0i64..1_000_000, e in 0i64..1_000_000) {
        let clock = Clock::new(Rate::new(25, 1).unwrap());
        let r = clock.range(&format!("{a}..+{d}")).unwrap();
        prop_assert_eq!(r, clock.range(&format!("{a}..{}", a + d)).unwrap());
        prop_assert_eq!(r.len(), d);
        let next = Range::new(a + d, a + d + e).unwrap();
        prop_assert_eq!(r.overlap(next), 0);
        prop_assert!(!r.contains(a + d));
        prop_assert_eq!(r.contains(a), d > 0);
        prop_assert_eq!(r.overlap(r), d);
    }

    /// Parsing a frame count and printing it again changes nothing, and a frame count needs no
    /// rate.
    #[test]
    fn frame_words_are_frames_at_any_rate(rate in rates(), f in -999_999_999_999_999_999i64..=999_999_999_999_999_999) {
        prop_assert_eq!(Time::parse(&f.to_string()), Ok(Time::Frames(f)));
        prop_assert_eq!(Clock::new(rate).frames(&f.to_string()), Ok(f));
    }
}

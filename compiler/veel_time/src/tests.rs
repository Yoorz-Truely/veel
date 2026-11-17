//! Unit tests: the spec's examples, every error with its code and wording, and the edges.

use super::*;

fn rate(word: &str) -> Rate {
    Rate::parse(word).unwrap()
}

fn tc(word: &str) -> Timecode {
    Timecode::parse(word).unwrap()
}

#[test]
fn the_specs_examples() {
    // Spec section 6: with start=01:00:00:00 at 25, 01:00:04:00 is frame 100 and 00:00:04:00
    // is before frame 0.
    let main = Clock::with_timecode(rate("25"), rate("25"), tc("01:00:00:00")).unwrap();
    assert_eq!(main.frames("01:00:04:00"), Ok(100));
    let err = main.frames("00:00:04:00").unwrap_err();
    assert_eq!(err.code(), "veel-range");
    assert_eq!(
        err.to_string(),
        "`00:00:04:00` is before the timecode of frame 0, 01:00:00:00"
    );
    // Spec section 3: at 24000/1001, `at 12s` would be 287.71 frames, an error.
    let film = Clock::new(rate("24000/1001"));
    let err = film.frames("12s").unwrap_err();
    assert_eq!(err.code(), "veel-time-inexact");
    assert_eq!(
        err.to_string(),
        "`12s` is 287.712… frames at 24000/1001 frames per second, not a whole frame; write `287` or `12.012s`"
    );
    // Spec section 14: a decimal rate.
    let err = Rate::parse("29.97").unwrap_err();
    assert_eq!(err.code(), "veel-time-inexact");
    assert_eq!(err.to_string(), "29.97 is not exact; write 30000/1001");
}

#[test]
fn the_choices_keep_the_unit_where_it_is_exact() {
    let pal = Clock::new(rate("25"));
    let TimeError::Inexact(e) = pal.frames("0.03s").unwrap_err() else {
        panic!()
    };
    assert_eq!((e.below, e.above), (0, 1));
    assert_eq!(e.choices, ["0s", "0.04s"]);
    let TimeError::Inexact(e) = pal.frames("10ms").unwrap_err() else {
        panic!()
    };
    assert_eq!(e.choices, ["0ms", "40ms"]);
    // A source at 50 in a sequence at 25: one source frame is half a sequence frame, and the
    // choices are the source timecodes on whole sequence frames.
    let source = Clock::with_timecode(rate("25"), rate("50"), tc("00:00:00:00")).unwrap();
    let TimeError::Inexact(e) = source.frames("00:00:00:01").unwrap_err() else {
        panic!()
    };
    assert_eq!(e.choices, ["00:00:00:00", "00:00:00:02"]);
    // A bare decimal is frames already.
    let TimeError::Inexact(e) = Time::parse("1.5").unwrap_err() else {
        panic!()
    };
    assert_eq!((e.below, e.above, e.rate), (1, 2, None));
    assert_eq!(
        TimeError::Inexact(e).to_string(),
        "`1.5` is 1.5 frames, not a whole frame; write `1` or `2`"
    );
}

#[test]
fn rates() {
    assert_eq!(rate("25"), Rate::new(25, 1).unwrap());
    assert_eq!(rate("30000/1001").to_string(), "30000/1001");
    assert_eq!(rate("50/2").to_string(), "25");
    for (decimal, exact) in [
        ("23.976", "24000/1001"),
        ("23.98", "24000/1001"),
        ("29.97", "30000/1001"),
        ("59.94", "60000/1001"),
        ("47.952", "48000/1001"),
        ("119.88", "120000/1001"),
        ("12.5", "25/2"),
        ("25.0", "25"),
    ] {
        match Rate::parse(decimal) {
            Err(TimeError::NotExactRate { exact: e, .. }) => {
                assert_eq!(e.to_string(), exact, "{decimal}")
            }
            other => panic!("{decimal}: {other:?}"),
        }
    }
    for bad in [
        "0", "-25", "25/0", "0/1", "abc", "1/2/3", "", "25fps", "1e3", ".5",
    ] {
        let err = Rate::parse(bad).unwrap_err();
        assert!(matches!(err, TimeError::NotARate { .. }), "{bad}: {err:?}");
        assert_eq!(err.code(), "veel-type");
    }
    assert_eq!(rate("30000/1001").timecode_base(), 30);
    assert_eq!(rate("24000/1001").timecode_base(), 24);
    assert_eq!(rate("25").timecode_base(), 25);
}

#[test]
fn time_words() {
    assert_eq!(Time::parse("120"), Ok(Time::Frames(120)));
    assert_eq!(Time::parse("-12"), Ok(Time::Frames(-12)));
    assert_eq!(
        Time::parse("1.5s"),
        Ok(Time::Seconds(Ratio::new(3, 2).unwrap()))
    );
    assert_eq!(Time::parse("-2s"), Ok(Time::Seconds(Ratio::int(-2))));
    assert_eq!(Time::parse("250ms"), Ok(Time::Millis(Ratio::int(250))));
    assert_eq!(
        Time::parse("01:00:10:12"),
        Ok(Time::Timecode(tc("01:00:10:12")))
    );
    for bad in [
        "",
        "-",
        "1.",
        ".5s",
        "--1",
        "+5",
        "1.2.3s",
        "s",
        "00:0:00:00",
        "00:00:10;00",
        "1:02.250",
    ] {
        let err = Time::parse(bad).unwrap_err();
        assert!(matches!(err, TimeError::NotATime { .. }), "{bad}: {err:?}");
    }
    for unit in ["4800smp", "2min", "1h", "12deg", "1e3"] {
        let err = Time::parse(unit).unwrap_err();
        assert_eq!(err.code(), "veel-unit", "{unit}");
    }
    let err = Time::parse("1234567890123456789s").unwrap_err();
    assert!(matches!(err, TimeError::TooLarge { .. }));
}

#[test]
fn timecode_fields() {
    let ntsc = rate("30000/1001");
    assert_eq!(tc("00:00:00:29").count(ntsc), Ok(29));
    let err = tc("00:00:00:30").count(ntsc).unwrap_err();
    assert_eq!(
        err.to_string(),
        "`00:00:00:30` has 30 frames; timecode at 30000/1001 counts frames from 0 to 29"
    );
    assert!(tc("00:60:00:00").count(ntsc).is_err());
    assert!(tc("00:00:60:00").count(ntsc).is_err());
    // Non-drop: an hour of timecode at 30000/1001 is 108,000 frames, 3,603.6 seconds of media.
    assert_eq!(tc("01:00:00:00").count(ntsc), Ok(108_000));
    assert_eq!(ntsc.seconds(108_000), Ratio::new(36036, 10).unwrap());
    assert_eq!(Timecode::from_count(108_000, ntsc), Some(tc("01:00:00:00")));
    assert_eq!(Timecode::from_count(-1, ntsc), None);
    assert_eq!(tc("1:02:03:04").to_string(), "01:02:03:04");
}

#[test]
fn ranges() {
    let pal = Clock::new(rate("25"));
    let r = pal.range("0..+5s").unwrap();
    assert_eq!((r.start(), r.end(), r.len()), (0, 125, 125));
    assert_eq!(pal.range("5..5").unwrap().len(), 0);
    assert!(pal.range("5..5").unwrap().is_empty());
    for (bad, code) in [
        ("10..5", "veel-range"),
        ("0..+-5", "veel-range"),
        ("5", "veel-type"),
    ] {
        assert_eq!(pal.range(bad).unwrap_err().code(), code, "{bad}");
    }
    assert_eq!(
        pal.range("10..5").unwrap_err().to_string(),
        "`10..5` ends before it starts"
    );
    // A clip's source range in the source's timecode (spec section 3).
    let source = Clock::with_timecode(rate("25"), rate("25"), tc("10:00:00:00")).unwrap();
    let r = source.range("10:02:10:00..+12s").unwrap();
    assert_eq!((r.start(), r.len()), (3250, 300));
    // Half-open: touching ranges do not overlap.
    let a = Range::new(0, 10).unwrap();
    let b = Range::new(10, 20).unwrap();
    assert_eq!(a.overlap(b), 0);
    assert_eq!(a.overlap(Range::new(7, 12).unwrap()), 3);
    assert!(a.contains(9) && !a.contains(10) && !a.contains(-1));
    assert_eq!(Range::new(5, 4), None);
}

#[test]
fn writing_back() {
    let ntsc = Clock::new(rate("30000/1001"));
    assert_eq!(
        ntsc.write(30, Unit::Seconds, false).as_deref(),
        Some("1.001s")
    );
    assert_eq!(
        ntsc.write(30, Unit::Millis, false).as_deref(),
        Some("1001ms")
    );
    assert_eq!(ntsc.write(1, Unit::Seconds, false), None);
    assert_eq!(ntsc.write(1, Unit::Frames, false).as_deref(), Some("1"));
    let main = Clock::with_timecode(rate("25"), rate("25"), tc("01:00:00:00")).unwrap();
    assert_eq!(
        main.write(100, Unit::Timecode, false).as_deref(),
        Some("01:00:04:00")
    );
    assert_eq!(
        main.write(100, Unit::Timecode, true).as_deref(),
        Some("00:00:04:00")
    );
    assert_eq!(main.write(-1, Unit::Timecode, true), None);
    assert_eq!(main.duration("00:00:04:00"), Ok(100));
}

/// Annilay's B-118: a frame whose exact decimal has more digits than a time word may hold is not
/// written in `s` or `ms`, so every word `write` gives reads back; an inexact error's choice falls
/// back to frames. Both cases were found at random by the property tests.
#[test]
fn writing_back_never_gives_a_word_too_long_to_read() {
    let odd = Clock::new(rate("32768/113"));
    assert_eq!(odd.write(292_857, Unit::Seconds, false), None);
    assert_eq!(odd.write(292_857, Unit::Millis, false), None);
    assert!(matches!(
        odd.frames("1009.913360595703125s"),
        Err(TimeError::TooLarge { .. })
    ));
    for f in [0, 1, 4096, 292_857, -292_857, 999_999] {
        for unit in [Unit::Frames, Unit::Seconds, Unit::Millis] {
            if let Some(word) = odd.write(f, unit, false) {
                assert_eq!(odd.frames(&word), Ok(f), "{word}");
            }
        }
    }
    assert_eq!(
        odd.write(4096, Unit::Seconds, false).as_deref(),
        Some("14.125s")
    );
}

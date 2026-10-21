use rewire_core::{CityEvent, MissionVerdict, check_mission, parse_city_output};

fn green(i: u32) -> CityEvent {
    CityEvent::Green { i, delay: 0 }
}

#[test]
fn parse_city_strips_lines_and_keeps_order() {
    let out = "hello\n@city {\"op\":\"green\",\"i\":0,\"delay\":2}\r\nmid\r\n@city {\"op\":\"lamp\",\"i\":1,\"on\":true}\n@city {\"op\":\"say\",\"text\":\"hi \\\"you\\\"\"}\nend";
    let (rest, ev) = parse_city_output(out);
    assert_eq!(rest, "hello\nmid\r\nend");
    assert_eq!(
        ev,
        vec![
            CityEvent::Green { i: 0, delay: 2 },
            CityEvent::Lamp { i: 1, on: true },
            CityEvent::Say {
                text: "hi \"you\"".into()
            },
        ]
    );
}

#[test]
fn parse_city_ignores_malformed_line() {
    let out = "@city {not json}\n@city {\"op\":\"nope\"}\n@city\n @city {\"op\":\"lamp\",\"i\":1,\"on\":true}\n@city {\"op\":\"lamp\",\"i\":1,\"on\":false}\n";
    let (rest, ev) = parse_city_output(out);
    assert_eq!(ev, vec![CityEvent::Lamp { i: 1, on: false }]);
    assert_eq!(
        rest,
        "@city {not json}\n@city {\"op\":\"nope\"}\n@city\n @city {\"op\":\"lamp\",\"i\":1,\"on\":true}\n"
    );
}

#[test]
fn parse_city_real_stub_format() {
    let (rest, ev) = parse_city_output("@city {\"op\":\"green\",\"i\":0,\"delay\":0}\n");
    assert_eq!(rest, "");
    assert_eq!(ev, vec![green(0)]);
}

#[test]
fn city_event_serde_shape() {
    let v = serde_json::to_value(CityEvent::Lamp { i: 2, on: true }).unwrap();
    assert_eq!(v, serde_json::json!({"op":"lamp","i":2,"on":true}));
}

#[test]
fn mission_pass() {
    assert_eq!(
        check_mission(&[green(0), green(1)], &[green(0), green(1)]),
        MissionVerdict::Pass
    );
    assert_eq!(check_mission(&[], &[]), MissionVerdict::Pass);
}

#[test]
fn mission_first_mismatch_index() {
    let e = [green(0), green(1), green(2), green(3)];
    let a = [
        green(0),
        green(1),
        green(2),
        CityEvent::Say { text: "x".into() },
    ];
    assert_eq!(
        check_mission(&e, &a),
        MissionVerdict::WrongEvents { first_mismatch: 3 }
    );
    // prefix cases
    assert_eq!(
        check_mission(&e, &e[..2]),
        MissionVerdict::WrongEvents { first_mismatch: 2 }
    );
    assert_eq!(
        check_mission(&e[..2], &e),
        MissionVerdict::WrongEvents { first_mismatch: 2 }
    );
    assert_eq!(
        check_mission(&[], &[green(0)]),
        MissionVerdict::WrongEvents { first_mismatch: 0 }
    );
}

#[test]
fn mission_no_events() {
    assert_eq!(check_mission(&[green(0)], &[]), MissionVerdict::NoEvents);
}

#[test]
fn verdict_serde_shape() {
    let v = serde_json::to_value(MissionVerdict::WrongEvents { first_mismatch: 3 }).unwrap();
    assert_eq!(
        v,
        serde_json::json!({"kind":"wrong_events","first_mismatch":3})
    );
    assert_eq!(
        serde_json::to_value(MissionVerdict::NoEvents).unwrap(),
        serde_json::json!({"kind":"no_events"})
    );
    assert_eq!(
        serde_json::to_value(MissionVerdict::Pass).unwrap(),
        serde_json::json!({"kind":"pass"})
    );
}

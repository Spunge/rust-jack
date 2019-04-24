
use jack_sys as j;

use Client;
use ClientOptions;
use Position;

fn open_test_client(name: &str) -> Client {
    Client::new(name, ClientOptions::NO_START_SERVER).unwrap().0
}

#[test]
fn client_can_query_transport() {
    let c = open_test_client("cp_can_query_transport");
    let (state, pos) = c.transport_query();
    assert_eq!(pos.bar, 0);
    assert_eq!(state, 0);
}

#[test]
fn client_can_reposition_transport() {
    let c = open_test_client("cp_can_reposition_transport");

    let pos = Position::default();
    c.transport_reposition(pos);
    let (_, pos) = c.transport_query();
    assert_eq!(pos.bar, 0);

    let mut pos = Position::default();
    pos.valid = j::JackPositionBBT;
    pos.beats_per_minute = 120.0;
    pos.beats_per_bar = 4.0;
    pos.beat_type = 4.0;
    pos.beat = 1;

    c.transport_reposition(pos);
    let (_, pos) = c.transport_query();
    assert_eq!(pos.beat, 1);
}


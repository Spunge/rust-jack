
use Client;
use ClientOptions;

fn open_test_client(name: &str) -> Client {
    Client::new(name, ClientOptions::NO_START_SERVER).unwrap().0
}

#[test]
fn client_can_query_transport() {
    let c = open_test_client("cp_can_query_transport");
    let (state, pos) = c.transport_query();
    println!("{:?} {:?} {:?}", pos.bar, pos.beat, pos.tick);
    println!("{:?}", state);
}

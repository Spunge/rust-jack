use std::{mem, ptr, thread, time};
use std::sync::Mutex;

use super::*;
use AudioIn;
use Client;
use Control;
use Frames;
use LatencyType;
use NotificationHandler;
use TimebaseHandler;
use PortId;
use ProcessHandler;
use TransportState;
use Position;

#[derive(Debug, Default)]
pub struct Counter {
    pub process_return_val: Control,
    pub induce_xruns: bool,
    pub thread_init_count: Mutex<usize>,
    pub frames_processed: usize,
    pub buffer_size_change_history: Vec<Frames>,
    pub registered_client_history: Vec<String>,
    pub unregistered_client_history: Vec<String>,
    pub port_register_history: Vec<PortId>,
    pub port_unregister_history: Vec<PortId>,
    pub xruns_count: usize,
    pub last_frame_time: Frames,
    pub frames_since_cycle_start: Frames,
}

impl NotificationHandler for Counter {
    fn thread_init(&self, _: &Client) {
        *self.thread_init_count.lock().unwrap() += 1;
    }

    fn buffer_size(&mut self, _: &Client, size: Frames) -> Control {
        self.buffer_size_change_history.push(size);
        Control::Continue
    }

    fn client_registration(&mut self, _: &Client, name: &str, is_registered: bool) {
        match is_registered {
            true => self.registered_client_history.push(name.to_string()),
            false => self.unregistered_client_history.push(name.to_string()),
        }
    }

    fn port_registration(&mut self, _: &Client, pid: PortId, is_registered: bool) {
        match is_registered {
            true => self.port_register_history.push(pid),
            false => self.port_unregister_history.push(pid),
        }
    }

    fn xrun(&mut self, _: &Client) -> Control {
        self.xruns_count += 1;
        Control::Continue
    }
}

impl ProcessHandler for Counter {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> Control {
        self.frames_processed += ps.n_frames() as usize;
        self.last_frame_time = ps.last_frame_time();
        self.frames_since_cycle_start = ps.frames_since_cycle_start();
        let _cycle_times = ps.cycle_times();
        if self.induce_xruns {
            thread::sleep(time::Duration::from_millis(400));
        }
        Control::Continue
    }
}

impl TimebaseHandler for Counter {
    fn timebase(&mut self, _: &Client, _state: TransportState, _n_frames: Frames, _pos: *mut Position, _is_new_pos: bool) {
    }
}

fn open_test_client(name: &str) -> Client {
    Client::new(name, ClientOptions::NO_START_SERVER).unwrap().0
}

fn active_test_client(name: &str) -> (AsyncClient<Counter, Counter, Counter>) {
    let c = open_test_client(name);
    let ac = c.activate_async(Counter::default(), Counter::default(), Counter::default())
        .unwrap();
    ac
}

#[test]
fn client_cback_has_proper_default_callbacks() {
    // defaults shouldn't care about these params
    let wc = unsafe { Client::from_raw(ptr::null_mut()) };
    let ps = unsafe { ProcessScope::from_raw(0, ptr::null_mut()) };
    let mut h = ();

    // check each callbacks
    assert_eq!(h.thread_init(&wc), ());
    assert_eq!(h.shutdown(client_status::ClientStatus::empty(), "mock"), ());
    assert_eq!(h.process(&wc, &ps), Control::Continue);
    assert_eq!(h.freewheel(&wc, true), ());
    assert_eq!(h.freewheel(&wc, false), ());
    assert_eq!(h.buffer_size(&wc, 0), Control::Continue);
    assert_eq!(h.sample_rate(&wc, 0), Control::Continue);
    assert_eq!(h.client_registration(&wc, "mock", true), ());
    assert_eq!(h.client_registration(&wc, "mock", false), ());
    assert_eq!(h.port_registration(&wc, 0, true), ());
    assert_eq!(h.port_registration(&wc, 0, false), ());
    assert_eq!(
        h.port_rename(&wc, 0, "old_mock", "new_mock"),
        Control::Continue
    );
    assert_eq!(h.ports_connected(&wc, 0, 1, true), ());
    assert_eq!(h.ports_connected(&wc, 2, 3, false), ());
    assert_eq!(h.graph_reorder(&wc), Control::Continue);
    assert_eq!(h.xrun(&wc), Control::Continue);
    assert_eq!(h.latency(&wc, LatencyType::Capture), ());
    assert_eq!(h.latency(&wc, LatencyType::Playback), ());

    mem::forget(wc);
    mem::forget(ps);
}

#[test]
fn client_cback_calls_thread_init() {
    let ac = active_test_client("client_cback_calls_thread_init");
    let counter = ac.deactivate().unwrap().1;
    // IDK why this isn't 1, even with a single thread.
    assert!(*counter.thread_init_count.lock().unwrap() > 0);
}

#[test]
fn client_cback_calls_process() {
    let ac = active_test_client("client_cback_calls_process");
    let counter = ac.deactivate().unwrap().2;
    assert!(counter.frames_processed > 0);
    assert!(counter.last_frame_time > 0);
    assert!(counter.frames_since_cycle_start > 0);
}

#[test]
fn client_cback_calls_buffer_size() {
    let ac = active_test_client("client_cback_calls_process");
    let initial = ac.as_client().buffer_size();
    let second = initial / 2;
    let third = second / 2;
    ac.as_client().set_buffer_size(second).unwrap();
    ac.as_client().set_buffer_size(third).unwrap();
    ac.as_client().set_buffer_size(initial).unwrap();
    let counter = ac.deactivate().unwrap().1;
    let mut history_iter = counter.buffer_size_change_history.iter().cloned();
    assert_eq!(history_iter.find(|&s| s == initial), Some(initial));
    assert_eq!(history_iter.find(|&s| s == second), Some(second));
    assert_eq!(history_iter.find(|&s| s == third), Some(third));
    assert_eq!(history_iter.find(|&s| s == initial), Some(initial));
}

#[test]
fn client_cback_calls_after_client_registered() {
    let ac = active_test_client("client_cback_cacr");
    let _other_client = open_test_client("client_cback_cacr_other");
    let counter = ac.deactivate().unwrap().1;
    assert!(
        counter
            .registered_client_history
            .contains(&"client_cback_cacr_other".to_string(),)
    );
    assert!(!counter
        .unregistered_client_history
        .contains(&"client_cback_cacr_other".to_string(),));
}

#[test]
fn client_cback_calls_after_client_unregistered() {
    let ac = active_test_client("client_cback_cacu");
    let other_client = open_test_client("client_cback_cacu_other");
    drop(other_client);
    let counter = ac.deactivate().unwrap().1;
    assert!(
        counter
            .registered_client_history
            .contains(&"client_cback_cacu_other".to_string(),)
    );
    assert!(
        counter
            .unregistered_client_history
            .contains(&"client_cback_cacu_other".to_string(),)
    );
}

#[test]
fn client_cback_reports_xruns() {
    let c = open_test_client("client_cback_reports_xruns");
    let mut counter = Counter::default();
    counter.induce_xruns = true;
    let ac = c.activate_async(Counter::default(), counter, Counter::default()).unwrap();
    let counter = ac.deactivate().unwrap().1;
    assert!(counter.xruns_count > 0, "No xruns encountered.");
}

#[test]
fn client_cback_calls_port_registered() {
    let ac = active_test_client("client_cback_cpr");
    let _pa = ac.as_client()
        .register_port("pa", AudioIn::default())
        .unwrap();
    let _pb = ac.as_client()
        .register_port("pb", AudioIn::default())
        .unwrap();
    let counter = ac.deactivate().unwrap().1;
    assert_eq!(
        counter.port_register_history.len(),
        2,
        "Did not detect port registrations."
    );
    assert!(
        counter.port_unregister_history.is_empty(),
        "Detected false port deregistrations."
    );
}

#[test]
fn client_cback_calls_port_unregistered() {
    let ac = active_test_client("client_cback_cpr");
    let pa = ac.as_client()
        .register_port("pa", AudioIn::default())
        .unwrap();
    let pb = ac.as_client()
        .register_port("pb", AudioIn::default())
        .unwrap();
    ac.as_client().unregister_port(pa).unwrap();
    ac.as_client().unregister_port(pb).unwrap();
    let counter = ac.deactivate().unwrap().1;
    assert!(
        counter.port_register_history.len() >= 2,
        "Did not detect port registrations."
    );
    assert!(
        counter.port_unregister_history.len() >= 2,
        "Did not detect port deregistrations."
    );
}

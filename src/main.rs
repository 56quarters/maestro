extern crate crossbeam_channel;
extern crate libc;
extern crate signal_hook;

use crossbeam_channel::Receiver;
use libc::c_int;
use signal_hook::iterator::Signals;
use std::cell::Cell;
use std::env;
use std::process::{self, Command};
use std::sync::{Arc, Mutex};
use std::thread;

struct ChildPid {
    pid: Mutex<Cell<Option<i32>>>,
}

impl ChildPid {
    fn get_pid(&self) -> Option<i32> {
        let cell = self.pid.lock().unwrap();
        cell.get()
    }

    fn set_pid(&self, pid: i32) {
        let cell = self.pid.lock().unwrap();
        cell.set(Some(pid))
    }
}

impl From<i32> for ChildPid {
    fn from(pid: i32) -> Self {
        ChildPid { pid: Mutex::new(Cell::new(Some(pid))) }
    }
}

impl Default for ChildPid {
    fn default() -> Self {
        ChildPid { pid: Mutex::new(Cell::new(None)) }
    }
}

fn receive_signals(signums: &[c_int]) -> Receiver<c_int> {
    let (s, r) = crossbeam_channel::unbounded();
    let signals = Signals::new(signums).unwrap();

    thread::spawn(move || {
        for signal in signals.forever() {
            s.send(signal);
        }
    });

    r
}

fn handle_signals(child_pid: Arc<ChildPid>, receiver: Receiver<i32>) {
    thread::spawn(move || {
        for sig in receiver {
            let pid = child_pid.get_pid();

            if let Some(p) = pid {
                println!("Sending signal {:?} to {:?}", sig, p);
                unsafe { libc::kill(p, sig as c_int) };
            }
        }
    });
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Need at least a command to execute!");
        process::exit(1);
    }

    let receiver = receive_signals(&[
        signal_hook::SIGABRT,
        signal_hook::SIGALRM,
        signal_hook::SIGBUS,
        signal_hook::SIGHUP,
        signal_hook::SIGINT,
        signal_hook::SIGQUIT,
        signal_hook::SIGTERM,
        signal_hook::SIGUSR1,
        signal_hook::SIGUSR2,
    ]);

    let pid = Arc::new(ChildPid::default());
    let our_pid = Arc::clone(&pid);

    handle_signals(our_pid, receiver);

    let mut child = Command::new(&args[1])
        .args(args[2..].iter())
        .spawn()
        .unwrap();

    pid.set_pid(child.id() as i32);

    println!("My pid: {} - child: {}", process::id(), child.id());
    child.wait().unwrap();
}

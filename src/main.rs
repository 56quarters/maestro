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

fn handle_signals(pid: Arc<Mutex<Cell<Option<i32>>>>, receiver: Receiver<i32>) {
    thread::spawn(move || {
        for sig in receiver {
            let child_pid = pid.lock().unwrap().get();

            if let Some(p) = child_pid {
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

    let pid = Arc::new(Mutex::new(Cell::new(None)));
    let our_pid = Arc::clone(&pid);

    handle_signals(our_pid, receiver);

    let mut child = Command::new(&args[1])
        .args(args[2..].iter())
        .spawn()
        .unwrap();

    pid.lock().unwrap().set(Some(child.id() as i32));
    child.wait().unwrap();
}

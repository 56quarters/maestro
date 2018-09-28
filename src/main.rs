extern crate crossbeam_channel;
extern crate libc;
extern crate signal_hook;

use crossbeam_channel::{Receiver, Sender};
use libc::c_int;
use signal_hook::iterator::Signals;
use std::cell::Cell;
use std::env;
use std::process::{self, Command};
use std::sync::{Arc, Mutex};
use std::thread;

fn launch_signals(signals: &[c_int]) -> Receiver<c_int> {
    let (s, r) = crossbeam_channel::bounded(100);
    let signals = Signals::new(signals).unwrap();

    thread::spawn(move || {
        for signal in signals.forever() {
            s.send(signal);
        }
    });

    r
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Need at least a command to execute!");
        process::exit(1);
    }

    let receiver = launch_signals(&[
        signal_hook::SIGHUP,
        signal_hook::SIGTERM,
        signal_hook::SIGINT,
        signal_hook::SIGQUIT,
    ]);

    let pid = Arc::new(Mutex::new(Cell::new(0)));
    let our_pid = Arc::clone(&pid);

    thread::spawn(move || {
        for i in receiver {
            let child_pid = our_pid.lock().unwrap().get();

            if child_pid != 0 {
                println!("Sending signal {:?} to {:?}", i, child_pid);

                unsafe {
                    libc::kill(child_pid, i as c_int);
                };
            } else {
                println!("Invalid pid {:?}", child_pid);
            }
        }
    });

    let mut child = Command::new(&args[1])
        .args(args[2..].iter())
        .spawn()
        .unwrap();

    pid.lock().unwrap().set(child.id() as i32);
    child.wait().unwrap();
}

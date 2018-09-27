extern crate crossbeam_channel;
extern crate libc;
extern crate signal_hook;

use crossbeam_channel::{Receiver, Sender};
use libc::c_int;
use signal_hook::iterator::Signals;
use std::env;
use std::process::{self, Command};
use std::thread;

fn notify(signals: &[c_int]) -> Receiver<c_int> {
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

    let receiver = notify(&[
        signal_hook::SIGHUP,
        signal_hook::SIGTERM,
        signal_hook::SIGINT,
        signal_hook::SIGQUIT,
    ]);

    let mut child = Command::new(&args[1])
        .args(args[2..].iter())
        .spawn()
        .unwrap();

    child.wait().unwrap();

    for i in receiver {
        println!("Got signal {:?}", i);
        if signal_hook::SIGTERM == i {
            break;
        }
    }
}

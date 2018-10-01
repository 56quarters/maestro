extern crate crossbeam_channel;
extern crate libc;
extern crate nix;
extern crate signal_hook;

use crossbeam_channel::Receiver;
use libc::c_int;
use nix::sys::signal::{SigSet, Signal};
use signal_hook::iterator::Signals;
use std::cell::Cell;
use std::env;
use std::process::{self, Command};
use std::ptr;
use std::sync::{Arc, Mutex};
use std::thread;

const CHANNEL_CAP: usize = 32;

///
///
///
const SIGNALS_TO_HANDLE: &[c_int] = &[
    signal_hook::SIGABRT,
    signal_hook::SIGALRM,
    signal_hook::SIGBUS,
    signal_hook::SIGCHLD,
    signal_hook::SIGCONT,
    signal_hook::SIGHUP,
    signal_hook::SIGINT,
    signal_hook::SIGIO,
    signal_hook::SIGPIPE,
    signal_hook::SIGPROF,
    signal_hook::SIGQUIT,
    signal_hook::SIGSYS,
    signal_hook::SIGTERM,
    signal_hook::SIGTRAP,
    signal_hook::SIGUSR1,
    signal_hook::SIGUSR2,
    signal_hook::SIGWINCH,
];

///
///
///
struct ThreadMasker {
    mask: SigSet,
}

impl ThreadMasker {
    fn new(allowed: &[c_int]) -> Self {
        // Start from an empty set of signals and only add the ones that we expect
        // to handle and hence need to mask from all threads that *aren't* specifically
        // for handling signals.
        let mut mask = SigSet::empty();

        for sig in allowed {
            mask.add(Signal::from_c_int(*sig).unwrap());
        }

        ThreadMasker { mask }
    }

    ///
    ///
    ///
    fn allow_for_thread(&self) {
        self.mask.thread_unblock().unwrap();
    }

    ///
    ///
    ///
    fn block_for_thread(&self) {
        self.mask.thread_block().unwrap();
    }

    fn print_blocked_for_thread(&self) {
        let t = SigSet::thread_get_mask().unwrap();

        let mut blocked = vec![];
        for sig in Signal::iterator() {
            if t.contains(sig) {
                blocked.push(sig)
            }
        }

        eprintln!("{:?} blocked: {:?}", thread::current().id(), blocked);
    }
}

///
///
///
#[derive(Debug)]
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
        ChildPid {
            pid: Mutex::new(Cell::new(Some(pid))),
        }
    }
}

impl Default for ChildPid {
    fn default() -> Self {
        ChildPid {
            pid: Mutex::new(Cell::new(None)),
        }
    }
}

///
///
///
struct SignalCatcher {
    signals: Signals,
    masker: ThreadMasker,
}

impl SignalCatcher {
    fn new(allowed: &[c_int]) -> Self {
        SignalCatcher {
            signals: Signals::new(allowed).unwrap(),
            masker: ThreadMasker::new(allowed),
        }
    }

    ///
    ///
    ///
    fn launch(self) -> (thread::JoinHandle<()>, Receiver<Signal>) {
        let (send, recv) = crossbeam_channel::bounded(CHANNEL_CAP);
        let handle = thread::spawn(move || {
            self.masker.allow_for_thread();

            for sig in self.signals.forever() {
                send.send(Signal::from_c_int(sig).unwrap());
            }
        });

        (handle, recv)
    }
}

///
///
///
struct SignalHandler {
    receiver: Receiver<Signal>,
    child: Arc<ChildPid>,
    masker: ThreadMasker,
}

impl SignalHandler {
    fn new(receiver: Receiver<Signal>, child: Arc<ChildPid>, allowed: &[c_int]) -> Self {
        SignalHandler {
            receiver,
            child,
            masker: ThreadMasker::new(allowed),
        }
    }

    fn wait_child() {
        loop {
            let res = unsafe { libc::waitpid(-1, ptr::null_mut(), libc::WNOHANG) };
            if res <= 0 {
                eprintln!("No children have changed state yet: {}", res);
                break;
            } else {
                eprintln!("Pid of child: {}", res);
            }
        }
    }

    fn propagate(pid: i32, sig: Signal) {
        eprintln!("{:?} sending {:?} to {:?}", thread::current().id(), sig, pid);
        unsafe { libc::kill(pid, sig as c_int) };
    }

    ///
    ///
    ///
    fn launch(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            self.masker.block_for_thread();

            for sig in self.receiver {
                let pid = self.child.get_pid();

                if let Some(p) = pid {
                    if sig == Signal::SIGCHLD {
                        Self::wait_child();
                    }

                    Self::propagate(p, sig);
                }
            }
        })
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Need at least a command to execute!");
        process::exit(1);
    }

    let masker = ThreadMasker::new(SIGNALS_TO_HANDLE);
    masker.block_for_thread();

    let catcher = SignalCatcher::new(SIGNALS_TO_HANDLE);
    let (t1, receiver) = catcher.launch();

    let pid = Arc::new(ChildPid::default());
    let pid_clone = Arc::clone(&pid);

    let handler = SignalHandler::new(receiver, pid_clone, SIGNALS_TO_HANDLE);
    let t2 = handler.launch();

    let mut child = Command::new(&args[1])
        .args(args[2..].iter())
        .spawn()
        .unwrap();

    pid.set_pid(child.id() as i32);
    println!("My pid: {} - child: {}", process::id(), child.id());

    match child.wait() {
        Err(e) => eprintln!("error waiting for child: {}", e),
        _ => (),
    }

    t1.join().unwrap();
    t2.join().unwrap();
}

use misc::panic::PanicHandler;
use std::{process, time::Duration};

fn main() {
    PanicHandler::new()
        .add_mail("wu_bin3@dahuatech.com")
        .enable_restart()
        .setup();

    let pid = process::id();
    println!("pid is {pid}");
    std::thread::spawn(move || {
        if pid % 3 == 0 {
            panic!("test panic here")
        }
    });

    let mut n = 0;
    loop {
        std::thread::sleep(Duration::from_secs(2));
        println!("test running ... {n}");
        n += 1;
    }
}

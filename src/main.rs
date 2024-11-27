use std::time::Duration;
use infinity_threads::InfinityThread;

fn main() {
    let mut thread = InfinityThread::new();
    let mut message: String = String::new();
    let mut counter = 0;

    thread.start(move || {
        message.push_str("Hello, ");
        println!("Run Counter: {}", counter);
        counter += 1;
        if counter > 10 {
            counter = 0;
            panic!("Counter is greater than 10, sending artificial panic");
        }
    }, Duration::from_millis(1000));

    let mut counter_2 = 0;

    println!("Starting counter 2...");
    loop {
        println!("Counter 2: {}", counter_2);
        std::thread::sleep(Duration::from_secs(1));
        counter_2 += 1;
        if counter_2 > 20 {
            println!("SENDING STOP REQUEST");
            thread.stop();
            break;
        }
    }

    // Keep the main thread alive for demonstration.
    std::thread::sleep(Duration::from_secs(90));
}
use std::time::{Duration, Instant};

pub fn start_timer(seconds: u64, cx: &mut App) -> [u64; 2] {
    let start = Instant::now();
    let duration = Duration::from_secs(seconds + 1);
    let mut prev_output = 0;
    loop {
        let elapsed = Instant::now() - start;
        let remaining = if duration > elapsed {
            duration - elapsed
        } else {
            Duration::ZERO
        };
        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;

        if remaining.is_zero() {
            break;
        }

        if prev_output != secs {
            println!("{:02}:{:02}", mins, secs);
            prev_output = secs;
        }
    }
    println!("Completed countdown!");
}

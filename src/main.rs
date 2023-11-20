use nfl_schedule_simulator::*;
use std::io::Write;
use std::time::Instant;

fn main() {
    let season_year: i32 = 2023;
    let mut season: Season = Season::new_from_year(season_year);

    let now: Instant = Instant::now();
    for i in 0..10000 {
        season.run_simulation();
        print!("\r{i}");
        std::io::stdout()
            .flush()
            .expect("stdout could not be flushed");
    }
    let elapsed: std::time::Duration = now.elapsed();
    println!("\n{:.2?}", elapsed);
}

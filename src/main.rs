use nfl_schedule_simulator::*;
use std::io::Write;
use std::time::Instant;

fn main() {
    let season_year: i32 = 2018;
    let mut season: Season = Season::new_from_year(season_year);

    season.run_simulation();

    // println!("{:#?}", season.current_simulation_result.team_records);
    // 26 is NYJ
    // println!(
    //     "{:#?}",
    //     season.current_simulation_result.team_records.get(&26)
    // );

    // run_timed_simulations(season_year, 10000);
}

#[allow(dead_code)]
fn run_timed_simulations(season_year: i32, sims: i32) {
    let mut season: Season = Season::new_from_year(season_year);

    let now: Instant = Instant::now();
    for i in 0..sims {
        season.run_simulation();
        print!("\r{i}");
        std::io::stdout()
            .flush()
            .expect("stdout could not be flushed");
    }
    let elapsed: std::time::Duration = now.elapsed();
    println!("\n{:.2?}", elapsed);
}

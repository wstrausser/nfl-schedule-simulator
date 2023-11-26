use nfl_schedule_simulator::*;
use std::io::Write;
use std::time::Instant;

mod migrations;

fn main() {
    migrations::rebuild();
    // migrations::destroy();
    let season_year: i32 = 2023;
    let mut season: Season = Season::new_from_year(season_year);

    // season.set_simulation_id(1000);

    // println!("{:#?}", season);

    // season.run_simulation();

    // println!("{:#?}", season.current_simulation_result.team_records);
    // 26 is NYJ
    // println!(
    //     "{:#?}",
    //     season.current_simulation_result.team_records.get(&26)
    // );

    season.run_all_game_simulations(100000);
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
    println!("{:#?}", season.overall_results);
    println!("\n{:.2?}", elapsed);
}

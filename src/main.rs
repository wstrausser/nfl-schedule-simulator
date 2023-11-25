use kdam::tqdm;
use nfl_schedule_simulator::*;
use std::io::Write;
use std::time::Instant;

fn main() {
    let season_year: i32 = 2023;
    // let mut season: Season = Season::new_from_year(season_year);

    // season.run_simulation();

    // println!("{:#?}", season.current_simulation_result.team_records);
    // 26 is NYJ
    // println!(
    //     "{:#?}",
    //     season.current_simulation_result.team_records.get(&26)
    // );

    run_all_game_simulations(season_year, 1000);
}

#[allow(dead_code)]
fn run_all_game_simulations(season_year: i32, sims: u64) {
    let mut season: Season = Season::new_from_year(season_year);
    let games = season.actual_games.clone();
    for (game_id, _) in tqdm!(games.iter()) {
        let actual_game: Game = season.actual_games.get(game_id).unwrap().clone();
        match actual_game.game_result {
            Some(_) => {}
            None => {
                season.simulate_for_game(game_id.clone(), GameResult::HomeWin, sims);
                season.simulate_for_game(game_id.clone(), GameResult::AwayWin, sims);
                season.simulate_for_game(game_id.clone(), GameResult::Tie, sims);
            }
        }
    }

    println!("{:#?}", season.overall_results);
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

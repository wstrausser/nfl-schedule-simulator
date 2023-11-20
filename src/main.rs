use nfl_schedule_simulator::*;

fn main() {
    let season_year: i32 = 2023;
    let mut season: Season = Season::new_from_year(season_year);

    season.run_simulation();

    println!("{:#?}", season.simulated_games);
}

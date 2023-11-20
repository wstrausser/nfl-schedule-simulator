use nfl_schedule_simulator::*;

fn main() {
    let season_year: i32 = 2023;
    let season: Season = Season::new_from_year(season_year);

    let team = season.teams.get(&13).unwrap();
    let game = season.games.get(&3797).unwrap();

    let is_same_team = { &game.home_team == team };

    println!("{:?}", is_same_team);
}

use nfl_schedule_simulator::*;
use std::collections::HashMap;

fn main() {
    let season: i32 = 2023;
    let teams = get_season_teams(2023);
    let games: HashMap<i32, Game> = get_season_games(season, &teams);

    let game: &Game = games.get(&3924).unwrap();

    let division_game: bool = {
        game.home_team.division == game.away_team.division
    };

    println!("{:?}", division_game);
}

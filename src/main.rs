use nfl_schedule_simulator::*;

fn main() {
    let teams = get_season_teams(2023);

    println!("{:?}", teams);
}

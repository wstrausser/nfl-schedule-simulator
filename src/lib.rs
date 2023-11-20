use dotenv::dotenv;
use postgres::{Client, Error, NoTls, Row};
use std::collections::HashMap;
use std::env::var;

#[derive(Debug)]
pub enum GameResult {
    HomeWin,
    AwayWin,
    Tie,
}

#[derive(Debug)]
pub struct Team {
    pub team_id: i32,
    pub abbreviation: String,
    pub name: String,
    pub conference: String,
    pub division: String,
}

#[derive(Debug)]
pub struct Game<'a> {
    pub game_id: i32,
    pub season: i32,
    pub week: i32,
    pub home_team: &'a Team,
    pub away_team: &'a Team,
    pub game_result: Option<GameResult>,
}

pub fn get_season_teams(season: i32) -> HashMap<i32, Team> {
    let query: String = format!(
        "
        SELECT
            team_id,
            abbreviation,
            name,
            conference,
            division
        FROM nfl.teams
        WHERE team_id in (
            SELECT DISTINCT home_team_id
            FROM nfl.games
            WHERE season={season}
        )
        ORDER BY division, abbreviation;
    "
    );

    let results: Vec<Row> = run_query(query);

    let mut teams: HashMap<i32, Team> = HashMap::new();
    for row in results {
        let team: Team = Team {
            team_id: row.get(0),
            abbreviation: row.get(1),
            name: row.get(2),
            conference: row.get(3),
            division: row.get(4),
        };
        teams.insert(team.team_id, team);
    }
    teams
}

pub fn get_season_games<'a>(season: i32, teams: &'a HashMap<i32, Team>) -> HashMap<i32, Game<'a>> {
    let query: String = format!(
        "
        SELECT
            game_id,
            week,
            home_team_id,
            away_team_id,
            home_score,
            away_score
        FROM nfl.games
        WHERE season={season};
    "
    );

    let results: Vec<Row> = run_query(query);

    let mut games: HashMap<i32, Game> = HashMap::new();
    for row in results {
        let game_id: i32 = row.get(0);
        let week: i32 = row.get(1);
        let home_team_id: i32 = row.get(2);
        let away_team_id: i32 = row.get(3);
        let home_score: Option<i32> = row.get(4);
        let away_score: Option<i32> = row.get(5);

        let home_team: &Team = teams.get(&home_team_id).expect("Team does not exist");
        let away_team: &Team = teams.get(&away_team_id).expect("Team does not exist");

        let game_result: Option<GameResult> = {
            if home_score.is_none() && away_score.is_none() {
                None
            }
            else if home_score.unwrap() > away_score.unwrap() {
                Some(GameResult::HomeWin)
            }
            else if home_score.unwrap() < away_score.unwrap() {
                Some(GameResult::AwayWin)
            }
            else {
                Some(GameResult::Tie)
            }
        };

        let game: Game = Game {
            game_id: game_id,
            season: season,
            week: week,
            home_team: home_team,
            away_team: away_team,
            game_result: game_result,
        };
        games.insert(game_id, game);
    }
    games
}

fn get_variable(key: &str) -> String {
    match var(key) {
        Ok(val) => val,
        Err(err) => panic!("{}", err),
    }
}

fn get_conn_string() -> String {
    dotenv().ok();

    let pg_locn: String = get_variable("PG_LOCN");
    let pg_dtbs: String = get_variable("PG_DTBS");
    let pg_user: String = get_variable("PG_USER");
    let pg_pass: String = get_variable("PG_PASS");

    format!("postgres://{pg_user}:{pg_pass}@{pg_locn}/{pg_dtbs}")
}

fn connect() -> Client {
    let conn_string = get_conn_string();
    let mut client: Client = match Client::connect(&conn_string, NoTls) {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };
    client
}

fn run_query(query: String) -> Vec<Row> {
    let mut client: Client = connect();
    let results = match client.query(&query, &[]) {
        Ok(r) => r,
        Err(e) => panic!("{}", e),
    };
    results
}

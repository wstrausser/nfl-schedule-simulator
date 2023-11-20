use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
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
    pub game_id: u32,
    pub season: Season<'a>,
    pub week: u32,
    pub home_team: &'a Team,
    pub away_team: &'a Team,
    pub game_result: Option<GameResult>,
}

#[derive(Debug)]
pub struct Season<'a> {
    pub year: u32,
    pub games: Vec<Game<'a>>,
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

    let mut client: Client = match connect() {
        Ok(c) => c,
        Err(e) => panic!("{}", e),
    };

    let results = match client.query(&query, &[]) {
        Ok(r) => r,
        Err(e) => panic!("{}", e),
    };

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

fn connect() -> Result<Client, Error> {
    let conn_string = get_conn_string();
    Client::connect(&conn_string, NoTls)
}

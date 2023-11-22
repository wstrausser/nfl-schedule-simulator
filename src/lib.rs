use dotenv::dotenv;
use postgres::{Client, NoTls, Row};
use rand::Rng;
use std::collections::HashMap;
use std::env::var;

#[derive(Clone, Debug, PartialEq)]
pub struct Team {
    pub team_id: i32,
    pub abbreviation: String,
    pub name: String,
    pub conference: String,
    pub division: String,
}

impl Team {
    pub fn new_from_db_row(row: Row) -> Team {
        let team: Team = Team {
            team_id: row.get(0),
            abbreviation: row.get(1),
            name: row.get(2),
            conference: row.get(3),
            division: row.get(4),
        };
        team
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameResult {
    HomeWin,
    AwayWin,
    Tie,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub game_id: i32,
    pub season_year: i32,
    pub week: i32,
    pub division_game: bool,
    pub conference_game: bool,
    pub home_team: Team,
    pub away_team: Team,
    pub game_result: Option<GameResult>,
    pub is_simulated: bool,
}

impl Game {
    pub fn new_from_db_row(row: Row, teams: HashMap<i32, Team>) -> Game {
        let game_id: i32 = row.get(0);
        let season_year: i32 = row.get(1);
        let week: i32 = row.get(2);
        let home_team_id: i32 = row.get(3);
        let away_team_id: i32 = row.get(4);
        let home_score: Option<i32> = row.get(5);
        let away_score: Option<i32> = row.get(6);

        let home_team: Team = teams
            .get(&home_team_id)
            .expect("Team does not exist")
            .clone();
        let away_team: Team = teams
            .get(&away_team_id)
            .expect("Team does not exist")
            .clone();

        let division_game: bool = { home_team.division == away_team.division };
        let conference_game: bool = { home_team.conference == away_team.conference };

        let game_result: Option<GameResult> = {
            if home_score.is_none() && away_score.is_none() {
                None
            } else if home_score.unwrap() > away_score.unwrap() {
                Some(GameResult::HomeWin)
            } else if home_score.unwrap() < away_score.unwrap() {
                Some(GameResult::AwayWin)
            } else {
                Some(GameResult::Tie)
            }
        };

        let game: Game = Game {
            game_id,
            season_year,
            week,
            division_game,
            conference_game,
            home_team,
            away_team,
            game_result,
            is_simulated: false,
        };

        game
    }

    pub fn simulate_if_undecided(&mut self) {
        if self.game_result.is_none() {
            let tie_likelihood: f64 = 0.003421;

            let mut rng: rand::rngs::ThreadRng = rand::thread_rng();
            let tie_predictor: f64 = rng.gen();
            let win_predictor: f64 = rng.gen();

            if tie_predictor <= tie_likelihood {
                self.game_result = Some(GameResult::Tie);
            } else if win_predictor < 0.5 {
                self.game_result = Some(GameResult::HomeWin);
            } else if win_predictor >= 0.5 {
                self.game_result = Some(GameResult::AwayWin);
            };

            self.is_simulated = true;
        }
    }
}

#[derive(Debug)]
pub struct TeamRecord {
    pub overall_record: (u8, u8, u8),
    pub overall_percent: f64,
    pub conference_record: (u8, u8, u8),
    pub conference_percent: f64,
    pub division_record: (u8, u8, u8),
    pub division_percent: f64,
}

impl TeamRecord {
    fn new() -> TeamRecord {
        TeamRecord {
            overall_record: (0, 0, 0),
            overall_percent: 0.0,
            conference_record: (0, 0, 0),
            conference_percent: 0.0,
            division_record: (0, 0, 0),
            division_percent: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct CurrentSimulationResult {
    pub team_records: HashMap<i32, TeamRecord>,
    pub playoff_seeding: HashMap<u8, Vec<Team>>,
    pub division_winners: Vec<Team>,
    pub wildcard_teams: Vec<Team>,
    pub draft_order: Vec<Team>,
}

impl CurrentSimulationResult {
    fn new() -> CurrentSimulationResult {
        CurrentSimulationResult {
            team_records: HashMap::new(),
            playoff_seeding: HashMap::new(),
            division_winners: Vec::new(),
            wildcard_teams: Vec::new(),
            draft_order: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct SimulationResultLookup {
    pub game_id: i32,
    pub game_result: GameResult,
    pub team_id: i32,
}

#[derive(Debug)]
pub struct TeamSimulationResults {
    pub made_playoffs: i32,
    pub playoff_seedings: Vec<i32>,
    pub division_winner: i32,
    pub wildcard_team: i32,
    pub draft_picks: Vec<i32>,
}

#[derive(Debug)]
pub struct Season {
    pub season_year: i32,
    pub teams: HashMap<i32, Team>,
    pub conference_mapping: HashMap<String, Vec<i32>>,
    pub division_mapping: HashMap<String, Vec<i32>>,
    pub actual_games: HashMap<i32, Game>,
    pub current_simulated_games: HashMap<i32, Game>,
    pub current_simulation_result: CurrentSimulationResult,
    pub overall_results: HashMap<SimulationResultLookup, TeamSimulationResults>,
}

impl Season {
    pub fn new_from_year(season_year: i32) -> Season {
        let mut season: Season = Season {
            season_year,
            teams: HashMap::new(),
            conference_mapping: HashMap::new(),
            division_mapping: HashMap::new(),
            actual_games: HashMap::new(),
            current_simulated_games: HashMap::new(),
            current_simulation_result: CurrentSimulationResult::new(),
            overall_results: HashMap::new(),
        };

        season.load_teams();
        season.load_conference_division_mapping();
        season.load_games();
        season
    }

    pub fn run_simulation(&mut self) {
        self.current_simulation_result = CurrentSimulationResult::new();
        self.current_simulated_games = self.actual_games.clone();
        for game_item in self.current_simulated_games.iter_mut() {
            let game: &mut Game = game_item.1;
            game.simulate_if_undecided();
        }
        self.evaluate_simulation_results();
    }

    fn evaluate_simulation_results(&mut self) {
        self.populate_records();
        self.calculate_percentages();
        self.evaluate_divisions();
    }

    fn populate_records(&mut self) {
        for (team_id, _) in self.teams.iter() {
            self.current_simulation_result
                .team_records
                .insert(team_id.clone(), TeamRecord::new());
        }
        for (game_id, game) in self.current_simulated_games.iter() {
            let (winning_team, losing_team): (Option<i32>, Option<i32>) = {
                if game.game_result == Some(GameResult::HomeWin) {
                    (
                        Some(game.home_team.team_id.clone()),
                        Some(game.away_team.team_id.clone()),
                    )
                } else if game.game_result == Some(GameResult::AwayWin) {
                    (
                        Some(game.away_team.team_id.clone()),
                        Some(game.home_team.team_id.clone()),
                    )
                } else if game.game_result == Some(GameResult::Tie) {
                    (None, None)
                } else {
                    panic!("Game not simulated yet");
                }
            };

            match winning_team {
                Some(team_id) => {
                    let record = self
                        .current_simulation_result
                        .team_records
                        .get_mut(&team_id)
                        .unwrap();
                    record.overall_record.0 += 1;
                    if game.conference_game {
                        record.conference_record.0 += 1;
                    }
                    if game.division_game {
                        record.division_record.0 += 1;
                    }
                }
                None => {
                    let team_id = game.home_team.team_id;
                    let record = self
                        .current_simulation_result
                        .team_records
                        .get_mut(&team_id)
                        .unwrap();
                    record.overall_record.2 += 1;
                    if game.conference_game {
                        record.conference_record.2 += 1;
                    }
                    if game.division_game {
                        record.division_record.2 += 1;
                    }
                }
            };
            match losing_team {
                Some(team_id) => {
                    let record = self
                        .current_simulation_result
                        .team_records
                        .get_mut(&team_id)
                        .unwrap();
                    record.overall_record.1 += 1;
                    if game.conference_game {
                        record.conference_record.1 += 1;
                    }
                    if game.division_game {
                        record.division_record.1 += 1;
                    }
                }
                None => {
                    let team_id = game.away_team.team_id;
                    let record = self
                        .current_simulation_result
                        .team_records
                        .get_mut(&team_id)
                        .unwrap();
                    record.overall_record.2 += 1;
                    if game.conference_game {
                        record.conference_record.2 += 1;
                    }
                    if game.division_game {
                        record.division_record.2 += 1;
                    }
                }
            };
        }
    }

    fn calculate_percentages(&mut self) {
        fn calculate_from_tuple(record_tuple: (u8, u8, u8)) -> f64 {
            let (wins, losses, ties) = record_tuple;
            let computed_wins: f64 = f64::from(wins) + (f64::from(ties) / 2.0);

            computed_wins / (f64::from(wins + losses + ties))
        }
        for (team_id, record) in self.current_simulation_result.team_records.iter_mut() {
            record.overall_percent = calculate_from_tuple(record.overall_record);
            record.conference_percent = calculate_from_tuple(record.conference_record);
            record.division_percent = calculate_from_tuple(record.division_record);
        }
    }

    fn evaluate_divisions(&mut self) {
        for (division, team_ids) in self.division_mapping.iter() {}
    }

    fn load_teams(&mut self) {
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
                WHERE season={0}
            )
            ORDER BY division, abbreviation;
        ",
            self.season_year,
        );

        for row in run_query(query) {
            let team: Team = Team::new_from_db_row(row);
            self.teams.insert(team.team_id, team);
        }
    }

    fn load_conference_division_mapping(&mut self) {
        for (_, team) in self.teams.iter() {
            if !self.conference_mapping.contains_key(&team.conference) {
                self.conference_mapping
                    .insert(team.conference.clone(), Vec::new());
            }

            let conference_vector: &mut Vec<i32> =
                self.conference_mapping.get_mut(&team.conference).unwrap();
            conference_vector.push(team.team_id.clone());

            if !self.division_mapping.contains_key(&team.division) {
                self.division_mapping
                    .insert(team.division.clone(), Vec::new());
            }

            let division_vector: &mut Vec<i32> =
                self.division_mapping.get_mut(&team.division).unwrap();
            division_vector.push(team.team_id.clone());
        }
    }

    fn load_games(&mut self) {
        let query: String = format!(
            "
            SELECT
                game_id,
                season,
                week,
                home_team_id,
                away_team_id,
                home_score,
                away_score
            FROM nfl.games
            WHERE season={0};
        ",
            self.season_year,
        );

        let results: Vec<Row> = run_query(query);

        for row in results {
            let game: Game = Game::new_from_db_row(row, self.teams.clone());
            self.actual_games.insert(game.game_id.clone(), game);
        }
    }
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
    let client: Client = match Client::connect(&conn_string, NoTls) {
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

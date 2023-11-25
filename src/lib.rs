use dotenv::dotenv;
use kdam::tqdm;
use postgres::{Client, NoTls, Row};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Clone, Debug)]
pub struct TeamRecord {
    pub overall_record: (u8, u8, u8),
    pub overall_percent: u16,
    pub conference_record: (u8, u8, u8),
    pub conference_percent: u16,
    pub division_record: (u8, u8, u8),
    pub division_percent: u16,
}

impl TeamRecord {
    fn new() -> TeamRecord {
        TeamRecord {
            overall_record: (0, 0, 0),
            overall_percent: 0,
            conference_record: (0, 0, 0),
            conference_percent: 0,
            division_record: (0, 0, 0),
            division_percent: 0,
        }
    }
}

#[derive(Debug)]
pub struct CurrentSimulationResult {
    pub team_records: HashMap<i32, TeamRecord>,
    pub playoff_seeding: HashMap<u8, HashSet<i32>>,
    pub division_winners: HashSet<i32>,
    pub wildcard_teams: HashSet<i32>,
    pub draft_order: Vec<i32>,
}

impl CurrentSimulationResult {
    fn new() -> CurrentSimulationResult {
        CurrentSimulationResult {
            team_records: HashMap::new(),
            playoff_seeding: HashMap::new(),
            division_winners: HashSet::new(),
            wildcard_teams: HashSet::new(),
            draft_order: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SimulationResultLookup {
    pub game_id: i32,
    pub game_result: GameResult,
    pub team_id: i32,
}

#[derive(Debug)]
pub struct TeamSimulationResults {
    pub sims_run: u64,
    pub made_playoffs: i32,
    pub playoff_seedings: Vec<i32>,
    pub division_winner: i32,
    pub wildcard_team: i32,
    pub draft_picks: Vec<i32>,
}

impl TeamSimulationResults {
    fn new() -> TeamSimulationResults {
        TeamSimulationResults {
            sims_run: 0,
            made_playoffs: 0,
            playoff_seedings: Vec::new(),
            division_winner: 0,
            wildcard_team: 0,
            draft_picks: Vec::new(),
        }
    }

    fn new_with_sims(sims_run: u64) -> TeamSimulationResults {
        TeamSimulationResults {
            sims_run: sims_run,
            made_playoffs: 0,
            playoff_seedings: Vec::new(),
            division_winner: 0,
            wildcard_team: 0,
            draft_picks: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Season {
    pub season_year: i32,
    pub teams: HashMap<i32, Team>,
    pub conference_mapping: HashMap<String, Vec<i32>>,
    pub division_mapping: HashMap<String, Vec<i32>>,
    pub actual_games: HashMap<i32, Game>,
    pub current_simulation_game: Option<(i32, GameResult)>,
    pub current_simulation_base_games: HashMap<i32, Game>,
    pub current_simulation_games: HashMap<i32, Game>,
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
            current_simulation_game: None,
            current_simulation_base_games: HashMap::new(),
            current_simulation_games: HashMap::new(),
            current_simulation_result: CurrentSimulationResult::new(),
            overall_results: HashMap::new(),
        };

        season.load_teams();
        season.load_conference_division_mapping();
        season.load_games();
        season
    }

    pub fn simulate_for_game(&mut self, game_id: i32, game_result: GameResult, sims: u64) {
        self.current_simulation_game = Some((game_id.clone(), game_result.clone()));
        self.current_simulation_base_games = self.actual_games.clone();
        self.current_simulation_base_games
            .get_mut(&game_id)
            .unwrap()
            .game_result = Some(game_result.clone());

        for (team_id, _) in self.teams.iter() {
            let new_lookup = SimulationResultLookup {
                game_id: game_id.clone(),
                game_result: game_result.clone(),
                team_id: team_id.clone(),
            };
            self.overall_results.insert(
                new_lookup,
                TeamSimulationResults::new_with_sims(sims.clone()),
            );
        }

        for _ in 0..sims {
            self.run_simulation();
        }
    }

    pub fn run_simulation(&mut self) {
        self.current_simulation_result = CurrentSimulationResult::new();
        self.current_simulation_games = self.current_simulation_base_games.clone();
        for game_item in self.current_simulation_games.iter_mut() {
            let game: &mut Game = game_item.1;
            game.simulate_if_undecided();
        }
        self.evaluate_simulation_results();
    }

    fn evaluate_simulation_results(&mut self) {
        self.populate_records();
        self.calculate_percentages();
        self.evaluate_divisions();
        self.increment_overall_results();
    }

    fn populate_records(&mut self) {
        for (team_id, _) in self.teams.iter() {
            self.current_simulation_result
                .team_records
                .insert(team_id.clone(), TeamRecord::new());
        }
        for (game_id, game) in self.current_simulation_games.iter() {
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
        for (team_id, record) in self.current_simulation_result.team_records.iter_mut() {
            record.overall_percent = Self::calculate_percent_from_tuple(record.overall_record);
            record.conference_percent =
                Self::calculate_percent_from_tuple(record.conference_record);
            record.division_percent = Self::calculate_percent_from_tuple(record.division_record);
        }
    }

    fn calculate_percent_from_tuple(record_tuple: (u8, u8, u8)) -> u16 {
        let (wins, mut losses, mut ties) = record_tuple;
        let wins: u32 = u32::from(wins);
        let losses: u32 = u32::from(losses);
        let ties: u32 = u32::from(ties);
        let computed_wins: u32 = (wins * 1000) + ((ties * 1000) / 2);

        u16::try_from(computed_wins / (wins + losses + ties)).unwrap()
    }

    fn evaluate_divisions(&mut self) {
        for (division, team_ids) in self.division_mapping.iter_mut() {
            let mut working_vec: Vec<(i32, (u16, u16, u16))> = Vec::new();
            for team_id in team_ids {
                let team_record = self
                    .current_simulation_result
                    .team_records
                    .get(team_id)
                    .unwrap();
                let team_pcts = (
                    team_id.clone(),
                    (
                        team_record.overall_percent.clone(),
                        team_record.conference_percent.clone(),
                        team_record.division_percent.clone(),
                    ),
                );
                working_vec.push(team_pcts);
            }
            working_vec.sort_by_key(|t| t.1 .0);
            working_vec.reverse();

            let max_pct = working_vec.get(0).unwrap().1 .0;
            let mut tied_teams = HashSet::new();
            for (team_id, pcts) in &working_vec {
                if pcts.0 == max_pct {
                    tied_teams.insert(team_id.clone());
                } else {
                    break;
                }
            }

            if tied_teams.len() > 1 {
                tied_teams = Self::evaluate_head_to_head(
                    tied_teams.clone(),
                    self.current_simulation_games.clone(),
                );
            }
            if tied_teams.len() > 1 {
                tied_teams = Self::evaluate_division_records(
                    tied_teams.clone(),
                    self.current_simulation_result.team_records.clone(),
                );
            }
            if tied_teams.len() > 1 {
                tied_teams = Self::evaluate_common_games(
                    tied_teams.clone(),
                    self.current_simulation_games.clone(),
                );
            }
            if tied_teams.len() > 1 {
                tied_teams = Self::pick_random_team_from_tied(tied_teams.clone());
            }

            let tied_teams = Vec::from_iter(tied_teams);
            let division_winner: i32 = tied_teams.first().unwrap().clone();

            self.current_simulation_result
                .division_winners
                .insert(division_winner);
        }
    }

    fn evaluate_division_records(
        tied_teams: HashSet<i32>,
        records: HashMap<i32, TeamRecord>,
    ) -> HashSet<i32> {
        let mut working_vec: Vec<(i32, u16)> = Vec::new();
        for team_id in tied_teams.iter() {
            working_vec.push((
                team_id.clone(),
                records.get(&team_id).unwrap().division_percent.clone(),
            ));
        }
        working_vec.sort_by_key(|t| t.1);
        working_vec.reverse();

        let mut remaining_tied_teams: HashSet<i32> = HashSet::new();
        let max_pct = working_vec.get(0).unwrap().1;
        for (team_id, pct) in working_vec {
            if pct == max_pct {
                remaining_tied_teams.insert(team_id.clone());
            } else {
                break;
            }
        }
        remaining_tied_teams
    }

    fn evaluate_head_to_head(tied_teams: HashSet<i32>, games: HashMap<i32, Game>) -> HashSet<i32> {
        let mut records: HashMap<i32, (u8, u8, u8)> = HashMap::new();
        for team_id in &tied_teams {
            records.insert(team_id.clone(), (0, 0, 0));
        }
        for (_, game) in games.iter() {
            if tied_teams.contains(&game.home_team.team_id)
                && tied_teams.contains(&game.away_team.team_id)
            {
                match game.game_result {
                    Some(GameResult::HomeWin) => {
                        records.get_mut(&game.home_team.team_id).unwrap().0 += 1;
                        records.get_mut(&game.away_team.team_id).unwrap().1 += 1;
                    }
                    Some(GameResult::AwayWin) => {
                        records.get_mut(&game.home_team.team_id).unwrap().1 += 1;
                        records.get_mut(&game.away_team.team_id).unwrap().0 += 1;
                    }
                    Some(GameResult::Tie) => {
                        records.get_mut(&game.home_team.team_id).unwrap().2 += 1;
                        records.get_mut(&game.away_team.team_id).unwrap().2 += 1;
                    }
                    None => panic!("Game has no result"),
                }
            }
        }
        let mut working_vec: Vec<(i32, u16)> = Vec::new();
        for (team_id, record) in records {
            working_vec.push((team_id.clone(), Self::calculate_percent_from_tuple(record)));
        }
        working_vec.sort_by_key(|t| t.1);
        working_vec.reverse();

        let mut remaining_tied_teams: HashSet<i32> = HashSet::new();
        let max_pct = working_vec.get(0).unwrap().1;
        for (team_id, pct) in working_vec {
            if pct == max_pct {
                remaining_tied_teams.insert(team_id.clone());
            } else {
                break;
            }
        }

        remaining_tied_teams
    }

    fn evaluate_common_games(tied_teams: HashSet<i32>, games: HashMap<i32, Game>) -> HashSet<i32> {
        let mut records: HashMap<i32, (u8, u8, u8)> = HashMap::new();
        for team_id in &tied_teams {
            records.insert(team_id.clone(), (0, 0, 0));
        }

        let mut team_opponents: HashMap<i32, HashSet<i32>> = HashMap::new();
        for team_id in &tied_teams {
            team_opponents.insert(team_id.clone(), HashSet::new());
        }
        for (_, game) in games.iter() {
            if tied_teams.contains(&game.home_team.team_id) {
                team_opponents
                    .get_mut(&game.home_team.team_id)
                    .unwrap()
                    .insert(game.away_team.team_id.clone());
            } else if tied_teams.contains(&game.away_team.team_id) {
                team_opponents
                    .get_mut(&game.away_team.team_id)
                    .unwrap()
                    .insert(game.home_team.team_id.clone());
            }
        }

        let mut set_vec: Vec<HashSet<i32>> = Vec::new();
        for (_, team_opponents_set) in team_opponents.iter() {
            set_vec.push(team_opponents_set.clone());
        }
        let mut iter = set_vec.iter();
        let common_opponents = iter
            .next()
            .map(|set| {
                iter.fold(set.clone(), |set1, set2| {
                    set1.intersection(&set2).cloned().collect()
                })
            })
            .unwrap();

        for (_, game) in games.iter() {
            if tied_teams.contains(&game.home_team.team_id)
                && common_opponents.contains(&game.away_team.team_id)
            {
                match game.game_result {
                    Some(GameResult::HomeWin) => {
                        records.get_mut(&game.home_team.team_id).unwrap().0 += 1;
                    }
                    Some(GameResult::AwayWin) => {
                        records.get_mut(&game.home_team.team_id).unwrap().1 += 1;
                    }
                    Some(GameResult::Tie) => {
                        records.get_mut(&game.home_team.team_id).unwrap().2 += 1;
                    }
                    None => panic!("Game has no result"),
                }
            } else if common_opponents.contains(&game.home_team.team_id)
                && tied_teams.contains(&game.away_team.team_id)
            {
                match game.game_result {
                    Some(GameResult::HomeWin) => {
                        records.get_mut(&game.away_team.team_id).unwrap().1 += 1;
                    }
                    Some(GameResult::AwayWin) => {
                        records.get_mut(&game.away_team.team_id).unwrap().0 += 1;
                    }
                    Some(GameResult::Tie) => {
                        records.get_mut(&game.away_team.team_id).unwrap().2 += 1;
                    }
                    None => panic!("Game has no result"),
                }
            }
        }

        let mut working_vec: Vec<(i32, u16)> = Vec::new();
        for (team_id, record) in records {
            working_vec.push((team_id.clone(), Self::calculate_percent_from_tuple(record)));
        }
        working_vec.sort_by_key(|t| t.1);
        working_vec.reverse();

        let mut remaining_tied_teams: HashSet<i32> = HashSet::new();
        let max_pct = working_vec.get(0).unwrap().1;
        for (team_id, pct) in working_vec {
            if pct == max_pct {
                remaining_tied_teams.insert(team_id.clone());
            } else {
                break;
            }
        }

        remaining_tied_teams
    }

    fn pick_random_team_from_tied(tied_teams: HashSet<i32>) -> HashSet<i32> {
        let tied_teams_vec: Vec<i32> = Vec::from_iter(tied_teams.clone());
        let mut rng: rand::rngs::ThreadRng = rand::thread_rng();
        let index = rng.gen_range(0..tied_teams_vec.len());
        let winner = tied_teams_vec.get(index).unwrap().clone();

        let mut tied_teams = HashSet::new();
        tied_teams.insert(winner);
        tied_teams
    }

    fn increment_overall_results(&mut self) {
        let simulation_game = self.current_simulation_game.as_ref().unwrap();
        let current_result = &self.current_simulation_result;
        for team_id in current_result.division_winners.iter() {
            let lookup = SimulationResultLookup {
                game_id: simulation_game.0.clone(),
                game_result: simulation_game.1.clone(),
                team_id: team_id.clone(),
            };
            match self.overall_results.get_mut(&lookup) {
                Some(result) => {
                    result.division_winner += 1;
                }
                None => panic!("Overall results not initialized properly"),
            }
        }
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
            WHERE
                season={0}
                AND game_type='REG';
        ",
            self.season_year,
        );

        let results: Vec<Row> = run_query(query);

        for row in results {
            let game: Game = Game::new_from_db_row(row, self.teams.clone());
            self.actual_games.insert(game.game_id.clone(), game);
        }

        self.current_simulation_base_games = self.actual_games.clone();
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

pub fn run_query(query: String) -> Vec<Row> {
    let mut client: Client = connect();
    let results = match client.query(&query, &[]) {
        Ok(r) => r,
        Err(e) => panic!("{}", e),
    };
    results
}

pub fn execute(statement: String) {
    let mut client: Client = connect();
    match client.execute(&statement, &[]) {
        Ok(_) => {}
        Err(e) => panic!("{}", e),
    };
}

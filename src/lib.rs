use chrono;
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

#[derive(Clone, Debug)]
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
    pub game_id: Option<i32>,
    pub game_result: Option<GameResult>,
    pub team_id: i32,
}

#[derive(Clone, Debug)]
pub struct TeamSimulationResults {
    pub made_playoffs: i32,
    pub playoff_seedings: Vec<i32>,
    pub division_winner: i32,
    pub wildcard_team: i32,
    pub draft_picks: Vec<i32>,
}

impl TeamSimulationResults {
    fn new() -> TeamSimulationResults {
        TeamSimulationResults {
            made_playoffs: 0,
            playoff_seedings: Vec::new(),
            division_winner: 0,
            wildcard_team: 0,
            draft_picks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum PoolType {
    Division,
    Wildcard,
    DraftOrder,
    PlayoffSeeding,
}

#[derive(Clone, Debug)]
pub struct TeamPool {
    pub pool_type: PoolType,
    pub teams: HashSet<i32>,
    pub conference_mapping: HashMap<String, Vec<i32>>,
    pub division_mapping: HashMap<String, Vec<i32>>,
    pub tied_teams: HashSet<i32>,
    pub winner: Option<i32>,
    pub ranking: Option<Vec<i32>>,
    pub team_records: HashMap<i32, TeamRecord>,
    pub games: HashMap<i32, Game>,
}

impl TeamPool {
    pub fn new(source_vec: Vec<i32>, pool_type: PoolType, season: &Season) -> TeamPool {
        TeamPool {
            pool_type,
            teams: HashSet::from_iter(source_vec.clone()),
            conference_mapping: season.conference_mapping.clone(),
            division_mapping: season.division_mapping.clone(),
            tied_teams: HashSet::from_iter(source_vec.clone()),
            winner: None,
            ranking: None,
            team_records: season.current_simulation_result.team_records.clone(),
            games: season.current_simulation_games.clone(),
        }
    }

    pub fn evaluate(&mut self) {
        match self.pool_type {
            PoolType::Division => self.evaluate_division(),
            PoolType::Wildcard => self.evaluate_wildcard(),
            PoolType::DraftOrder => self.evaluate_draft_order(),
            PoolType::PlayoffSeeding => self.evaluate_playoff_seeding(),
        }
    }

    fn evaluate_division(&mut self) {
        self.break_by_percent("overall");
        self.break_by_percent("division");
        self.break_by_head_to_head();
        self.break_by_common_games(0);
        self.break_by_percent("conference");
        self.break_by_random();
        self.winner = Some(self.tied_teams.iter().next().unwrap().clone());
    }

    fn evaluate_wildcard(&mut self) {
        self.ranking = Some(Vec::new());
        for _ in 0..3 {
            self.break_by_percent("overall");
            if self.tied_teams.len() > 2 {
                self.break_wildcard_division_ties();
            }
            if self.tied_teams.len() > 2 {
                self.break_by_head_to_head_sweep();
            }
            if self.tied_teams.len() > 2 {
                self.break_by_percent("conference");
            }
            if self.tied_teams.len() > 2 {
                self.break_by_common_games(4);
            }
            if self.tied_teams.len() > 2 {
                self.pick_two_random();
            }

            self.break_by_head_to_head();
            self.break_by_percent("conference");
            self.break_by_common_games(4);
            self.break_by_random();

            let top_team = self.tied_teams.iter().next().unwrap().clone();
            self.ranking.as_mut().unwrap().push(top_team);
            self.tied_teams = self.teams.clone();
            for team_id in self.ranking.as_ref().unwrap() {
                self.tied_teams.remove(team_id);
            }
        }
    }

    fn evaluate_draft_order(&mut self) {
        todo!()
    }

    fn evaluate_playoff_seeding(&mut self) {
        todo!()
    }

    fn break_by_head_to_head_sweep(&mut self) {
        match self.tied_teams.len() {
            tt if tt > 1 => {
                let mut records: HashMap<i32, (u8, u8, u8)> = HashMap::new();
                for team_id in &self.tied_teams {
                    records.insert(team_id.clone(), (0, 0, 0));
                }
                for (_, game) in self.games.iter() {
                    if self.tied_teams.contains(&game.home_team.team_id)
                        && self.tied_teams.contains(&game.away_team.team_id)
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
                let mut sweeper: Option<i32> = None;
                let mut swept: HashSet<i32> = HashSet::new();
                for (team_id, record) in records {
                    if record.1 == 0 && record.2 == 0 {
                        sweeper = Some(team_id);
                    } else if record.0 == 0 && record.2 == 0 {
                        swept.insert(team_id);
                    }
                }

                let mut new_tied_teams: HashSet<i32> = HashSet::new();
                match sweeper {
                    Some(team_id) => {
                        new_tied_teams.insert(team_id);
                    }
                    None => {
                        for team_id in self.tied_teams.iter() {
                            if !swept.contains(team_id) {
                                new_tied_teams.insert(team_id.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn break_wildcard_division_ties(&mut self) {
        match self.tied_teams.len() {
            tt if tt > 1 => {
                let mut tied_team_divisions: HashMap<String, HashSet<i32>> = HashMap::new();
                for team_id in self.tied_teams.iter() {
                    let team_division = self.get_team_division(team_id).unwrap();
                    if tied_team_divisions.contains_key(&team_division) {
                        tied_team_divisions
                            .get_mut(&team_division)
                            .unwrap()
                            .insert(team_id.clone());
                    } else {
                        let mut new_set: HashSet<i32> = HashSet::new();
                        new_set.insert(team_id.clone());
                        tied_team_divisions.insert(team_division, new_set);
                    }
                }

                let mut division_winners: HashSet<i32> = HashSet::new();
                for (_, teams) in tied_team_divisions.iter() {
                    if teams.len() > 1 {
                        let mut division_pool = self.clone();
                        division_pool.pool_type = PoolType::Division;
                        division_pool.teams = teams.clone();
                        division_pool.tied_teams = teams.clone();
                        division_pool.evaluate();
                        division_winners.insert(division_pool.winner.unwrap());
                    } else if teams.len() == 1 {
                        for team in teams.iter() {
                            division_winners.insert(team.clone());
                        }
                    } else {
                        panic!("Division has no teams associated")
                    }
                }

                self.tied_teams = division_winners;
            }
            _ => {}
        }
    }

    fn get_team_division(&self, team_id: &i32) -> Option<String> {
        let mut team_division: Option<String> = None;
        for (division, teams) in self.division_mapping.iter() {
            if teams.contains(team_id) {
                team_division = Some(division.clone());
                break;
            }
        }
        team_division
    }

    fn break_by_percent(&mut self, percent_type: &str) {
        match self.tied_teams.len() {
            tt if tt > 1 => {
                let mut working_vec: Vec<(i32, u16)> = Vec::new();
                for team_id in self.tied_teams.iter() {
                    let percent = match percent_type {
                        t if t == "overall" => {
                            self.team_records.get(team_id).unwrap().overall_percent
                        }
                        t if t == "division" => {
                            self.team_records.get(team_id).unwrap().division_percent
                        }
                        t if t == "conference" => {
                            self.team_records.get(team_id).unwrap().conference_percent
                        }
                        t => panic!("Invalid percent type {}", t),
                    };
                    working_vec.push((team_id.clone(), percent.clone()));
                }
                working_vec.sort_by_key(|t| t.1);
                working_vec.reverse();

                let max_pct = working_vec.get(0).unwrap().1;
                self.tied_teams = HashSet::new();
                for (team_id, percent) in &working_vec {
                    if percent == &max_pct {
                        self.tied_teams.insert(team_id.clone());
                    } else {
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    fn break_by_head_to_head(&mut self) {
        match self.tied_teams.len() {
            tt if tt > 1 => {
                let mut records: HashMap<i32, (u8, u8, u8)> = HashMap::new();
                for team_id in &self.tied_teams {
                    records.insert(team_id.clone(), (0, 0, 0));
                }
                for (_, game) in self.games.iter() {
                    if self.tied_teams.contains(&game.home_team.team_id)
                        && self.tied_teams.contains(&game.away_team.team_id)
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
                    working_vec.push((
                        team_id.clone(),
                        Season::calculate_percent_from_tuple(record),
                    ));
                }
                working_vec.sort_by_key(|t| t.1);
                working_vec.reverse();

                self.tied_teams = HashSet::new();
                let max_pct = working_vec.get(0).unwrap().1;
                for (team_id, pct) in working_vec {
                    if pct == max_pct {
                        self.tied_teams.insert(team_id.clone());
                    } else {
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    fn break_by_common_games(&mut self, min_games: u8) {
        match self.tied_teams.len() {
            tt if tt > 1 => {
                let mut records: HashMap<i32, (u8, u8, u8)> = HashMap::new();
                for team_id in &self.tied_teams {
                    records.insert(team_id.clone(), (0, 0, 0));
                }

                let mut team_opponents: HashMap<i32, HashSet<i32>> = HashMap::new();
                for team_id in &self.tied_teams {
                    team_opponents.insert(team_id.clone(), HashSet::new());
                }

                for (_, game) in self.games.iter() {
                    if self.tied_teams.contains(&game.home_team.team_id) {
                        team_opponents
                            .get_mut(&game.home_team.team_id)
                            .unwrap()
                            .insert(game.away_team.team_id.clone());
                    } else if self.tied_teams.contains(&game.away_team.team_id) {
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
                let mut iter = team_opponents.iter();
                let common_opponents = iter
                    .next()
                    .map(|(_, set)| {
                        iter.fold(set.clone(), |set1, (_, set2)| {
                            set1.intersection(&set2).cloned().collect()
                        })
                    })
                    .unwrap();

                let mut total_common_games = 0;
                for (_, game) in self.games.iter() {
                    if self.tied_teams.contains(&game.home_team.team_id)
                        && common_opponents.contains(&game.away_team.team_id)
                    {
                        total_common_games += 1;
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
                        && self.tied_teams.contains(&game.away_team.team_id)
                    {
                        total_common_games += 1;
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

                match total_common_games {
                    tcg if tcg > min_games => {
                        let mut working_vec: Vec<(i32, u16)> = Vec::new();
                        for (team_id, record) in records {
                            working_vec.push((
                                team_id.clone(),
                                Season::calculate_percent_from_tuple(record),
                            ));
                        }
                        working_vec.sort_by_key(|t| t.1);
                        working_vec.reverse();

                        self.tied_teams = HashSet::new();
                        let max_pct = working_vec.get(0).unwrap().1;
                        for (team_id, pct) in working_vec {
                            if pct == max_pct {
                                self.tied_teams.insert(team_id.clone());
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn break_by_random(&mut self) {
        let tied_teams_vec: Vec<i32> = Vec::from_iter(self.tied_teams.clone());
        let mut rng: rand::rngs::ThreadRng = rand::thread_rng();
        let index = rng.gen_range(0..tied_teams_vec.len());
        let winner = tied_teams_vec.get(index).unwrap().clone();
        self.tied_teams = HashSet::new();
        self.tied_teams.insert(winner);
    }

    fn pick_two_random(&mut self) {
        let mut tied_teams_vec: Vec<i32> = Vec::from_iter(self.tied_teams.clone());
        let mut rng: rand::rngs::ThreadRng = rand::thread_rng();
        let index = rng.gen_range(0..tied_teams_vec.len());
        let winner1 = tied_teams_vec.get(index).unwrap().clone();

        tied_teams_vec.retain(|team_id| team_id != &winner1);
        let mut rng: rand::rngs::ThreadRng = rand::thread_rng();
        let index = rng.gen_range(0..tied_teams_vec.len());
        let winner2 = tied_teams_vec.get(index).unwrap().clone();

        self.tied_teams = HashSet::new();
        self.tied_teams.insert(winner1);
        self.tied_teams.insert(winner2);
    }
}

#[derive(Clone, Debug)]
pub struct Season {
    pub season_year: i32,
    pub teams: HashMap<i32, Team>,
    pub conference_mapping: HashMap<String, Vec<i32>>,
    pub division_mapping: HashMap<String, Vec<i32>>,
    pub actual_games: HashMap<i32, Game>,
    pub simulation_id: Option<i32>,
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
            simulation_id: None,
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

    pub fn run_all_game_simulations(&mut self, sims: u64, include_decided: bool) {
        self.set_simulation_id(sims.clone());

        println!("\n{} - Simulating current season state...", now(),);
        self.simulate_current_state(sims);

        let games = self.actual_games.clone();
        let total_games = games.len();
        let mut i: u32 = 1;
        for (game_id, _) in games.iter() {
            println!(
                "\n{} - Processing game {} of {} (id: {})...",
                now(),
                i,
                total_games,
                game_id
            );
            i += 1;
            let actual_game: Game = self.actual_games.get(game_id).unwrap().clone();

            let mut simulate_scenarios = || {
                println!("{} - Simulating home win...", now());
                self.simulate_for_game(game_id.clone(), GameResult::HomeWin, sims);

                println!("{} - Simulating away win...", now());
                self.simulate_for_game(game_id.clone(), GameResult::AwayWin, sims);

                println!("{} - Simulating tie...", now());
                self.simulate_for_game(game_id.clone(), GameResult::Tie, sims);
            };
            match actual_game.game_result {
                Some(_) => match include_decided {
                    true => simulate_scenarios(),
                    false => {}
                },
                None => {
                    simulate_scenarios();
                }
            }
        }
        self.insert_results();
    }

    pub fn simulate_current_state(&mut self, sims: u64) {
        for (team_id, _) in self.teams.iter() {
            let new_lookup = SimulationResultLookup {
                game_id: None,
                game_result: None,
                team_id: team_id.clone(),
            };
            self.overall_results
                .insert(new_lookup, TeamSimulationResults::new());
        }
        for _ in 0..sims {
            self.run_simulation(true);
        }
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
                game_id: Some(game_id.clone()),
                game_result: Some(game_result.clone()),
                team_id: team_id.clone(),
            };
            self.overall_results
                .insert(new_lookup, TeamSimulationResults::new());
        }

        for _ in 0..sims {
            self.run_simulation(true);
        }
    }

    pub fn run_simulation(&mut self, increment: bool) {
        self.current_simulation_result = CurrentSimulationResult::new();
        self.current_simulation_games = self.current_simulation_base_games.clone();
        for game_item in self.current_simulation_games.iter_mut() {
            let game: &mut Game = game_item.1;
            game.simulate_if_undecided();
        }
        self.evaluate_simulation_results(increment);
    }

    fn evaluate_simulation_results(&mut self, increment: bool) {
        self.populate_records();
        self.calculate_percentages();
        self.evaluate_divisions();
        self.evaluate_wildcards();
        match increment {
            true => self.increment_overall_results(),
            false => {}
        };
    }

    fn populate_records(&mut self) {
        for (team_id, _) in self.teams.iter() {
            self.current_simulation_result
                .team_records
                .insert(team_id.clone(), TeamRecord::new());
        }
        for (_, game) in self.current_simulation_games.iter() {
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
        for (_, record) in self.current_simulation_result.team_records.iter_mut() {
            record.overall_percent = Self::calculate_percent_from_tuple(record.overall_record);
            record.conference_percent =
                Self::calculate_percent_from_tuple(record.conference_record);
            record.division_percent = Self::calculate_percent_from_tuple(record.division_record);
        }
    }

    pub fn calculate_percent_from_tuple(record_tuple: (u8, u8, u8)) -> u16 {
        let (wins, losses, ties) = record_tuple;
        let wins: u32 = u32::from(wins);
        let losses: u32 = u32::from(losses);
        let ties: u32 = u32::from(ties);
        let computed_wins: u32 = (wins * 1000) + ((ties * 1000) / 2);

        let total_games = wins + losses + ties;
        let win_percent: u16;
        match total_games {
            tg if tg != 0 => {
                win_percent = u16::try_from(computed_wins / (wins + losses + ties)).unwrap();
            }
            _ => {
                win_percent = 0;
            }
        }

        win_percent
    }

    fn evaluate_divisions(&mut self) {
        for (_, team_ids) in self.division_mapping.iter() {
            let mut team_pool: TeamPool = TeamPool::new(team_ids.clone(), PoolType::Division, self);
            team_pool.evaluate();
            self.current_simulation_result
                .division_winners
                .insert(team_pool.winner.unwrap());
        }
    }

    fn evaluate_wildcards(&mut self) {
        for (_, team_ids) in self.conference_mapping.iter() {
            let mut team_ids_without_division_winners = team_ids.clone();

            team_ids_without_division_winners.retain(|team_id| {
                !self
                    .current_simulation_result
                    .division_winners
                    .contains(team_id)
            });

            let mut team_pool: TeamPool = TeamPool::new(
                team_ids_without_division_winners.clone(),
                PoolType::Wildcard,
                self,
            );
            team_pool.evaluate();
            for team_id in team_pool.ranking.unwrap() {
                self.current_simulation_result
                    .wildcard_teams
                    .insert(team_id);
            }
        }
    }

    fn increment_overall_results(&mut self) {
        let simulation_game: Option<&(i32, GameResult)> = self.current_simulation_game.as_ref();
        let current_result = &self.current_simulation_result;
        for team_id in current_result.division_winners.iter() {
            let lookup = match simulation_game {
                Some(sg) => SimulationResultLookup {
                    game_id: Some(sg.0.clone()),
                    game_result: Some(sg.1.clone()),
                    team_id: team_id.clone(),
                },
                None => SimulationResultLookup {
                    game_id: None,
                    game_result: None,
                    team_id: team_id.clone(),
                },
            };
            match self.overall_results.get_mut(&lookup) {
                Some(result) => {
                    result.division_winner += 1;
                }
                None => panic!("Overall results not initialized properly"),
            }
        }
        for team_id in current_result.wildcard_teams.iter() {
            let lookup = match simulation_game {
                Some(sg) => SimulationResultLookup {
                    game_id: Some(sg.0.clone()),
                    game_result: Some(sg.1.clone()),
                    team_id: team_id.clone(),
                },
                None => SimulationResultLookup {
                    game_id: None,
                    game_result: None,
                    team_id: team_id.clone(),
                },
            };
            match self.overall_results.get_mut(&lookup) {
                Some(result) => {
                    result.wildcard_team += 1;
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

    pub fn set_simulation_id(&mut self, sims: u64) {
        // Insert new simulation into db and add simulation_id to Season struct
        let statement = format!(
            "
                INSERT INTO  nfl.simulations
                VALUES (
                    DEFAULT,
                    NOW(),
                    {},
                    {}
                )
            ",
            self.season_year, sims,
        );
        execute(statement);

        let query = String::from(
            "
            SELECT MAX(simulation_id)
            FROM nfl.simulations;
        ",
        );

        let results: Vec<Row> = run_query(query);

        for row in results {
            self.simulation_id = Some(row.get(0));
        }
    }

    fn insert_results(&self) {
        // Insert all results in self.overall_results into database
        println!("Inserting results...");
        let mut new_rows: Vec<String> = Vec::new();
        for (lookup, result) in self.overall_results.iter() {
            let simulation_id = self.simulation_id.unwrap();
            let game_id: String = match lookup.game_id {
                Some(gid) => format!("{gid}"),
                None => String::from("NULL"),
            };
            let simulated_game_result = match &lookup.game_result {
                Some(gr) => match gr {
                    GameResult::HomeWin => String::from("'home win'"),
                    GameResult::AwayWin => String::from("'away win'"),
                    GameResult::Tie => String::from("'tie'"),
                },
                None => String::from("NULL"),
            };
            let simulation_team_id = lookup.team_id;
            let mut results: HashMap<String, i32> = HashMap::new();
            results.insert(String::from("division winner"), result.division_winner);
            results.insert(String::from("wildcard team"), result.wildcard_team);

            for (season_outcome, simulations_with_outcome) in results.iter() {
                let new_row: String = format!(
                    "(DEFAULT,{simulation_id},{game_id},{simulated_game_result},{simulation_team_id},'{season_outcome}',{simulations_with_outcome})",
                );
                new_rows.push(new_row);
            }
        }
        let statement: String = format!(
            "INSERT INTO nfl.simulation_results
            VALUES {}",
            new_rows.join(","),
        );
        execute(statement);
    }
}

fn get_variable(key: &str) -> String {
    match var(key) {
        Ok(val) => val,
        Err(err) => panic!("{}", err),
    }
}

fn get_conn_string() -> String {
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
        Err(e) => println!(
            "Failed to execute statement:\n\n{}\n\n{}\n------------------------------",
            statement, e
        ),
    };
}

pub fn now() -> String {
    let time = chrono::offset::Local::now();

    time.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

CREATE TYPE nfl.gameresult AS ENUM ('home win', 'away win', 'tie');

CREATE TABLE nfl.simulations (
    simulation_id serial4 NOT NULL,
    simulation_timestamp timestamptz NOT NULL DEFAULT NOW(),
	season int4 NOT NULL,
    simulations_per_game_result bigint NOT NULL,
    CONSTRAINT simulations_pkey PRIMARY KEY (simulation_id)
);

CREATE TABLE nfl.simulation_results (
    simulation_result_id bigserial NOT NULL,
    simulation_id int4 NOT NULL,
    game_id int4,
    simulated_game_result nfl.gameresult,
    simulation_team_id int4 NOT NULL,
    season_outcome varchar(20) NOT NULL,
    simulations_with_outcome bigint NOT NULL,
    CONSTRAINT simulation_results_pkey PRIMARY KEY (simulation_result_id),
    CONSTRAINT simulation_results_simulation_id_fkey FOREIGN KEY (simulation_id) REFERENCES nfl.simulations(simulation_id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT simulation_results_game_id_fkey FOREIGN KEY (game_id) REFERENCES nfl.games(game_id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT simulation_results_simulation_team_id_fkey FOREIGN KEY (simulation_team_id) REFERENCES nfl.teams(team_id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE VIEW nfl.division_winners 
AS
	WITH
		cte AS (
			SELECT
				s.simulation_id,
				g.game_id,
				g.week,
				t3.abbreviation AS sim_team,
				t1.abbreviation AS home_team,
				t2.abbreviation AS away_team,
				sr.simulated_game_result,
				sr.simulations_with_outcome,
				s.simulations_per_game_result AS total_simulations
			FROM nfl.simulation_results sr
			LEFT JOIN nfl.games g
			USING (game_id)
			LEFT JOIN nfl.teams t1
			ON g.home_team_id=t1.team_id
			LEFT JOIN nfl.teams t2
			ON g.away_team_id=t2.team_id
			LEFT JOIN nfl.teams t3
			ON sr.simulation_team_id=t3.team_id
			LEFT JOIN nfl.simulations s
			USING (simulation_id)
			WHERE sr.season_outcome='division winner'
		),
		cte2 AS (
			SELECT
				simulation_id,
				game_id,
				week,
				sim_team,
				home_team,
				away_team,
				total_simulations,
				MAX(CASE WHEN simulated_game_result='home win' THEN simulations_with_outcome ELSE NULL END) AS home_win,
				MAX(CASE WHEN simulated_game_result='away win' THEN simulations_with_outcome ELSE NULL END) AS away_win,
				MAX(CASE WHEN simulated_game_result='tie' THEN simulations_with_outcome ELSE NULL END) AS tie
			FROM cte
			GROUP BY
				simulation_id,
				game_id,
				week,
				sim_team,
				home_team,
				away_team,
				total_simulations
		),
		cte3 AS (
			SELECT
				*,
				ABS(home_win - away_win) AS difference
			FROM cte2
		)
	SELECT *
	FROM cte3;

CREATE VIEW nfl.wildcard_teams 
AS
	WITH
		cte AS (
			SELECT
				s.simulation_id,
				g.game_id,
				g.week,
				t3.abbreviation AS sim_team,
				t1.abbreviation AS home_team,
				t2.abbreviation AS away_team,
				sr.simulated_game_result,
				sr.simulations_with_outcome,
				s.simulations_per_game_result AS total_simulations
			FROM nfl.simulation_results sr
			LEFT JOIN nfl.games g
			USING (game_id)
			LEFT JOIN nfl.teams t1
			ON g.home_team_id=t1.team_id
			LEFT JOIN nfl.teams t2
			ON g.away_team_id=t2.team_id
			LEFT JOIN nfl.teams t3
			ON sr.simulation_team_id=t3.team_id
			LEFT JOIN nfl.simulations s
			USING (simulation_id)
			WHERE sr.season_outcome='wildcard team'
		),
		cte2 AS (
			SELECT
				simulation_id,
				game_id,
				week,
				sim_team,
				home_team,
				away_team,
				total_simulations,
				MAX(CASE WHEN simulated_game_result='home win' THEN simulations_with_outcome ELSE NULL END) AS home_win,
				MAX(CASE WHEN simulated_game_result='away win' THEN simulations_with_outcome ELSE NULL END) AS away_win,
				MAX(CASE WHEN simulated_game_result='tie' THEN simulations_with_outcome ELSE NULL END) AS tie
			FROM cte
			GROUP BY
				simulation_id,
				game_id,
				week,
				sim_team,
				home_team,
				away_team,
				total_simulations
		),
		cte3 AS (
			SELECT
				*,
				ABS(home_win - away_win) AS difference
			FROM cte2
		)
	SELECT *
	FROM cte3;

CREATE VIEW nfl.current_state 
AS
	WITH
	cte AS (
		SELECT
			s.simulation_id,
			t.abbreviation AS sim_team,
			sr.season_outcome,
			sr.simulations_with_outcome,
			s.simulations_per_game_result AS total_simulations
		FROM nfl.simulation_results sr 
		LEFT JOIN nfl.simulations s 
		USING (simulation_id)
		LEFT JOIN nfl.teams t
		ON sr.simulation_team_id=t.team_id
		WHERE game_id IS NULL
	)
	SELECT *
	FROM cte;
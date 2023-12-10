CREATE TYPE nfl.gameresult AS ENUM ('home win', 'away win', 'tie');

CREATE TYPE nfl.resultset AS ENUM ('playoff seed', 'draft position');

CREATE TABLE IF NOT EXISTS nfl.simulations (
    simulation_id serial4 NOT NULL,
    simulation_timestamp timestamptz NOT NULL DEFAULT NOW(),
	season int4 NOT NULL,
    simulations_per_game_result bigint NOT NULL,
    CONSTRAINT simulations_pkey PRIMARY KEY (simulation_id)
);

CREATE TABLE IF NOT EXISTS nfl.simulation_results (
    simulation_result_id bigserial NOT NULL,
    simulation_id int4 NOT NULL,
    game_id int4,
    simulated_game_result nfl.gameresult,
    simulation_team_id int4 NOT NULL,
	result_set nfl.resultset,
	team_rank smallint,
    simulations_with_rank bigint NOT NULL,
    CONSTRAINT simulation_results_pkey PRIMARY KEY (simulation_result_id),
    CONSTRAINT simulation_results_simulation_id_fkey FOREIGN KEY (simulation_id) REFERENCES nfl.simulations(simulation_id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT simulation_results_game_id_fkey FOREIGN KEY (game_id) REFERENCES nfl.games(game_id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT simulation_results_simulation_team_id_fkey FOREIGN KEY (simulation_team_id) REFERENCES nfl.teams(team_id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE VIEW nfl.simulation_results_readable 
AS 
	WITH
		cte1 AS (
			SELECT
				s.simulation_id AS simulation_id,
				s.simulation_timestamp AS simulation_timestamp,
				t3.abbreviation AS simulation_team,
				g.week,
				g.api_game_id AS api_game_id,
				t1.abbreviation AS home_team,
				t2.abbreviation AS away_team,
				sr.simulated_game_result AS simulated_game_result,
				sr.result_set AS result_set,
				sr.team_rank AS team_rank,
				sr.simulations_with_rank AS simulations_with_rank,
				s.simulations_per_game_result AS simulations_run
			FROM nfl.simulation_results sr
			LEFT JOIN nfl.simulations s
			USING (simulation_id)
			LEFT JOIN nfl.games g
			USING (game_id)
			LEFT JOIN nfl.teams t1
			ON g.home_team_id = t1.team_id
			LEFT JOIN nfl.teams t2
			ON g.away_team_id = t2.team_id
			LEFT JOIN nfl.teams t3
			ON sr.simulation_team_id = t3.team_id
			WHERE
				g.api_game_id IS NOT NULL
				AND g.game_type = 'REG'
		),
		cte2 AS (
			SELECT
				*,
				CASE 
					WHEN result_set = 'playoff seed' AND team_rank <= 4 THEN 'division winner'
					WHEN result_set = 'playoff seed' AND team_rank >= 5 THEN 'wildcard team'
					WHEN result_set = 'draft position' AND team_rank = 1 THEN 'first pick'
					ELSE NULL
				END AS result_condition,
				CAST(simulations_with_rank AS float) / CAST(simulations_run AS float) AS probability
			FROM cte1
		),
		cte3 AS (
			SELECT
				*,
				CASE
					WHEN result_set = 'playoff seed' AND team_rank <= 7 THEN 'playoff team'
					WHEN result_set = 'draft position' AND team_rank <= 5 THEN 'top 5 pick'
					ELSE NULL
				END AS result_condition,
				CAST(simulations_with_rank AS float) / CAST(simulations_run AS float) AS probability
			FROM cte1
		),
		cte4 AS (
			SELECT
				*,
				CASE
					WHEN result_set = 'draft position' AND team_rank <= 10 THEN 'top 10 pick'
					ELSE NULL
				END AS result_condition,
				CAST(simulations_with_rank AS float) / CAST(simulations_run AS float) AS probability
			FROM cte1
		),
		cte5 AS (
			SELECT *
			FROM cte2
			UNION (
				SELECT *
				FROM cte3
			)
			UNION (
				SELECT *
				FROM cte4
			)
		),
		cte6 AS (
			SELECT
				simulation_id,
				simulation_timestamp,
				simulation_team,
				week,
				api_game_id,
				home_team,
				away_team,
				result_condition,
				SUM(
					CASE
						WHEN simulated_game_result = 'home win' THEN probability
						ELSE 0
					END
				) AS home_win,
				SUM(
					CASE
						WHEN simulated_game_result = 'tie' THEN probability
						ELSE 0
					END
				) AS tie,
				SUM(
					CASE
						WHEN simulated_game_result = 'away win' THEN probability
						ELSE 0
					END
				) AS away_win
			FROM cte5
			WHERE result_condition IS NOT NULL
			GROUP BY
				simulation_id,
				simulation_timestamp,
				simulation_team,
				week,
				api_game_id,
				home_team,
				away_team,
				result_condition
		),
		cte7 AS (
		SELECT
			*,
			ABS(home_win - away_win) AS difference
		FROM cte6
		)

	SELECT *
	FROM cte7;
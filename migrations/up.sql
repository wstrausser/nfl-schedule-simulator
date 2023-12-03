CREATE TYPE nfl.gameresult AS ENUM ('home win', 'away win', 'tie');

CREATE TYPE nfl.resultset AS ENUM ('playoff seed', 'draft position');

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
	result_set nfl.resultset,
	team_rank smallint,
    simulations_with_rank bigint NOT NULL,
    CONSTRAINT simulation_results_pkey PRIMARY KEY (simulation_result_id),
    CONSTRAINT simulation_results_simulation_id_fkey FOREIGN KEY (simulation_id) REFERENCES nfl.simulations(simulation_id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT simulation_results_game_id_fkey FOREIGN KEY (game_id) REFERENCES nfl.games(game_id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT simulation_results_simulation_team_id_fkey FOREIGN KEY (simulation_team_id) REFERENCES nfl.teams(team_id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE VIEW nfl.current_state 
AS 
	WITH
		cte AS (
			SELECT
				s.simulation_timestamp,
				t.abbreviation AS simulation_team,
				sr.result_set,
				sr.team_rank,
				sr.simulations_with_rank,
				s.simulations_per_game_result AS total_simulations
			FROM nfl.simulation_results sr 
			LEFT JOIN nfl.simulations s 
			USING (simulation_id)
			LEFT JOIN nfl.teams t
			ON sr.simulation_team_id=t.team_id
			WHERE
				game_id IS NULL
				AND simulation_id = (
					SELECT MAX(simulation_id)
					FROM nfl.simulations
				)
		)
		SELECT
			*,
			CAST(simulations_with_rank AS float4) / total_simulations AS probability
		FROM cte
		ORDER BY simulation_team, result_set, team_rank;

CREATE VIEW nfl.game_simulations_readable 
AS
	WITH
		cte1 AS (
			SELECT
				s.simulation_timestamp,
				t.abbreviation AS simulation_team,
				sr.game_id,
				sr.simulated_game_result,
				sr.result_set,
				sr.team_rank,
				sr.simulations_with_rank,
				s.simulations_per_game_result AS total_simulations
			FROM nfl.simulation_results sr 
			LEFT JOIN nfl.simulations s 
			USING (simulation_id)
			LEFT JOIN nfl.teams t
			ON sr.simulation_team_id=t.team_id
			WHERE
				game_id IS NOT NULL
				AND simulation_id = (
					SELECT MAX(simulation_id)
					FROM nfl.simulations
				)
		),
		cte2 AS (
			SELECT
				g.game_id,
				g.api_game_id,
				g.season,
				g.week,
				t1.abbreviation AS home_team,
				t2.abbreviation AS away_team,
				g.home_score,
				g.away_score
			FROM nfl.games g
			LEFT JOIN nfl.teams t1 
			ON g.home_team_id = t1.team_id
			LEFT JOIN nfl.teams t2 
			ON g.away_team_id = t2.team_id
		)
		SELECT
			c1.simulation_timestamp,
			c2.api_game_id,
			c2.season,
			c2.week,
			CASE
				WHEN c1.simulated_game_result = 'home win' THEN c2.home_team
				WHEN c1.simulated_game_result = 'away win' THEN c2.away_team
				WHEN c1.simulated_game_result = 'tie' THEN 'tie'
			END AS winner,
			CASE
				WHEN c1.simulated_game_result = 'home win' THEN c2.away_team
				WHEN c1.simulated_game_result = 'away win' THEN c2.home_team
				WHEN c1.simulated_game_result = 'tie' THEN 'tie'
			END AS loser,
			c1.simulation_team,
			c1.result_set,
			c1.team_rank,
			c1.simulations_with_rank,
			c1.total_simulations,
			CAST (c1.simulations_with_rank AS float4) / c1.total_simulations AS probability		
		FROM cte1 c1
		LEFT JOIN cte2 c2
		USING (game_id)
		ORDER BY
			simulation_team,
			api_game_id,
			result_set,
			team_rank,
			winner;
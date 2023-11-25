CREATE TYPE nfl.gameresult AS ENUM ('home win', 'away win', 'tie');

CREATE TABLE nfl.simulation_results (
    simulation_result_id bigserial NOT NULL,
    simulation_timestamp timestamptz NOT NULL,
    game_id int4 NOT NULL,
    simulated_game_result nfl.gameresult NOT NULL,
    simulation_team_id int4 NOT NULL,
    simulations_run bigint NOT NULL,
    season_outcome varchar(20) NOT NULL,
    simulations_with_outcome bigint NOT NULL,
    CONSTRAINT simulation_results_pkey PRIMARY KEY (simulation_result_id),
    CONSTRAINT simulation_results_game_id_fkey FOREIGN KEY (game_id) REFERENCES nfl.games(game_id) ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT simulation_results_simulation_team_id_fkey FOREIGN KEY (simulation_team_id) REFERENCES nfl.teams(team_id) ON DELETE CASCADE ON UPDATE CASCADE
);

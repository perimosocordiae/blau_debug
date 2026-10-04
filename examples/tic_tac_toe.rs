use blau_api::{DynSafeGameAPI, GameAPI, PlayerInfo, Result};
use blau_debug::run_server;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
enum GridCell {
    X,
    O,
    Empty,
}
#[derive(Serialize, Deserialize)]
struct GameState {
    grid: [[GridCell; 3]; 3],
    curr_player_idx: usize,
}
impl GameState {
    fn empty_cell(&self) -> Result<(usize, usize)> {
        for (r, row) in self.grid.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if *cell == GridCell::Empty {
                    return Ok((r, c));
                }
            }
        }
        Err("No empty cell".into())
    }
}

#[derive(Serialize, Deserialize)]
struct Action {
    row: usize,
    col: usize,
}

struct TicTacToe {
    game: GameState,
    player_info: Vec<PlayerInfo>,
    game_over: bool,
}
impl TicTacToe {
    fn do_action<F: FnMut(&str, &str)>(&mut self, action: &Action, mut notice_cb: F) -> Result<()> {
        let new_cell = if self.game.curr_player_idx == 0 {
            GridCell::X
        } else {
            GridCell::O
        };
        match self.game.grid[action.row][action.col] {
            GridCell::Empty => {
                self.game.grid[action.row][action.col] = new_cell;
            }
            _ => {
                return Err("Cell already occupied".into());
            }
        }
        // Check for game over.
        let grid = &self.game.grid;
        if grid[0][0] == new_cell {
            self.game_over = (grid[0][1] == new_cell && grid[0][2] == new_cell)
                || (grid[1][0] == new_cell && grid[2][0] == new_cell)
                || (grid[1][1] == new_cell && grid[2][2] == new_cell);
        } else if grid[0][1] == new_cell {
            self.game_over = grid[1][1] == new_cell && grid[2][1] == new_cell;
        } else if grid[0][2] == new_cell {
            self.game_over = (grid[1][2] == new_cell && grid[2][2] == new_cell)
                || (grid[1][1] == new_cell && grid[2][0] == new_cell);
        } else if grid[1][0] == new_cell {
            self.game_over = grid[1][1] == new_cell && grid[1][2] == new_cell;
        } else if grid[2][0] == new_cell {
            self.game_over = grid[2][1] == new_cell && grid[2][2] == new_cell;
        }
        // Next player.
        if !self.game_over {
            if self.game.empty_cell().is_ok() {
                self.game.curr_player_idx += 1;
                self.game.curr_player_idx %= 2;
            } else {
                self.game_over = true; // Cat's game.
            }
        }

        let view = serde_json::to_string(&self.game)?;
        for p in self.player_info.iter() {
            notice_cb(&p.id, &view);
        }
        Ok(())
    }
    fn process_agents<F: FnMut(&str, &str)>(&mut self, mut notice_cb: F) -> Result<()> {
        while !self.game_over && self.player_info[self.game.curr_player_idx].level.is_some() {
            let (row, col) = self.game.empty_cell()?;
            self.do_action(&Action { row, col }, &mut notice_cb)?;
        }
        Ok(())
    }
}
impl GameAPI for TicTacToe {
    fn init(players: &[PlayerInfo], _params: Option<&str>) -> Result<Self> {
        if players.len() != 2 {
            return Err("Need exactly two players".into());
        }
        let game = GameState {
            grid: [
                [GridCell::Empty, GridCell::Empty, GridCell::Empty],
                [GridCell::Empty, GridCell::Empty, GridCell::Empty],
                [GridCell::Empty, GridCell::Empty, GridCell::Empty],
            ],
            curr_player_idx: 0,
        };
        Ok(Self {
            game,
            player_info: players.to_vec(),
            game_over: false,
        })
    }
    fn restore(players: &[PlayerInfo], final_state: &str) -> Result<Self> {
        let game: GameState = serde_json::from_str(final_state)?;
        let mut res = Self::init(players, None)?;
        res.game = game;
        res.game_over = true;
        Ok(res)
    }
    fn start<F: FnMut(&str, &str)>(&mut self, game_id: i64, mut notice_cb: F) -> Result<()> {
        let msg = format!(r#"{{"action": "start", "game_id": {game_id}}}"#);
        for p in self.player_info.iter() {
            notice_cb(&p.id, &msg);
        }
        // Advance to wait for the next player action.
        self.process_agents(notice_cb)?;
        Ok(())
    }
    fn process_action<F: FnMut(&str, &str)>(
        &mut self,
        action: &str,
        mut notice_cb: F,
    ) -> Result<()> {
        if self.game_over {
            return Err("Game is over".into());
        }
        let action: Action = serde_json::from_str(action)?;
        self.do_action(&action, &mut notice_cb)?;
        // Advance to wait for the next player action.
        self.process_agents(&mut notice_cb)?;
        Ok(())
    }
}
impl DynSafeGameAPI for TicTacToe {
    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn final_state(&self) -> Result<String> {
        if !self.game_over {
            return Err("Game is not finished".into());
        }
        // TODO: include scores here.
        Ok(serde_json::to_string(&self.game)?)
    }

    fn player_view(&self, _player_id: &str) -> Result<String> {
        Ok(serde_json::to_string(&self.game)?)
    }

    fn current_player_id(&self) -> &str {
        &self.player_info[self.game.curr_player_idx].id
    }

    fn player_scores(&self) -> Vec<i32> {
        // TODO: check for three in a row
        vec![0, 0]
    }
}

fn main() {
    env_logger::init();
    run_server::<TicTacToe>();
}

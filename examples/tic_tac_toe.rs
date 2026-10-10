use blau_api::{DynSafeGameAPI, GameAPI, PlayerInfo, Result};
use blau_debug::run_server;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
enum GridCell {
    X,
    O,
    Empty,
}
#[derive(Serialize, Deserialize, Clone)]
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
    fn winner(&self) -> Option<GridCell> {
        let grid = &self.grid;
        for row in grid.iter() {
            if row[0] != GridCell::Empty && row[0] == row[1] && row[1] == row[2] {
                return Some(row[0]);
            }
        }
        for c in 0..3 {
            if grid[0][c] != GridCell::Empty && grid[0][c] == grid[1][c] && grid[1][c] == grid[2][c]
            {
                return Some(grid[0][c]);
            }
        }
        if grid[0][0] != GridCell::Empty && grid[0][0] == grid[1][1] && grid[1][1] == grid[2][2] {
            return Some(grid[0][0]);
        }
        if grid[0][2] != GridCell::Empty && grid[0][2] == grid[1][1] && grid[1][1] == grid[2][0] {
            return Some(grid[0][2]);
        }
        // If no cells are Empty and no winner, it's a draw, represented by returning Empty.
        if grid
            .iter()
            .all(|row| row.iter().all(|&cell| cell != GridCell::Empty))
        {
            return Some(GridCell::Empty);
        }
        None
    }
}

#[derive(Serialize, Deserialize)]
struct FinalState {
    game: GameState,
    winner: GridCell,
}

#[derive(Serialize, Deserialize)]
struct Action {
    row: usize,
    col: usize,
}

struct TicTacToe {
    game: GameState,
    player_info: Vec<PlayerInfo>,
    winner: Option<GridCell>,
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
        self.winner = self.game.winner();
        // Next player.
        if self.winner.is_none() {
            self.game.curr_player_idx += 1;
            self.game.curr_player_idx %= 2;
        }

        let view = serde_json::to_string(&self.game)?;
        for p in self.player_info.iter() {
            notice_cb(&p.id, &view);
        }
        Ok(())
    }
    fn process_agents<F: FnMut(&str, &str)>(&mut self, mut notice_cb: F) -> Result<()> {
        while self.winner.is_none() && self.player_info[self.game.curr_player_idx].level.is_some() {
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
            winner: None,
        })
    }
    fn restore(players: &[PlayerInfo], final_state: &str) -> Result<Self> {
        let fs: FinalState = serde_json::from_str(final_state)?;
        let mut res = Self::init(players, None)?;
        res.game = fs.game;
        res.winner = Some(fs.winner);
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
        if self.winner.is_some() {
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
        self.winner.is_some()
    }

    fn final_state(&self) -> Result<String> {
        match self.winner {
            Some(w) => Ok(serde_json::to_string(&FinalState {
                game: self.game.clone(),
                winner: w,
            })?),
            None => Err("Game is not finished".into()),
        }
    }

    fn player_view(&self, _player_id: &str) -> Result<String> {
        Ok(serde_json::to_string(&self.game)?)
    }

    fn current_player_id(&self) -> &str {
        &self.player_info[self.game.curr_player_idx].id
    }

    fn player_scores(&self) -> Vec<i32> {
        match self.winner {
            Some(GridCell::X) => vec![1, 0],
            Some(GridCell::O) => vec![0, 1],
            _ => vec![0, 0],
        }
    }
}

fn main() {
    env_logger::init();
    run_server::<TicTacToe>();
}

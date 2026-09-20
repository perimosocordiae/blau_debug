use blau_api::{GameAPI, PlayerInfo};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead};

#[macro_use]
extern crate log;

#[derive(Deserialize, Serialize)]
struct StartMessage {
    players: Vec<(String, Option<u8>)>,
    params: Option<String>,
}

pub fn run_server<G: GameAPI>() {
    info!("Starting game debug server...");
    let stdin = io::stdin();
    let mut game: Option<G> = None;

    let mut notice_cb = |id: &str, msg: &str| {
        info!("NOTICE: {id} sent: {msg}");
    };

    while let Some(Ok(line)) = stdin.lock().lines().next() {
        match game {
            None => match serde_json::from_str::<StartMessage>(&line) {
                Ok(msg) => {
                    let players: Vec<PlayerInfo> = msg
                        .players
                        .iter()
                        .map(|(id, level)| PlayerInfo {
                            id: id.to_string(),
                            level: *level,
                        })
                        .collect();
                    match G::init(&players, msg.params.as_deref()) {
                        Ok(mut g) => {
                            info!("Game initialized with {} players.", msg.players.len());
                            if let Err(e) = g.start(0, &mut notice_cb) {
                                println!("\"Failed to start game: {e}\"");
                            } else {
                                if g.is_game_over() {
                                    print_final_state(&g);
                                    break;
                                } else {
                                    print_current_state(&g);
                                    game = Some(g);
                                }
                            }
                        }
                        Err(e) => {
                            println!("\"Invalid player info: {e}\"");
                        }
                    }
                }
                Err(e) => {
                    println!("\"Invalid StartMessage json: {e}\"");
                }
            },
            Some(ref mut g) => {
                info!("Got action: {line}");
                if let Err(e) = g.process_action(&line, &mut notice_cb) {
                    println!("\"Invalid action: {e}\"");
                } else {
                    if g.is_game_over() {
                        print_final_state(g);
                        break;
                    } else {
                        print_current_state(g);
                    }
                }
            }
        }
    }
}

fn print_current_state(g: &impl GameAPI) {
    let p = g.current_player_id();
    info!("Next turn: {p}");
    match g.player_view(p) {
        Ok(j) => {
            println!("{j}");
        }
        Err(e) => {
            println!("\"Failed to make current state: {e}\"");
        }
    }
}
fn print_final_state(g: &impl GameAPI) {
    info!("Game over. Final scores: {:?}", g.player_scores());
    match g.final_state() {
        Ok(j) => {
            println!("{j}");
        }
        Err(e) => {
            println!("\"Failed to make final state: {e}\"");
        }
    }
}

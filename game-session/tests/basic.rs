#![no_std]

use game_session_io::*;
use gtest::{Program, ProgramBuilder, System};

const USER1: u64 = 10;
const SESSION_PROGRAM_ID: u64 = 1;
const TARGET_PROGRAM_ID: u64 = 2;

#[test]
fn test_game_session_state() {
    let system = System::new();
    system.init_logger();

    let proxy_program: Program =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
            .with_id(SESSION_PROGRAM_ID)
            .build(&system);

    let target_program: Program =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(TARGET_PROGRAM_ID)
            .build(&system);

    system.mint_to(USER1, 10000000000000000);
    target_program.send_bytes(USER1, []);
    system.run_next_block();
    proxy_program.send(USER1, target_program.id());
    system.run_next_block();
    proxy_program.send(USER1, SessionAction::StartGame { user: USER1.into() });
    system.run_next_block();
    proxy_program.send(
        USER1,
        SessionAction::CheckWord {
            user: USER1.into(),
            word: "house".into(),
        },
    );
    system.run_next_block();

    let state: Session = proxy_program.read_state(()).expect("Failed to read state");
    assert_eq!(state.session_status, SessionStatus::Waiting);

    proxy_program.send(
        USER1,
        SessionAction::CheckWord {
            user: USER1.into(),
            word: "horse".into(),
        },
    );
    system.run_next_block();
    proxy_program.send(
        USER1,
        SessionAction::CheckWord {
            user: USER1.into(),
            word: "human".into(),
        },
    );
    system.run_next_block();
    let state: Session = proxy_program.read_state(()).expect("Failed to read state");
    assert_eq!(
        state.session_status,
        SessionStatus::GameEnded {
            result: GameResult::Win
        }
    );
}

#[test]
fn test_timeout() {
    let system = System::new();
    system.init_logger();

    let proxy_program: Program =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
            .with_id(SESSION_PROGRAM_ID)
            .build(&system);

    let target_program: Program =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(TARGET_PROGRAM_ID)
            .build(&system);

    system.mint_to(USER1, 10000000000000000);

    target_program.send_bytes(USER1, []);
    system.run_next_block();
    proxy_program.send(USER1, target_program.id());
    system.run_next_block();
    proxy_program.send(USER1, SessionAction::StartGame { user: USER1.into() });
    system.run_next_block();
    proxy_program.send(
        USER1,
        SessionAction::CheckWord {
            user: USER1.into(),
            word: "hello".into(),
        },
    );
    system.run_next_block();
    system.run_to_block(300);

    proxy_program.send(USER1, SessionAction::CheckGameStatus { user: USER1.into() });
    system.run_next_block();

    let state: Session = proxy_program.read_state(()).expect("Failed to read state");
    assert_eq!(
        state.session_status,
        SessionStatus::GameEnded {
            result: GameResult::Lose
        }
    );
}

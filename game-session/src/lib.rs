#![no_std]
#![allow(static_mut_refs)]
use game_session_io::*;
use gstd::{debug, exec, msg, prelude::*, ActorId};
use wordle_io::*;

static mut SESSION: Option<Session> = None;

#[no_mangle]
extern "C" fn init() {
    let target_program_id = msg::load().expect("Unable to decode Init");

    unsafe {
        SESSION = Some(Session {
            target_program_id,
            session_status: SessionStatus::Waiting,
            game_status: GameStatus { game_result: None },
            msg_ids: Some((msg::id(), msg::id())),
            guess_count: 0,
            start_block: exec::block_height(),
        });
    }
}

#[no_mangle]
extern "C" fn handle() {
    debug!("===HANDLE START===");
    let session = unsafe { SESSION.as_mut().expect("The session is not initialized") };
    debug!("---SESSION: {:?}---", session);
    let action: SessionAction = msg::load().expect("Unable to decode `Action`");
    debug!("---SESSION ACTION: {:?}---", action);

    match &session.session_status {
        SessionStatus::Waiting => match action {
            SessionAction::StartGame { user } => {
                debug!("===WAITING AND START GAME===");
                msg::send(session.target_program_id, Action::StartGame { user }, 0)
                    .expect("Error in sending a message");
                session.session_status = SessionStatus::MessageSent;
                exec::wait();
            }
            SessionAction::CheckWord { user, ref word } => {
                debug!("===CHECK WORD FOR USER: {:?}===", user);

                if session.guess_count >= 6 {
                    msg::reply(
                        SessionEvent::GameError("Game over: Too many guesses".into()),
                        0,
                    )
                    .expect("Unable to reply");
                    return;
                }

                let current_game_status = get_game_status();

                if current_game_status.game_result.is_some() {
                    debug!("===GAME RESULT IS FIXED FOR USER: {:?}===", user);
                    msg::reply(SessionEvent::GameStatus(current_game_status.clone()), 0)
                        .expect("Unable to reply");
                } else {
                    msg::send(
                        session.target_program_id,
                        Action::CheckWord {
                            user,
                            word: word.clone(),
                        },
                        0,
                    )
                    .expect("Error in sending a message");
                    session.session_status = SessionStatus::MessageSent;
                    exec::wait();
                }
            }
            SessionAction::CheckGameStatus { user } => {
                debug!("===CHECK GAME STATUS===");
                check_game_status(session, user);
            }
        },
        SessionStatus::MessageSent => {
            debug!("===MESSAGE SENT===");
            msg::reply(
                SessionEvent::GameError("Message has already been sent, restart the game".into()),
                0,
            )
            .expect("Error in sending a reply");
        }
        SessionStatus::MessageReceived(event) => {
            debug!("===MESSAGE RECEIVED===");
            let session_event;
            match event {
                Event::GameStarted { user } => {
                    session_event = SessionEvent::GameStarted { user: *user };
                    msg::send_delayed(
                        exec::program_id(),
                        SessionAction::CheckGameStatus { user: *user },
                        0,
                        200,
                    )
                    .expect("Failed to send delayed message");
                    msg::reply(session_event.clone(), 0).expect("Error in sending a reply");
                }
                Event::WordChecked {
                    user,
                    ref correct_positions,
                    ref contained_in_word,
                } => {
                    let mut current_game_status = get_game_status();
                    session.guess_count += 1;
                    if correct_positions.len() == 5 {
                        current_game_status.game_result = Some(GameResult::Win);
                        session_event = SessionEvent::GameStatus(current_game_status.clone());
                        session.session_status = SessionStatus::GameEnded {
                            result: GameResult::Win,
                        };
                    } else if session.guess_count >= 6 {
                        current_game_status.game_result = Some(GameResult::Lose);
                        session_event = SessionEvent::GameStatus(current_game_status.clone());
                        session.session_status = SessionStatus::GameEnded {
                            result: GameResult::Lose,
                        };
                    } else {
                        session_event = SessionEvent::WordChecked {
                            user: *user,
                            correct_positions: correct_positions.to_vec(),
                            contained_in_word: contained_in_word.to_vec(),
                        };
                    }
                    msg::reply(session_event.clone(), 0).expect("Error in sending a reply");
                }
            };
            if !matches!(session.session_status, SessionStatus::GameEnded { .. }) {
                session.session_status = SessionStatus::Waiting;
            }
        }
        SessionStatus::GameEnded { result: _ } => {
            msg::reply(SessionEvent::GameStatus(get_game_status()), 0).expect("Unable to reply");
        }
    };

    // Handle CheckGameStatus action (in any state)
    if let SessionAction::CheckGameStatus { user } = &action {
        debug!("===CHECK GAME STATUS (ANY STATE)===");
        check_game_status(session, *user);
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    let _reply_to = msg::reply_to().expect("Failed to query reply_to data");
    let session = unsafe { SESSION.as_mut().expect("The session is not initialized") };

    let event: Event = msg::load().expect("Unable to decode `Event`");

    if let Some((_, original_message_id)) = session.msg_ids {
        session.session_status = SessionStatus::MessageReceived(event);
        let _ = exec::wake(original_message_id);
    }
}

#[no_mangle]
extern "C" fn state() {
    let session = unsafe { SESSION.as_ref().expect("State is not existing") };
    msg::reply(session.clone(), 0).expect("Unable to get the state");
}

fn get_game_status() -> GameStatus {
    unsafe {
        SESSION
            .as_ref()
            .map(|s| s.game_status.clone())
            .expect("Game status is not initialized")
    }
}

fn check_game_status(session: &mut Session, user: ActorId) {
    if msg::source() != exec::program_id() {
        msg::reply(
            SessionEvent::GameError("Only the program can check game status".into()),
            0,
        )
        .expect("Unable to reply");
        return;
    }

    let current_block = exec::block_height() as u64;

    if current_block >= (session.start_block + 200).into() {
        session.session_status = SessionStatus::GameEnded {
            result: GameResult::Lose,
        };
        let current_game_status = get_game_status();
        msg::send(
            user,
            SessionEvent::GameStatus(current_game_status.clone()),
            0,
        )
        .expect("Unable to send message to player");
    } else {
        let current_game_status = get_game_status();
        msg::send(
            user,
            SessionEvent::GameStatus(current_game_status.clone()),
            0,
        )
        .expect("Unable to send message to player");
    }
}

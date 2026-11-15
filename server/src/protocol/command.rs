use crate::config::Direction;
use crate::protocol::response::Response;
use crate::state::player::EquipSlot;

#[derive(Debug)]
pub enum ChatScope {
    Global,
    Room,
    Group,
}

#[derive(Debug)]
pub enum GroupAction {
    Create,
    Invite { target: String },
    Join { leader: String },
    Leave,
}

#[derive(Debug)]
pub enum QuestAction {
    List,
    Summary,
    Request { npc: String },
    Status,
    Accept { id: String },
    Complete { id: String },
}

#[derive(Debug)]
pub enum Command {
    Connect { name: String, class: Option<String> },
    Who,
    Status,
    Look,
    Chat { scope: ChatScope, text: String },
    Take { item: String },
    Drop { item: String },
    Equip { slot: EquipSlot, item: String },
    Unequip { slot: EquipSlot },
    Inventory,
    Group(GroupAction),
    Move { direction: Direction },
    Attack { target: String },
    Talk { target: String },
    Quest(QuestAction),
    Quit,
    Unknown(String),
}

impl Command {
    /// Parses a raw command line from the client into a `Command` enum.
    /// Returns a `Response::Error` if the parsing fails or if required arguments are missing.
    pub fn parse(line: &str) -> Result<Self, Response> {
        let mut parts = line.splitn(2, ' ');
        let verb = parts.next().unwrap_or("").to_uppercase();
        let rest = parts.next().unwrap_or("").trim();

        match verb.as_str() {
            "CONNECT" => parse_connect(rest),

            "WHO" => Ok(Command::Who),
            "STATUS" => Ok(Command::Status),
            "LOOK" => Ok(Command::Look),
            "INVENTORY" => Ok(Command::Inventory),
            "QUIT" => Ok(Command::Quit),

            "TAKE" => require(rest, "TAKE requires an item").map(|item| Command::Take { item }),

            "DROP" => require(rest, "DROP requires an item").map(|item| Command::Drop { item }),

            "EQUIP" => parse_equip(rest),
            "UNEQUIP" => parse_unequip(rest),

            "CHAT" => parse_chat(rest),
            "GROUP" => parse_group(rest),

            "MOVE" | "GO" => match Direction::from_input(rest) {
                Some(direction) => Ok(Command::Move { direction }),
                None => Err(Response::error(
                    400,
                    "MOVE requires a direction (north/south/east/west)",
                )),
            },

            "ATTACK" => {
                require(rest, "ATTACK requires a target").map(|target| Command::Attack { target })
            }

            "TALK" => {
                require(rest, "TALK requires a target").map(|target| Command::Talk { target })
            }

            "QUEST" => parse_quest(rest),
            "QUESTS" => Ok(Command::Quest(QuestAction::Summary)),

            other => Ok(Command::Unknown(other.to_string())),
        }
    }
}

/// Parses the `CONNECT` command and its scope.
fn parse_connect(rest: &str) -> Result<Command, Response> {
    let mut p = rest.splitn(2, ' ');
    let name = p.next().unwrap_or("").trim().to_string();
    if name.is_empty() {
        return Err(Response::error(400, "CONNECT requires a name"));
    }
    let class = p
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(Command::Connect { name, class })
}

/// Helper function to ensure that a command argument is present.
fn require(rest: &str, msg: &'static str) -> Result<String, Response> {
    if rest.is_empty() {
        Err(Response::error(400, msg))
    } else {
        Ok(rest.to_string())
    }
}

/// Parses the `CHAT` command and its scope.
fn parse_chat(rest: &str) -> Result<Command, Response> {
    let mut p = rest.splitn(2, ' ');
    let scope_raw = p.next().unwrap_or("").to_uppercase();
    let msg = p.next().unwrap_or("").trim();

    let scope = match scope_raw.as_str() {
        "GLOBAL" => ChatScope::Global,
        "ROOM" => ChatScope::Room,
        "GROUP" => ChatScope::Group,
        "" => return Err(Response::error(400, "CHAT requires a scope and a message")),
        other => {
            return Err(Response::error(
                400,
                format!("Unknown chat scope: {}", other),
            ))
        }
    };

    if msg.is_empty() {
        Err(Response::error(400, "CHAT requires a message"))
    } else {
        Ok(Command::Chat {
            scope,
            text: msg.to_string(),
        })
    }
}

/// Parses the `GROUP` command and its subcommands.
fn parse_group(rest: &str) -> Result<Command, Response> {
    let mut p = rest.splitn(2, ' ');
    let sub = p.next().unwrap_or("").to_uppercase();
    let arg = p.next().unwrap_or("").trim();

    let action = match sub.as_str() {
        "CREATE" => GroupAction::Create,
        "LEAVE" => GroupAction::Leave,
        "INVITE" => GroupAction::Invite {
            target: require(arg, "GROUP INVITE requires a username")?,
        },
        "JOIN" => GroupAction::Join {
            leader: require(arg, "GROUP JOIN requires a leader name")?,
        },
        "" => return Err(Response::error(400, "GROUP requires a subcommand")),
        other => {
            return Err(Response::error(
                400,
                format!("Unknown GROUP subcommand: {}", other),
            ))
        }
    };
    Ok(Command::Group(action))
}

/// Parses the `QUEST` command and its subcommands.
fn parse_quest(rest: &str) -> Result<Command, Response> {
    let mut p = rest.splitn(2, ' ');
    let sub = p.next().unwrap_or("").to_uppercase();
    let arg = p.next().unwrap_or("").trim();

    let action = match sub.as_str() {
        "LIST" => QuestAction::List,
        "SUMMARY" => QuestAction::Summary,
        "STATUS" => QuestAction::Status,
        "ACCEPT" => QuestAction::Accept {
            id: require(arg, "QUEST ACCEPT requires a quest id")?,
        },
        "COMPLETE" => QuestAction::Complete {
            id: require(arg, "QUEST COMPLETE requires a quest id")?,
        },
        "" => return Err(Response::error(400, "QUEST requires an npc name")),
        _ => QuestAction::Request {
            npc: rest.to_string(),
        },
    };
    Ok(Command::Quest(action))
}

fn parse_equip(rest: &str) -> Result<Command, Response> {
    let mut p = rest.splitn(2, ' ');
    let slot_raw = p.next().unwrap_or("").trim();
    let item = p.next().unwrap_or("").trim();
    if slot_raw.is_empty() || item.is_empty() {
        return Err(Response::error(400, "EQUIP requires a slot and an item"));
    }
    let slot = parse_slot(slot_raw)?;
    Ok(Command::Equip {
        slot,
        item: item.to_string(),
    })
}

fn parse_unequip(rest: &str) -> Result<Command, Response> {
    let slot_raw = rest.trim();
    if slot_raw.is_empty() {
        return Err(Response::error(400, "UNEQUIP requires a slot"));
    }
    let slot = parse_slot(slot_raw)?;
    Ok(Command::Unequip { slot })
}

fn parse_slot(raw: &str) -> Result<EquipSlot, Response> {
    match raw.to_uppercase().as_str() {
        "RIGHT" => Ok(EquipSlot::Right),
        "LEFT" => Ok(EquipSlot::Left),
        _ => Err(Response::error(400, "Unknown slot (RIGHT/LEFT)")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(line: &str) -> Command {
        Command::parse(line).unwrap_or_else(|e| panic!("{line:?} failed: {e:?}"))
    }

    fn error_code(line: &str) -> u16 {
        match Command::parse(line) {
            Err(Response::Error { code, .. }) => code,
            other => panic!("{line:?} should fail, got {other:?}"),
        }
    }

    #[test]
    fn verbs_are_case_insensitive() {
        assert!(matches!(parse_ok("look"), Command::Look));
        assert!(matches!(parse_ok("LoOk"), Command::Look));
        assert!(matches!(parse_ok("WHO"), Command::Who));
        assert!(matches!(parse_ok("quit"), Command::Quit));
    }

    #[test]
    fn connect_with_and_without_class() {
        match parse_ok("CONNECT neo warrior") {
            Command::Connect { name, class } => {
                assert_eq!(name, "neo");
                assert_eq!(class.as_deref(), Some("warrior"));
            }
            other => panic!("unexpected {other:?}"),
        }
        match parse_ok("CONNECT trinity") {
            Command::Connect { name, class } => {
                assert_eq!(name, "trinity");
                assert!(class.is_none());
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(error_code("CONNECT"), 400);
    }

    #[test]
    fn move_accepts_full_and_short_directions() {
        assert!(matches!(
            parse_ok("MOVE north"),
            Command::Move {
                direction: Direction::North
            }
        ));
        assert!(matches!(
            parse_ok("GO w"),
            Command::Move {
                direction: Direction::West
            }
        ));
        assert_eq!(error_code("MOVE up"), 400);
        assert_eq!(error_code("MOVE"), 400);
    }

    #[test]
    fn chat_keeps_the_whole_message() {
        match parse_ok("CHAT room hello there, friend") {
            Command::Chat {
                scope: ChatScope::Room,
                text,
            } => assert_eq!(text, "hello there, friend"),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(error_code("CHAT"), 400);
        assert_eq!(error_code("CHAT global"), 400);
        assert_eq!(error_code("CHAT whisper hi"), 400);
    }

    #[test]
    fn item_commands_require_an_argument() {
        match parse_ok("TAKE rusty sword") {
            Command::Take { item } => assert_eq!(item, "rusty sword"),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(error_code("TAKE"), 400);
        assert_eq!(error_code("DROP"), 400);
        assert_eq!(error_code("ATTACK"), 400);
        assert_eq!(error_code("TALK"), 400);
    }

    #[test]
    fn equip_parses_slot_and_item() {
        match parse_ok("EQUIP right iron sword") {
            Command::Equip {
                slot: EquipSlot::Right,
                item,
            } => assert_eq!(item, "iron sword"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            parse_ok("UNEQUIP LEFT"),
            Command::Unequip {
                slot: EquipSlot::Left
            }
        ));
        assert_eq!(error_code("EQUIP head helmet"), 400);
        assert_eq!(error_code("EQUIP right"), 400);
        assert_eq!(error_code("UNEQUIP"), 400);
    }

    #[test]
    fn group_subcommands() {
        assert!(matches!(
            parse_ok("GROUP create"),
            Command::Group(GroupAction::Create)
        ));
        match parse_ok("GROUP INVITE morpheus") {
            Command::Group(GroupAction::Invite { target }) => assert_eq!(target, "morpheus"),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(error_code("GROUP INVITE"), 400);
        assert_eq!(error_code("GROUP"), 400);
        assert_eq!(error_code("GROUP dance"), 400);
    }

    #[test]
    fn quest_subcommands_and_npc_requests() {
        assert!(matches!(
            parse_ok("QUESTS"),
            Command::Quest(QuestAction::Summary)
        ));
        match parse_ok("QUEST ACCEPT q1") {
            Command::Quest(QuestAction::Accept { id }) => assert_eq!(id, "q1"),
            other => panic!("unexpected {other:?}"),
        }
        match parse_ok("QUEST old hermit") {
            Command::Quest(QuestAction::Request { npc }) => assert_eq!(npc, "old hermit"),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(error_code("QUEST"), 400);
        assert_eq!(error_code("QUEST COMPLETE"), 400);
    }

    #[test]
    fn unknown_verbs_are_reported() {
        match parse_ok("DANCE wildly") {
            Command::Unknown(verb) => assert_eq!(verb, "DANCE"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn responses_are_single_json_lines() {
        let line = Response::error(400, "bad").to_line();
        assert!(line.ends_with('\n'));
        assert_eq!(line.matches('\n').count(), 1);
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["code"], 400);
    }
}

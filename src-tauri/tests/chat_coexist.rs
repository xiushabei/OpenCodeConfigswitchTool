mod chat_support;

use chat_support::{
    create_v1, insert_session, insert_v1_message, insert_v1_part, insert_v2_message, Fixture,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use switcher_core::chat::{list_sessions, read_session, Sort};

fn encode(value: &Value) -> String {
    serde_json::to_string(value).expect("serialize fixture JSON")
}

/// Adds the v2 message table next to the v1 tables without `session_v2`/`session_part`.
/// This mirrors the observed live layout: real dialogue in v1, only a few v2 control events.
fn add_v2_messages(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE session_message (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL,
           type TEXT NOT NULL,
           seq INTEGER NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL,
           FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE CASCADE
         );
         CREATE INDEX session_message_session_seq_idx ON session_message(session_id, seq);",
    )
    .expect("create coexist session_message table");
}

/// Expected logical sizes: the v1 dialogue bytes for `root` and `switched`, and the v2 message
/// bytes for `v2-only`. Control events on `switched` must never contribute.
fn seed(fixture: &Fixture) -> (u64, u64, u64) {
    let conn = create_v1(fixture);
    add_v2_messages(&conn);
    insert_session(&conn, "root", "根会话", None, 100, 300);
    insert_session(&conn, "switched", "已切换代理", None, 110, 400);
    insert_session(&conn, "v2-only", "仅 v2 会话", None, 120, 500);
    insert_session(&conn, "empty", "空会话", None, 130, 600);

    let user = encode(&json!({"role": "user"}));
    let assistant = encode(&json!({"role": "assistant"}));
    let root_text = encode(&json!({"type": "text", "text": "root hello"}));
    let root_answer = encode(&json!({"type": "text", "text": "root answer"}));
    insert_v1_message(&conn, "m1", "root", 10, &user);
    insert_v1_message(&conn, "m2", "root", 20, &assistant);
    insert_v1_part(&conn, "p1", "m1", "root", 11, &root_text);
    insert_v1_part(&conn, "p2", "m2", "root", 21, &root_answer);

    let before = encode(&json!({"type": "text", "text": "before switch"}));
    let after = encode(&json!({"type": "text", "text": "after switch"}));
    insert_v1_message(&conn, "m3", "switched", 30, &user);
    insert_v1_message(&conn, "m4", "switched", 40, &assistant);
    insert_v1_part(&conn, "p3", "m3", "switched", 31, &before);
    insert_v1_part(&conn, "p4", "m4", "switched", 41, &after);

    // Five control events on a session that already has v1 dialogue.
    for seq in 1..=5 {
        let event = encode(&json!({"type": "agent-switched", "agent": format!("agent-{seq}")}));
        insert_v2_message(
            &conn,
            &format!("e{seq}"),
            "switched",
            "agent-switched",
            seq,
            &event,
        );
    }
    // Two genuine v2 messages for a session that only exists in the new generation.
    let v2_user = encode(&json!({"text": "v2 hello"}));
    let v2_answer = encode(&json!({"content": [{"type": "text", "text": "v2 answer"}]}));
    insert_v2_message(&conn, "vm1", "v2-only", "user", 1, &v2_user);
    insert_v2_message(&conn, "vm2", "v2-only", "assistant", 2, &v2_answer);

    let root = u64::try_from(user.len() + assistant.len() + root_text.len() + root_answer.len())
        .expect("root size");
    let switched = u64::try_from(user.len() + assistant.len() + before.len() + after.len())
        .expect("switch size");
    let v2_only = u64::try_from(v2_user.len() + v2_answer.len()).expect("v2 size");
    (root, switched, v2_only)
}

#[test]
fn coexist_list_reports_the_source_that_holds_dialogue() {
    let fixture = Fixture::new("coexist-list");
    let (root_size, switched_size, v2_size) = seed(&fixture);

    let list = list_sessions(fixture.database(), Sort::Created, false).expect("list coexist");
    assert_eq!(list.schema, "v1");
    assert_eq!(list.sessions.len(), 4);
    let row = |id: &str| {
        list.sessions
            .iter()
            .find(|session| session.id == id)
            .expect("listed session")
    };

    let root = row("root");
    assert_eq!(root.message_count, 2);
    assert_eq!(root.size_bytes, root_size);

    let switched = row("switched");
    assert_eq!(switched.message_count, 2);
    assert_eq!(switched.size_bytes, switched_size);

    let v2_only = row("v2-only");
    assert_eq!(v2_only.message_count, 2);
    assert_eq!(v2_only.size_bytes, v2_size);

    let empty = row("empty");
    assert_eq!(empty.message_count, 0);
    assert_eq!(empty.size_bytes, 0);
}

#[test]
fn coexist_read_keeps_v1_dialogue_and_v2_fallback() {
    let fixture = Fixture::new("coexist-read");
    seed(&fixture);

    let read = read_session(fixture.database(), "switched").expect("read coexist v1 session");
    let ids: Vec<&str> = read
        .messages
        .iter()
        .map(|message| message.id.as_str())
        .collect();
    assert_eq!(ids, ["m3", "m4"]);
    assert_eq!(read.messages[0].role, "user");
    assert_eq!(read.messages[0].parts[0].text, "before switch");
    assert_eq!(read.messages[1].role, "assistant");
    assert_eq!(read.messages[1].parts[0].text, "after switch");
    assert!(
        read.messages
            .iter()
            .flat_map(|message| &message.parts)
            .all(|part| !part.text.contains("agent-switched")),
        "control events must not replace real dialogue"
    );

    let read_v2 = read_session(fixture.database(), "v2-only").expect("read v2-only session");
    let v2_ids: Vec<&str> = read_v2
        .messages
        .iter()
        .map(|message| message.id.as_str())
        .collect();
    assert_eq!(v2_ids, ["vm1", "vm2"]);
    assert_eq!(read_v2.messages[0].role, "user");
    assert_eq!(read_v2.messages[0].parts[0].text, "v2 hello");
    assert_eq!(read_v2.messages[1].role, "assistant");
    assert_eq!(read_v2.messages[1].parts[0].text, "v2 answer");

    let read_empty = read_session(fixture.database(), "empty").expect("read empty session");
    assert_eq!(read_empty.messages, Vec::new());
    assert!(!read_empty.truncated);
}

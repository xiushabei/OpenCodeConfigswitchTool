mod chat_support;

use chat_support::{
    create_v1, create_v2, insert_session, insert_v1_message, insert_v1_part, insert_v2_message,
    insert_v2_part, Fixture,
};
use serde_json::{json, Value};
use switcher_core::chat::{list_sessions, read_session, Sort};

fn encode(value: &Value) -> String {
    serde_json::to_string(value).expect("serialize fixture JSON")
}

fn seed_v1(fixture: &Fixture) -> usize {
    let conn = create_v1(fixture);
    insert_session(&conn, "root", "根会话", None, 100, 300);
    insert_session(&conn, "child", "子会话", Some("root"), 110, 400);
    insert_session(&conn, "grand", "孙会话", Some("child"), 120, 500);
    insert_session(&conn, "other", "其他", None, 90, 200);

    let user = encode(&json!({"role": "user"}));
    let assistant = encode(&json!({"role": "assistant"}));
    insert_v1_message(&conn, "m1", "root", 10, &user);
    insert_v1_message(&conn, "m2", "root", 20, &assistant);
    let text = encode(&json!({"type": "text", "text": "hello"}));
    let big = encode(&json!({"type": "text", "text": "x".repeat(20_000)}));
    let base64 = "A".repeat(2_000);
    let tool = encode(&json!({
        "type": "tool",
        "tool": "read",
        "state": {
            "status": "completed",
            "input": {"path": "a.txt"},
            "output": format!("data:image/png;base64,{base64}")
        }
    }));
    insert_v1_part(&conn, "p1", "m1", "root", 11, &text);
    insert_v1_part(&conn, "p2", "m2", "root", 21, &tool);
    insert_v1_part(&conn, "p3", "m2", "root", 22, &big);
    insert_v1_message(&conn, "m3", "child", 30, &user);
    insert_v1_message(&conn, "m4", "other", 40, &user);

    user.len() + assistant.len() + text.len() + tool.len() + big.len()
}

#[test]
fn v1_list_sorts_by_size_and_reports_parent_and_counts() {
    let fixture = Fixture::new("v1-list-size");
    let expected_size = seed_v1(&fixture);
    let list = list_sessions(fixture.database(), Sort::Size, true).expect("list v1");

    assert_eq!(list.schema, "v1");
    assert_eq!(
        list.sessions.first().map(|row| row.id.as_str()),
        Some("root")
    );
    assert_eq!(
        list.sessions.last().map(|row| row.id.as_str()),
        Some("grand")
    );
    let root = list
        .sessions
        .iter()
        .find(|row| row.id == "root")
        .expect("root session");
    assert_eq!(root.parent_id, None);
    assert_eq!(root.message_count, 2);
    assert_eq!(root.size_bytes, u64::try_from(expected_size).expect("size"));
    let child = list
        .sessions
        .iter()
        .find(|row| row.id == "child")
        .expect("child session");
    assert_eq!(child.parent_id.as_deref(), Some("root"));
}

#[test]
fn v1_list_sorts_by_created_ascending() {
    let fixture = Fixture::new("v1-list-created");
    seed_v1(&fixture);
    let list = list_sessions(fixture.database(), Sort::Created, false).expect("list v1");
    let ids: Vec<&str> = list.sessions.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["other", "root", "child", "grand"]);
}

#[test]
fn v1_read_orders_messages_and_returns_full_text_and_tools() {
    let fixture = Fixture::new("v1-read");
    seed_v1(&fixture);
    let read = read_session(fixture.database(), "root").expect("read v1");

    assert_eq!(read.title, "根会话");
    let ids: Vec<&str> = read.messages.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["m1", "m2"]);
    assert_eq!(read.messages[0].role, "user");
    assert_eq!(read.messages[0].parts[0].text, "hello");
    let tool = &read.messages[1].parts[0];
    assert_eq!(tool.kind, "tool");
    assert!(tool.text.contains("data:image/png;base64,"));
    assert!(tool.text.contains(&"A".repeat(2_000)));
    let big = &read.messages[1].parts[1];
    assert_eq!(big.kind, "text");
    assert!(!big.truncated);
    assert_eq!(big.text, "x".repeat(20_000));
    assert!(!read.truncated);
}

#[test]
fn v1_read_returns_every_message_past_300() {
    let fixture = Fixture::new("v1-page");
    let conn = create_v1(&fixture);
    insert_session(&conn, "many", "很多消息", None, 1, 1);
    let user = encode(&json!({"role": "user"}));
    for index in 0..305 {
        insert_v1_message(&conn, &format!("m{index}"), "many", index, &user);
    }
    drop(conn);

    let read = read_session(fixture.database(), "many").expect("read every message");
    assert_eq!(read.messages.len(), 305);
    assert_eq!(read.messages.last().unwrap().id, "m304");
    assert!(!read.truncated);
}

#[test]
fn v2_list_and_read_use_seq_then_part_ordinal() {
    let fixture = Fixture::new("v2-read");
    let conn = create_v2(&fixture);
    insert_session(&conn, "root", "v2 root", None, 100, 300);
    insert_session(&conn, "other", "v2 other", None, 90, 200);
    let user = encode(&json!({"text": "hello v2"}));
    let assistant = encode(&json!({"content": [{"type": "text", "text": "inline"}]}));
    insert_v2_message(&conn, "m-user", "root", "user", 2, &user);
    insert_v2_message(&conn, "m-assistant", "root", "assistant", 1, &assistant);
    let first = encode(&json!({"type": "text", "text": "first"}));
    let second = encode(&json!({"type": "text", "text": "second"}));
    insert_v2_part(&conn, "m-assistant", "root", 2, "text", &second);
    insert_v2_part(&conn, "m-assistant", "root", 1, "text", &first);
    insert_v2_message(&conn, "m-other", "other", "user", 1, &user);
    drop(conn);

    let list = list_sessions(fixture.database(), Sort::Size, true).expect("list v2");
    assert_eq!(list.schema, "v2");
    let root = list
        .sessions
        .iter()
        .find(|row| row.id == "root")
        .expect("root session");
    assert_eq!(root.message_count, 2);
    let expected = user.len() + assistant.len() + first.len() + second.len();
    assert_eq!(root.size_bytes, u64::try_from(expected).expect("size"));

    let read = read_session(fixture.database(), "root").expect("read v2");
    let ids: Vec<&str> = read.messages.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["m-assistant", "m-user"]);
    assert_eq!(read.messages[0].role, "assistant");
    let parts: Vec<&str> = read.messages[0]
        .parts
        .iter()
        .map(|part| part.text.as_str())
        .collect();
    assert_eq!(parts, ["first", "second", "inline"]);
    assert_eq!(read.messages[1].parts[0].text, "hello v2");
}

#[test]
fn v1_read_returns_a_large_session_completely() {
    let fixture = Fixture::new("v1-large");
    let conn = create_v1(&fixture);
    insert_session(&conn, "large", "大会话", None, 1, 1);
    let user = encode(&json!({"role": "user"}));
    let body = "y".repeat(8 * 1024);
    let part = encode(&json!({"type": "text", "text": body}));
    for index in 0..305 {
        let id = format!("m{index}");
        insert_v1_message(&conn, &id, "large", index, &user);
        insert_v1_part(&conn, &format!("p{index}"), &id, "large", index, &part);
    }
    drop(conn);

    let read = read_session(fixture.database(), "large").expect("read large session");
    assert_eq!(read.messages.len(), 305);
    assert!(!read.truncated);
    assert!(read
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .all(|part| !part.truncated && part.text == body));
    let total: usize = read
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .map(|part| part.text.len())
        .sum();
    assert_eq!(total, 305 * 8 * 1024);
    assert!(total > 1024 * 1024);
}

#[test]
fn v2_without_session_v2_table_reads_inline_content() {
    let fixture = Fixture::new("v2-legacy");
    let conn = fixture.connection();
    conn.execute_batch(
        "CREATE TABLE session (
           id TEXT PRIMARY KEY,
           title TEXT NOT NULL,
           directory TEXT NOT NULL,
           parent_id TEXT,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL
         );
         CREATE TABLE session_message (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL,
           type TEXT NOT NULL,
           seq INTEGER NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL,
           FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE CASCADE
         );",
    )
    .expect("create v2 legacy schema");
    insert_session(&conn, "root", "legacy", None, 1, 2);
    insert_v2_message(
        &conn,
        "m1",
        "root",
        "assistant",
        1,
        &encode(&json!({"content": [{"type": "text", "text": "legacy v2"}]})),
    );
    drop(conn);

    let list = list_sessions(fixture.database(), Sort::Updated, true).expect("list v2 legacy");
    assert_eq!(list.schema, "v2");
    let read = read_session(fixture.database(), "root").expect("read v2 legacy");
    assert_eq!(read.messages[0].parts[0].text, "legacy v2");
}

#[test]
fn malformed_schema_fails_closed_for_list_and_read() {
    let fixture = Fixture::new("malformed");
    let conn = fixture.connection();
    conn.execute_batch(
        "CREATE TABLE session (
           id TEXT PRIMARY KEY,
           title TEXT NOT NULL,
           directory TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL
         );",
    )
    .expect("create malformed schema");
    drop(conn);

    assert!(list_sessions(fixture.database(), Sort::Updated, true).is_err());
    assert!(read_session(fixture.database(), "root").is_err());
}

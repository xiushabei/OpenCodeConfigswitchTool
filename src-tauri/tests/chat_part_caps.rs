mod chat_support;

use chat_support::{
    create_v1, create_v2, insert_session, insert_v1_message, insert_v1_part, insert_v2_message,
    insert_v2_part, Fixture,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use switcher_core::chat::read_session;

fn encode(value: &Value) -> String {
    serde_json::to_string(value).expect("serialize fixture JSON")
}

fn insert_parts_session(conn: &Connection, v2: bool, session: &str, count: usize) {
    insert_session(conn, session, "parts", None, 1, 1);
    let text = encode(&json!({"type": "text", "text": "x"}));
    let message = format!("m-{session}");
    if v2 {
        insert_v2_message(conn, &message, session, "assistant", 1, &encode(&json!({})));
        for index in 0..count {
            let ordinal = i64::try_from(index).unwrap_or(i64::MAX);
            insert_v2_part(conn, &message, session, ordinal, "text", &text);
        }
        return;
    }
    insert_v1_message(
        conn,
        &message,
        session,
        1,
        &encode(&json!({"role": "assistant"})),
    );
    for index in 0..count {
        let created = i64::try_from(index).unwrap_or(i64::MAX);
        insert_v1_part(
            conn,
            &format!("p-{session}-{index}"),
            &message,
            session,
            created,
            &text,
        );
    }
}

#[test]
fn every_part_row_is_returned_past_32() {
    for v2 in [false, true] {
        let fixture = Fixture::new(if v2 { "part-cap-v2" } else { "part-cap-v1" });
        let conn = if v2 {
            create_v2(&fixture)
        } else {
            create_v1(&fixture)
        };
        insert_parts_session(&conn, v2, "exact", 32);
        insert_parts_session(&conn, v2, "overflow", 33);
        drop(conn);

        let exact = read_session(fixture.database(), "exact").expect("read 32 parts");
        assert_eq!(exact.messages[0].parts.len(), 32);
        assert!(!exact.truncated);

        let overflow = read_session(fixture.database(), "overflow").expect("read 33 parts");
        assert_eq!(overflow.messages[0].parts.len(), 33);
        assert_eq!(overflow.messages[0].parts.last().unwrap().text, "x");
        assert!(!overflow.truncated);
    }
}

mod chat_support;

use chat_support::{
    create_v1, create_v2, insert_session, insert_v1_message, insert_v1_part, insert_v2_message,
    Fixture,
};
use serde_json::{json, Value};
use switcher_core::chat::read_session;

fn encode(value: &Value) -> String {
    serde_json::to_string(value).expect("serialize fixture JSON")
}

/// A v2 message whose inline JSON is far past the old 32 KiB SQL prefix must arrive complete.
#[test]
fn v2_large_inline_json_is_returned_completely() {
    let fixture = Fixture::new("lossless-v2-inline");
    let conn = create_v2(&fixture);
    insert_session(&conn, "inline", "v2 inline", None, 1, 2);

    let body = format!("{}{}", "内联内容".repeat(10_000), "INLINE-SUFFIX");
    let payload = encode(&json!({
        "content": [{"type": "text", "text": body, "ordinal": 0}]
    }));
    insert_v2_message(&conn, "m1", "inline", "assistant", 1, &payload);
    drop(conn);

    let read = read_session(fixture.database(), "inline").expect("read v2 inline");
    assert!(!read.truncated, "a valid v2 read must not be truncated");
    assert_eq!(read.messages.len(), 1);
    let text = &read.messages[0].parts[0].text;
    assert_eq!(text.len(), body.len());
    assert!(text.ends_with("INLINE-SUFFIX"), "final bytes must survive");
    assert!(!read.messages[0].parts[0].truncated);
}

/// Unicode prose longer than 32 KiB must keep every code point and its tail.
#[test]
fn v1_unicode_prose_over_32_kib_keeps_its_suffix() {
    let fixture = Fixture::new("lossless-v1-unicode");
    let conn = create_v1(&fixture);
    insert_session(&conn, "unicode", "unicode", None, 1, 2);
    insert_v1_message(
        &conn,
        "m1",
        "unicode",
        1,
        &encode(&json!({"role": "assistant"})),
    );

    let body = format!("{}尾部后缀✅", "长".repeat(12_000));
    assert!(body.len() > 32 * 1024, "fixture must exceed the old cap");
    insert_v1_part(
        &conn,
        "p1",
        "m1",
        "unicode",
        1,
        &encode(&json!({"type": "text", "text": body})),
    );
    drop(conn);

    let read = read_session(fixture.database(), "unicode").expect("read unicode prose");
    let part = &read.messages[0].parts[0];
    assert_eq!(part.text, body);
    assert!(part.text.ends_with("尾部后缀✅"));
    assert!(!part.truncated);
    assert!(!read.truncated);
}

/// Every database part row must be returned, including the ones past the old 32-row cap.
#[test]
fn v1_all_part_rows_past_32_are_returned_with_the_final_text() {
    let fixture = Fixture::new("lossless-parts");
    let conn = create_v1(&fixture);
    insert_session(&conn, "parts", "parts", None, 1, 2);
    insert_v1_message(
        &conn,
        "m1",
        "parts",
        1,
        &encode(&json!({"role": "assistant"})),
    );
    for index in 0..40 {
        let text = if index == 39 { "LAST-PART" } else { "part" };
        insert_v1_part(
            &conn,
            &format!("p{index}"),
            "m1",
            "parts",
            index,
            &encode(&json!({"type": "text", "text": text})),
        );
    }
    drop(conn);

    let read = read_session(fixture.database(), "parts").expect("read all parts");
    assert_eq!(read.messages[0].parts.len(), 40);
    assert_eq!(read.messages[0].parts.last().unwrap().text, "LAST-PART");
    assert!(!read.truncated);
}

/// A session with more than the old 300-message page limit must read one-shot in full.
#[test]
fn v1_more_than_300_messages_are_returned() {
    let fixture = Fixture::new("lossless-messages");
    let conn = create_v1(&fixture);
    insert_session(&conn, "many", "many", None, 1, 2);
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

/// A session larger than the old one MiB response budget must still read completely.
#[test]
fn v1_session_over_one_mib_is_complete() {
    let fixture = Fixture::new("lossless-mib");
    let conn = create_v1(&fixture);
    insert_session(&conn, "big", "big", None, 1, 2);
    let user = encode(&json!({"role": "user"}));
    let body = "z".repeat(60_000);
    for index in 0..24 {
        let id = format!("m{index}");
        insert_v1_message(&conn, &id, "big", index, &user);
        insert_v1_part(
            &conn,
            &format!("p{index}"),
            &id,
            "big",
            index,
            &encode(&json!({"type": "text", "text": body})),
        );
    }
    drop(conn);

    let read = read_session(fixture.database(), "big").expect("read over one MiB");
    assert_eq!(read.messages.len(), 24);
    assert!(!read.truncated);
    let total: usize = read
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .map(|part| part.text.len())
        .sum();
    assert_eq!(total, 24 * 60_000);
    assert!(total > 1024 * 1024);
    assert!(read
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .all(|part| !part.truncated));
}

/// Tool and prose payloads keep long suffixes and base64-looking data verbatim.
#[test]
fn long_tool_and_base64_like_payloads_are_kept_verbatim() {
    let fixture = Fixture::new("lossless-tool");
    let conn = create_v1(&fixture);
    insert_session(&conn, "tool", "tool", None, 1, 2);
    insert_v1_message(
        &conn,
        "m1",
        "tool",
        1,
        &encode(&json!({"role": "assistant"})),
    );

    let encoded = "A".repeat(3_000);
    let output = format!("data:image/png;base64,{encoded}-TOOL-SUFFIX");
    let tool = encode(&json!({
        "type": "tool",
        "tool": "read",
        "state": {
            "status": "completed",
            "input": {"path": "C:/work/a.txt"},
            "output": output
        }
    }));
    insert_v1_part(&conn, "p1", "m1", "tool", 1, &tool);

    let prose = format!("{}==PROSE-SUFFIX", "QUJD".repeat(400));
    insert_v1_part(
        &conn,
        "p2",
        "m1",
        "tool",
        2,
        &encode(&json!({"type": "text", "text": prose})),
    );
    drop(conn);

    let read = read_session(fixture.database(), "tool").expect("read tool payloads");
    assert!(!read.truncated);
    let tool_text = &read.messages[0].parts[0].text;
    assert!(tool_text.contains("read"));
    assert!(tool_text.contains("completed"));
    assert!(tool_text.contains("C:/work/a.txt"));
    assert!(tool_text.contains(&encoded), "base64 must not be scrubbed");
    assert!(
        tool_text.ends_with("-TOOL-SUFFIX"),
        "tool suffix must survive"
    );
    assert!(!read.messages[0].parts[0].truncated);

    let prose_text = &read.messages[0].parts[1].text;
    assert_eq!(prose_text, &prose);
    assert!(prose_text.ends_with("==PROSE-SUFFIX"));
}

/// Malformed JSON stays readable and is explicitly flagged instead of aborting or vanishing.
#[test]
fn malformed_part_is_flagged_without_aborting_the_session() {
    let fixture = Fixture::new("lossless-malformed");
    let conn = create_v1(&fixture);
    insert_session(&conn, "bad", "bad", None, 1, 2);
    insert_v1_message(
        &conn,
        "m1",
        "bad",
        1,
        &encode(&json!({"role": "assistant"})),
    );
    insert_v1_part(&conn, "p1", "m1", "bad", 1, "{not json");
    drop(conn);

    let read = read_session(fixture.database(), "bad").expect("malformed part must not abort");
    assert_eq!(read.messages.len(), 1);
    assert!(read.messages[0].parts[0].truncated);
    assert!(read.truncated);
}

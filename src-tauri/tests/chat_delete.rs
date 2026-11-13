mod chat_support;

use chat_support::{
    count, create_v1, create_v2, has_table, insert_blob_ref, insert_inbox, insert_session,
    insert_v1_message, insert_v1_part, insert_v2_message, insert_v2_part, Fixture,
};
use serde_json::json;
use switcher_core::chat::{execute_delete, preview_delete};

fn encode(value: &serde_json::Value) -> String {
    serde_json::to_string(value).expect("serialize fixture JSON")
}

fn seed_tree(fixture: &Fixture) {
    let conn = create_v1(fixture);
    insert_session(&conn, "root", "根会话", None, 100, 300);
    insert_session(&conn, "child", "子会话", Some("root"), 110, 400);
    insert_session(&conn, "grand", "孙会话", Some("child"), 120, 500);
    insert_session(&conn, "other", "其他", None, 90, 200);
    let user = encode(&json!({"role": "user"}));
    for (message, session, created) in [
        ("m1", "root", 10_i64),
        ("m2", "child", 20),
        ("m3", "grand", 30),
        ("m4", "other", 40),
    ] {
        insert_v1_message(&conn, message, session, created, &user);
        insert_v1_part(
            &conn,
            &format!("p-{message}"),
            message,
            session,
            created,
            &encode(&json!({"type": "text", "text": format!("body {message}")})),
        );
    }
}

#[test]
fn preview_includes_descendants_and_sizes() {
    let fixture = Fixture::new("preview");
    seed_tree(&fixture);
    let preview = preview_delete(fixture.database(), "root").expect("preview");
    let mut ids = preview.session_ids.clone();
    ids.sort();
    assert_eq!(ids, ["child", "grand", "root"]);
    assert_eq!(preview.titles.len(), 3);
    assert!(preview.size_bytes > 0);
    assert_eq!(preview.blocked_reason, None);
}

#[test]
fn delete_removes_confirmed_subtree_and_backup_restores_it() {
    let fixture = Fixture::new("delete");
    seed_tree(&fixture);
    let preview = preview_delete(fixture.database(), "root").expect("preview");
    let mut confirmed = preview.session_ids.clone();
    confirmed.reverse();

    let report =
        execute_delete(fixture.database(), fixture.root(), "root", &confirmed).expect("delete");
    assert_eq!(report.deleted_session_ids.len(), 3);
    assert!(report.message.contains("WAL"));
    let backup = fixture.root().join(&report.backup);
    assert!(backup.is_file());

    let current = fixture.connection();
    assert_eq!(count(&current, "session"), 1);
    assert_eq!(count(&current, "message"), 1);
    assert_eq!(count(&current, "part"), 1);
    drop(current);

    let restored = rusqlite::Connection::open(&backup).expect("open backup");
    assert_eq!(count(&restored, "session"), 4);
    assert_eq!(count(&restored, "message"), 4);
    assert_eq!(count(&restored, "part"), 4);
}

#[test]
fn confirmation_mismatch_aborts_before_backup() {
    let fixture = Fixture::new("confirm");
    seed_tree(&fixture);
    let error = execute_delete(
        fixture.database(),
        fixture.root(),
        "root",
        &["root".to_string()],
    )
    .expect_err("mismatch must fail");
    assert!(error.to_string().contains("不一致"));
    assert!(!fixture.root().join("backups").exists());
    let current = fixture.connection();
    assert_eq!(count(&current, "session"), 4);
}

#[test]
fn descendant_cycle_guard_terminates() {
    let fixture = Fixture::new("cycle");
    let conn = create_v1(&fixture);
    insert_session(&conn, "root", "根", Some("child"), 1, 1);
    insert_session(&conn, "child", "子", Some("root"), 2, 2);
    drop(conn);
    let preview = preview_delete(fixture.database(), "root").expect("cycle preview");
    let mut ids = preview.session_ids.clone();
    ids.sort();
    assert_eq!(ids, ["child", "root"]);
}

#[test]
fn v2_delete_cleans_verified_optional_tables_only() {
    let fixture = Fixture::new("v2-delete");
    let conn = create_v2(&fixture);
    insert_session(&conn, "root", "v2 根", None, 1, 2);
    insert_session(&conn, "other", "v2 其他", None, 3, 4);
    let user = encode(&json!({"text": "hi"}));
    insert_v2_message(&conn, "m1", "root", "user", 1, &user);
    insert_v2_message(&conn, "m2", "other", "user", 1, &user);
    insert_v2_part(
        &conn,
        "m1",
        "root",
        0,
        "text",
        &encode(&json!({"type": "text", "text": "a"})),
    );
    insert_v2_part(
        &conn,
        "m2",
        "other",
        0,
        "text",
        &encode(&json!({"type": "text", "text": "b"})),
    );
    insert_blob_ref(&conn, "root", "m1", "h1");
    insert_blob_ref(&conn, "other", "m2", "h2");
    insert_inbox(&conn, "i1", "root");
    insert_inbox(&conn, "i2", "other");
    drop(conn);

    let preview = preview_delete(fixture.database(), "root").expect("v2 preview");
    execute_delete(
        fixture.database(),
        fixture.root(),
        "root",
        &preview.session_ids,
    )
    .expect("v2 delete");

    let current = fixture.connection();
    assert!(has_table(&current, "session_v2"));
    assert_eq!(count(&current, "session_v2"), 1);
    assert_eq!(count(&current, "session_message"), 1);
    assert_eq!(count(&current, "session_part"), 1);
    assert_eq!(count(&current, "blob_ref"), 1);
    assert_eq!(count(&current, "session_inbox"), 1);
}

#[test]
fn deletion_requires_cascade_foreign_keys() {
    let fixture = Fixture::new("no-fk");
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
         CREATE TABLE message (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL
         );
         CREATE TABLE part (
           id TEXT PRIMARY KEY,
           message_id TEXT NOT NULL,
           session_id TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           data TEXT NOT NULL
         );",
    )
    .expect("create schema without foreign keys");
    insert_session(&conn, "root", "无外键", None, 1, 1);
    insert_v1_message(&conn, "m1", "root", 1, &encode(&json!({"role": "user"})));
    drop(conn);

    assert!(switcher_core::chat::list_sessions(
        fixture.database(),
        switcher_core::chat::Sort::Updated,
        true
    )
    .is_ok());
    let error = execute_delete(
        fixture.database(),
        fixture.root(),
        "root",
        &["root".to_string()],
    )
    .expect_err("missing cascade must fail closed");
    assert!(error.to_string().contains("CASCADE"));
    assert!(!fixture.root().join("backups").exists());
    let current = fixture.connection();
    assert_eq!(count(&current, "session"), 1);
}

#[test]
fn malformed_schema_refuses_deletion_without_backup() {
    let fixture = Fixture::new("bad-delete");
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
         CREATE TABLE message (
           id TEXT PRIMARY KEY,
           session_id TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           time_updated INTEGER NOT NULL,
           data TEXT NOT NULL
         );
         CREATE TABLE part (
           id TEXT PRIMARY KEY,
           message_id TEXT NOT NULL,
           time_created INTEGER NOT NULL,
           data TEXT NOT NULL
         );",
    )
    .expect("create malformed schema");
    conn.execute(
        "INSERT INTO session (id, title, directory, time_created, time_updated) \
         VALUES ('root', '根', '/w', 1, 1)",
        [],
    )
    .expect("insert session");
    drop(conn);

    let error = execute_delete(
        fixture.database(),
        fixture.root(),
        "root",
        &["root".to_string()],
    )
    .expect_err("unknown schema must fail closed");
    assert!(error.to_string().contains("结构"));
    assert!(!fixture.root().join("backups").exists());
}

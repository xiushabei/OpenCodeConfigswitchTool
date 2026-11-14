mod chat_support;

use chat_support::{
    count, create_v1, create_v2, has_table, insert_blob_ref, insert_inbox, insert_session,
    insert_v1_message, insert_v1_part, insert_v2_message, insert_v2_part, Fixture,
};
use serde_json::json;
use std::fs;
use switcher_core::chat::{
    backup_database, execute_delete, execute_delete_with_options, preview_delete, DeleteOptions,
};

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
fn no_backup_delete_removes_confirmed_subtree_and_leaves_no_files() {
    let fixture = Fixture::new("delete-no-backup");
    seed_tree(&fixture);
    let preview = preview_delete(fixture.database(), "root").expect("preview");

    let report = execute_delete_with_options(
        fixture.database(),
        fixture.root(),
        "root",
        &preview.session_ids,
        DeleteOptions {
            backup_before_delete: false,
        },
    )
    .expect("delete without backup");

    let mut deleted = report.deleted_session_ids.clone();
    deleted.sort();
    assert_eq!(deleted, ["child", "grand", "root"]);
    assert_eq!(report.backup, "");
    assert!(report.message.contains("未创建备份"));
    assert!(
        !fixture.root().join("backups").exists(),
        "禁用备份时不得创建 backups 目录"
    );

    let current = fixture.connection();
    assert_eq!(count(&current, "session"), 1);
    assert_eq!(count(&current, "message"), 1);
    assert_eq!(count(&current, "part"), 1);
    let remaining_session: String = current
        .query_row("SELECT id FROM session", [], |row| row.get(0))
        .expect("remaining session");
    let remaining_message: String = current
        .query_row("SELECT id FROM message", [], |row| row.get(0))
        .expect("remaining message");
    let remaining_part: String = current
        .query_row("SELECT id FROM part", [], |row| row.get(0))
        .expect("remaining part");
    assert_eq!(remaining_session, "other");
    assert_eq!(remaining_message, "m4");
    assert_eq!(remaining_part, "p-m4");
}

#[test]
fn backup_failure_blocks_backup_delete_but_no_backup_delete_ignores_root() {
    let fixture = Fixture::new("delete-backup-blocked");
    seed_tree(&fixture);
    let blocked_root = fixture.root().join("not-a-directory");
    fs::write(&blocked_root, b"occupied").expect("create blocking file");
    let preview = preview_delete(fixture.database(), "root").expect("preview");

    let error = execute_delete_with_options(
        fixture.database(),
        &blocked_root,
        "root",
        &preview.session_ids,
        DeleteOptions {
            backup_before_delete: true,
        },
    )
    .expect_err("unwritable backup root must block the delete");
    assert!(error.to_string().contains("备份"));
    assert_eq!(count(&fixture.connection(), "session"), 4);

    let report = execute_delete_with_options(
        fixture.database(),
        &blocked_root,
        "root",
        &preview.session_ids,
        DeleteOptions {
            backup_before_delete: false,
        },
    )
    .expect("no-backup delete must not need the root");
    assert_eq!(report.backup, "");
    assert!(report.message.contains("未创建备份"));
    let current = fixture.connection();
    assert_eq!(count(&current, "session"), 1);
    assert_eq!(count(&current, "message"), 1);
    assert_eq!(count(&current, "part"), 1);
}

#[test]
fn backup_skips_free_pages_and_restores_every_row() {
    let fixture = Fixture::new("compact-backup");
    let conn = create_v1(&fixture);
    insert_session(&conn, "root", "根会话", None, 100, 300);
    insert_session(&conn, "child", "子会话", Some("root"), 110, 400);
    insert_session(&conn, "other", "其他", None, 90, 200);
    insert_session(&conn, "bulk", "历史", None, 1, 2);

    let user = encode(&json!({"role": "user"}));
    for (message, session, created) in [
        ("m1", "root", 10_i64),
        ("m2", "child", 20),
        ("m4", "other", 40),
    ] {
        insert_v1_message(&conn, message, session, created, &user);
        insert_v1_part(
            &conn,
            &format!("p-{message}"),
            message,
            session,
            created,
            &encode(&json!({"type": "text", "text": "body"})),
        );
    }

    let bulk = encode(&json!({"type": "text", "text": "x".repeat(32 * 1024)}));
    for index in 0..80 {
        let created = 1_000_i64 + index;
        let message = format!("m-bulk-{index}");
        insert_v1_message(&conn, &message, "bulk", created, &user);
        for part in 0..4 {
            insert_v1_part(
                &conn,
                &format!("p-{message}-{part}"),
                &message,
                "bulk",
                created,
                &bulk,
            );
        }
    }
    // Simulate an earlier confirmed delete: the bulk rows are gone but their pages stay free.
    conn.execute("DELETE FROM session WHERE id = 'bulk'", [])
        .expect("drop bulk session");
    drop(conn);

    let checkpoint = fixture.connection();
    checkpoint
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint");
    let pages: i64 = checkpoint
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .expect("page count");
    let free: i64 = checkpoint
        .query_row("PRAGMA freelist_count", [], |row| row.get(0))
        .expect("freelist count");
    assert!(
        free * 2 > pages,
        "fixture must be mostly free: {free}/{pages}"
    );
    drop(checkpoint);

    let database_bytes = std::fs::metadata(fixture.database())
        .expect("database metadata")
        .len();
    let preview = preview_delete(fixture.database(), "root").expect("preview");
    let report = execute_delete(
        fixture.database(),
        fixture.root(),
        "root",
        &preview.session_ids,
    )
    .expect("delete");

    let backup = fixture.root().join(&report.backup);
    let backup_bytes = std::fs::metadata(&backup).expect("backup metadata").len();
    assert!(
        backup_bytes * 2 < database_bytes,
        "backup must omit free pages: {backup_bytes} vs {database_bytes}"
    );

    let restored = rusqlite::Connection::open(&backup).expect("open backup");
    assert_eq!(count(&restored, "session"), 3);
    assert_eq!(count(&restored, "message"), 3);
    assert_eq!(count(&restored, "part"), 3);

    let current = fixture.connection();
    assert_eq!(count(&current, "session"), 1);
    assert_eq!(count(&current, "message"), 1);
    assert_eq!(count(&current, "part"), 1);
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

#[test]
fn concurrent_backups_are_distinct_and_complete() {
    let fixture = Fixture::new("concurrent-backup");
    seed_tree(&fixture);

    let mut handles = Vec::new();
    for _ in 0..8 {
        let database = fixture.database().to_path_buf();
        let root = fixture.root().to_path_buf();
        handles.push(std::thread::spawn(move || {
            backup_database(&database, &root)
        }));
    }
    let mut backups = Vec::new();
    for handle in handles {
        backups.push(handle.join().expect("join backup thread").expect("backup"));
    }
    backups.sort();
    backups.dedup();
    assert_eq!(backups.len(), 8, "并发备份必须使用不同文件名：{backups:?}");

    for backup in &backups {
        let conn = rusqlite::Connection::open(fixture.root().join(backup)).expect("open backup");
        assert_eq!(count(&conn, "session"), 4);
        assert_eq!(count(&conn, "message"), 4);
        assert_eq!(count(&conn, "part"), 4);
    }

    let partials = fs::read_dir(fixture.root().join("backups").join("chat"))
        .expect("backup directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "partial"))
        .count();
    assert_eq!(partials, 0, "成功的备份不应留下 partial 文件");
}

#[test]
fn backup_never_deletes_a_partial_it_did_not_create() {
    let fixture = Fixture::new("occupied-partial");
    seed_tree(&fixture);
    let directory = fixture.root().join("backups").join("chat");
    fs::create_dir_all(&directory).expect("backup directory");
    let occupied = directory.join("chat-1-2-3.db.partial");
    fs::write(&occupied, b"other writer").expect("seed partial");

    let backup = backup_database(fixture.database(), fixture.root()).expect("backup");
    assert!(fixture.root().join(backup).is_file());
    assert_eq!(fs::read(&occupied).expect("read partial"), b"other writer");
}

#[test]
fn failed_snapshot_publishes_no_final_file() {
    let fixture = Fixture::new("failed-backup");
    let source = fixture.root().join("broken.db");
    fs::write(&source, b"not a sqlite database").expect("seed broken database");

    backup_database(&source, fixture.root()).expect_err("broken source must fail");

    let leftovers: Vec<_> = fs::read_dir(fixture.root().join("backups").join("chat"))
        .expect("backup directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    assert!(leftovers.is_empty(), "失败不应留下文件：{leftovers:?}");
}

use std::path::{Path, PathBuf};

use super::{partial_error, restore_startup, Action, RestorePlan};
use crate::error::AppError;
use crate::model::Target;
use crate::startup::{BaselineFile, BaselineState, StartupBaseline};

#[test]
fn unreadable_baseline_blocks_restore_before_writes() {
    let baseline = StartupBaseline {
        files: vec![
            BaselineFile {
                target: Target::Opencode,
                path: PathBuf::from("unused-opencode.json"),
                state: BaselineState::Unreadable("read failed".to_string()),
            },
            BaselineFile {
                target: Target::Omo,
                path: PathBuf::from("unused-omo.json"),
                state: BaselineState::Missing,
            },
        ],
    };

    let error = restore_startup(Path::new("."), &baseline).expect_err("restore blocked");
    assert!(
        matches!(error, AppError::StartupRestoreBlocked { .. }),
        "{error}"
    );
}

#[test]
fn partial_error_lists_changed_failed_unprocessed_and_all_backups() {
    let files = [
        BaselineFile {
            target: Target::Opencode,
            path: PathBuf::from("opencode.json"),
            state: BaselineState::Missing,
        },
        BaselineFile {
            target: Target::Omo,
            path: PathBuf::from("omo.json"),
            state: BaselineState::Missing,
        },
    ];
    let error = AppError::TargetPathInvalid {
        reason: "write failed".to_string(),
    };
    let mut plans = [
        RestorePlan {
            file: &files[0],
            current: Some(b"changed".to_vec()),
            backup: Some("backups/opencode-1.json".to_string()),
            action: Action::Restored,
        },
        RestorePlan {
            file: &files[1],
            current: Some(b"changed".to_vec()),
            backup: Some("backups/omo-1.json".to_string()),
            action: Action::Unchanged,
        },
    ];

    let AppError::RestorePartial { reason } = partial_error(&plans, 1, &error) else {
        panic!("expected RestorePartial");
    };
    assert!(reason.contains("已变更：OpenCode 已恢复"), "{reason}");
    assert!(reason.contains("失败：OMO"), "{reason}");
    assert!(reason.contains("write failed"), "{reason}");
    assert!(reason.contains("未处理：无"), "{reason}");
    assert!(
        reason.contains("backups/opencode-1.json、backups/omo-1.json"),
        "{reason}"
    );

    plans[0].action = Action::Unchanged;
    let AppError::RestorePartial { reason } = partial_error(&plans, 0, &error) else {
        panic!("expected RestorePartial");
    };
    assert!(reason.contains("已变更：无"), "{reason}");
    assert!(reason.contains("失败：OpenCode"), "{reason}");
    assert!(reason.contains("未处理：OMO"), "{reason}");
}
